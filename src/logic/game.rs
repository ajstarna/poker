use actix::Addr;
use json::object;
use rand::Rng;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;

use super::card::{Card, Deck, HandResult, StandardDeck};
use super::player::{Player, PlayerAction, PlayerConfig};
use crate::hub::GameHub;
use crate::messages::{GameOver, JoinGameError, MetaAction, Returned, ReturnedReason};

use std::{cmp, iter, sync::Arc, thread, time};

use uuid::Uuid;

#[derive(Debug, PartialEq, Copy, Clone)]
enum Street {
    Preflop,
    Flop,
    Turn,
    River,
    ShowDown,
}

/// A pot keeps track of the total money, and which player (indices) contributed
/// A game hand can have multiple pots, when players go all-in, and betting continues
#[derive(Debug)]
struct Pot {
    money: u32,                        // total amount in this pot
    contributions: HashMap<Uuid, u32>, // which players have contributed to the pot, and how much
    // the most that any one player can put in. If a player goes all-in into a pot,
    // then the cap is the amount that player has put in
    cap: Option<u32>,
}

impl Pot {
    fn new() -> Self {
        Self {
            money: 0,
            contributions: HashMap::new(),
            cap: None,
        }
    }
}

/// The pot manager keeps track of how many pots there are and which players
/// how contributed how much to each.
#[derive(Debug)]
struct PotManager {
    pots: Vec<Pot>,
}

impl PotManager {
    fn new() -> Self {
        // the pot manager starts with a single main pot
        Self {
            pots: vec![Pot::new()],
        }
    }

    /// returns a vec of each pot.money for the all pots
    /// useful to pass to the front end
    fn simple_repr(&self) -> Vec<u32> {
        self.pots.iter().map(|x| x.money).collect()
    }

    /// given a player id and an amount they need to contribute to the pot
    /// and whether this is putting them all-in), this method puts the proper
    /// amount into the proper pot(s), and possibly create and redistribute into a new side pot
    fn contribute(&mut self, player_id: Uuid, amount: u32, all_in: bool) {
        println!(
            "inside contribute: {:?}, {:?}, all_in={:?}",
            player_id, amount, all_in
        );
        let mut to_contribute = amount;
        // insert_pot keeps track of the index at which we want to insert a pot (index+1) to,
        // and the new cap for the pot at that index
        let mut insert_pot: Option<(usize, u32)> = None;
        for (i, pot) in self.pots.iter_mut().enumerate() {
            let so_far = pot.contributions.entry(player_id).or_insert(0);
            if let Some(cap) = pot.cap {
                println!("cap of {}", cap);
                if *so_far > cap {
                    panic!(
                        "somehow player {} put in more than the cap for \
				    the the pot at index {}",
                        player_id, i
                    );
                } else if *so_far == cap {
                    println!("we have already filled up this pot");
                    continue;
                }
                // else, we need to put more into the pot
                let remaining = cap - *so_far; // amount left before the cap
                if remaining >= to_contribute {
                    println!(
                        "the new contribution fits since {} > {}",
                        remaining, to_contribute
                    );
                    *so_far += to_contribute;
                    pot.money += to_contribute;
                    if all_in {
                        // our all-in is smaller than the previous all-in
                        println!("our all-in is smaller than the previous all-in");
                        //pot.cap = Some(pot.contributions[&player_id]);
                        insert_pot = Some((i, pot.contributions[&player_id]));
                    }
                    break;
                } else {
                    // we need to contribute to the cap, then put more in the next pot
                    println!("we need to contribute to the cap, then put more in the next pot");
                    *so_far += remaining;
                    pot.money += remaining;
                    assert!(*so_far == cap);
                    to_contribute -= remaining;
                    println!("still need to contribute {}", to_contribute)
                }
            } else {
                // there is not cap on this pot, so simply put the new money in for this player
                println!("no cap");
                *so_far += to_contribute;
                pot.money += to_contribute;
                if all_in {
                    //pot.cap = Some(pot.contributions[&player_id]);
                    insert_pot = Some((i, pot.contributions[&player_id]));
                }
                break;
            }
        }
        if let Some((index, new_cap)) = insert_pot {
            println!(
                "inserting a pot at index {} and capping the previous pot at {}",
                index + 1,
                new_cap
            );
            self.pots.insert(index + 1, Pot::new());
            self.transfer_excess(index, new_cap)
        }
    }

    /// give the index of a newly-capped pot, we move any excess contributions from the pot
    /// to the next one in the vecdeque. We also move the existing cap-differential into the pot at index+1
    /// and set the new_cap in the pot at index.
    /// This happens when a all-in happens and makes the pot at index newly-capped
    /// Note: if none of the contributions to the previous pot were higher than the new cap, then no money
    /// will need to be transfered into the new pot at index+1
    fn transfer_excess(&mut self, index: usize, new_cap: u32) {
        let prev_pot = self.pots.get_mut(index).unwrap();
        println!("prev_pot = {:?}", prev_pot);
        let mut transfers = HashMap::<Uuid, u32>::new();
        let prev_cap_opt = prev_pot.cap; // move the previous cap to the new pot (if needed)
        prev_pot.cap = Some(new_cap);
        for (id, amount) in prev_pot.contributions.iter_mut() {
            //let b: bool = id;
            if *amount > new_cap {
                // we need to move the excess above the cap of the pot to the new pot
                let excess = *amount - new_cap;
                transfers.insert(*id, excess);
                *amount = new_cap;
                prev_pot.money -= excess;
            }
        }
        println!("after taking = {:?}", prev_pot);
        println!("transfers = {:?}", transfers);
        let mut new_pot = self.pots.get_mut(index + 1).unwrap();
        new_pot.money = transfers.values().sum();
        new_pot.contributions = transfers;

        if let Some(prev_cap) = prev_cap_opt {
            // if the old pot had a cap already, then the new pot is capped at the difference.
            // e.g. if someone was all-in with 750, then someone calls to go all-in with 500,
            // the the prev_pot is NOW capped at 500, and the next pot is capped at 250
            new_pot.cap = Some(prev_cap - new_cap);
        }
    }
}

#[derive(Debug)]
struct GameHand {
    street: Street,
    pot_manager: PotManager,
    total_contributions: [u32; 9], // how much a player contributed to the pot during the whole hand
    flop: Option<Vec<Card>>,
    turn: Option<Card>,
    river: Option<Card>,
}

impl GameHand {
    fn default() -> Self {
        GameHand {
            street: Street::Preflop,
            pot_manager: PotManager::new(),
            total_contributions: [0; 9],
            flop: None,
            turn: None,
            river: None,
        }
    }
}

// any game that runs for too long without a human will end, rather than looping indefinitely
const NON_HUMAN_HANDS_LIMIT: u32 = 3;

#[derive(Debug)]
pub struct Game {
    hub_addr: Option<Addr<GameHub>>, // needs to be able to communicate back to the hub sometimes
    pub name: String,
    deck: Box<dyn Deck>,
    players: [Option<Player>; 9], // 9 spots where players can sit
    player_ids_to_configs: HashMap<Uuid, PlayerConfig>,
    max_players: u8, // how many will we let in the game
    small_blind: u32,
    big_blind: u32,
    buy_in: u32,
    password: Option<String>,
    button_idx: usize, // index of the player with the button
    hands_played: u32, // keeps track of how many hands were played
}

/// useful for unit tests, for example
impl Default for Game {
    fn default() -> Self {
        Self {
            hub_addr: None,
            name: "Game".to_owned(),
            deck: Box::new(StandardDeck::new()),
            players: Default::default(),
            player_ids_to_configs: HashMap::<Uuid, PlayerConfig>::new(),
            max_players: 9,
            small_blind: 4,
            big_blind: 8,
            buy_in: 1000,
            password: None,
            button_idx: 0,
            hands_played: 0,
        }
    }
}

impl Game {
    /// the address of the GameHub is optional so that unit tests need not worry about it
    /// We can pass in a custom Deck object, but if not, we will just construct a StandardDeck
    pub fn new(
        hub_addr: Addr<GameHub>,
        name: String,
        deck_opt: Option<Box<dyn Deck>>,
        max_players: u8, // how many will we let in the game
        small_blind: u32,
        big_blind: u32,
        buy_in: u32,
        password: Option<String>,
    ) -> Self {
        let deck = if deck_opt.is_some() {
            deck_opt.unwrap()
        } else {
            Box::new(StandardDeck::new())
        };
        Game {
            hub_addr: Some(hub_addr),
            name,
            deck,
            players: Default::default(),
            player_ids_to_configs: HashMap::<Uuid, PlayerConfig>::new(),
            max_players,
            small_blind,
            big_blind,
            buy_in,
            password,
            button_idx: 0,
            hands_played: 0,
        }
    }

    /// add a given playerconfig to an empty seat
    /// if the game requires a password, then a matching password must be provided for the user to be added
    /// TODO: eventually we wanmt the player to select an open seat I guess
    /// returns the index of the seat that they joined (if they were able to join)
    pub fn add_user(
        &mut self,
        player_config: PlayerConfig,
        password: Option<String>,
    ) -> Result<usize, JoinGameError> {
        if let Some(game_password) = &self.password {
            if let Some(given_password) = password {
                if game_password.ne(&given_password) {
                    // the provided password does not match the game password
                    return Err(JoinGameError::InvalidPassword);
                }
            } else {
                // we did not provide a password, but the game requires one
                return Err(JoinGameError::InvalidPassword);
            }
        }
        let id = player_config.id; // copy so that we can send the messsage later
        let new_player = Player::new(id, true, self.buy_in);
        let result = self.add_player(player_config, new_player);
        if let Ok(index) = result {
            // note: I tried moving this message to either the hub or in handling meta actions,
            // but this messed up the front end, so I am just moving it back here rather than refactor the front
            let message = object! {
            msg_type: "joined_game".to_owned(),
            index: index,
            table_name: self.name.clone(),
            };
            PlayerConfig::send_specific_message(&message.dump(), id, &self.player_ids_to_configs);
            for (i, player_spot) in self.players.iter().enumerate() {
                // display the play positions for the front end to consume
                if let Some(player) = player_spot {
                    let mut message = object! {
                    msg_type: "player_info".to_owned(),
                    index: i,
                    };
                    let config = self.player_ids_to_configs.get(&player.id).unwrap();
                    let name = config.name.as_ref().unwrap().clone();
                    message["player_name"] = name.into();
                    message["money"] = player.money.into();
                    message["is_active"] = player.is_active.into();
                    PlayerConfig::send_group_message(&message.dump(), &self.player_ids_to_configs);
                }
            }
        }
        result
    }

    pub fn add_bot(&mut self, name: String) -> Result<usize, JoinGameError> {
        let new_bot = Player::new_bot(self.buy_in);
        let new_config = PlayerConfig::new(new_bot.id, Some(name), None);
        self.add_player(new_config, new_bot)
    }

    fn add_player(
        &mut self,
        player_config: PlayerConfig,
        player: Player,
    ) -> Result<usize, JoinGameError> {
        // Kinda weird, but first check if the player is already at the table
        // Could happen if their Leave wasn't completed yet
        // TODO: verify this can actually happen. Unit testable even?
        for (i, player_spot) in self.players.iter_mut().enumerate() {
            if let Some(existing) = player_spot {
                if existing.id == player.id {
                    println!("the player was ALREADY at the table!");
                    self.player_ids_to_configs
                        .insert(player_config.id, player_config);
                    return Ok(i);
                }
            }
        }

        if self.players.iter().flatten().count() >= self.max_players.into() {
            // we already have as many as we can fit in the game
            return Err(JoinGameError::GameIsFull);
        }

        for (i, player_spot) in self.players.iter_mut().enumerate() {
            if player_spot.is_none() {
                *player_spot = Some(player);
                self.player_ids_to_configs
                    .insert(player_config.id, player_config);
                return Ok(i);
            }
        }
        // if we did not early return, then we must have been full
        Err(JoinGameError::GameIsFull)
    }

    pub fn play(
        &mut self,
        incoming_actions: &Arc<Mutex<HashMap<Uuid, PlayerAction>>>,
        incoming_meta_actions: &Arc<Mutex<VecDeque<MetaAction>>>,
        hand_limit: Option<u32>, // how many hands total should be play? None == no limit
    ) {
        let mut non_human_hands = 0; // we only allow a certain number of hands without a human before ending
        loop {
            self.hands_played += 1;
            if let Some(limit) = hand_limit {
                if self.hands_played > limit {
                    println!("hand limit has been reached");
                    break;
                }
            }
            println!(
                "\n\n\nPlaying hand {}, button_idx = {}",
                self.hands_played, self.button_idx
            );
            let num_human_players = self
                .players
                .iter()
                .flatten()
                .filter(|player| player.human_controlled)
                .count();
            if num_human_players == 0 {
                non_human_hands += 1;
                println!("num human players == {:?}", num_human_players);
                println!("non human hands == {:?}", non_human_hands);
            }
            if non_human_hands > NON_HUMAN_HANDS_LIMIT {
                // the game ends no matter what if we haven't had a human after too many turns
                break;
            }
            if self.player_ids_to_configs.len() <= 1 {
                // no use playing a hand, so we just wait for a bit and try again
                continue;
            }
            let message = object! {
            msg_type: "new_hand".to_owned(),
            hand_num: self.hands_played,
            button_index: self.button_idx,
                };
            PlayerConfig::send_group_message(&message.dump(), &self.player_ids_to_configs);

            self.play_one_hand(&incoming_actions, &incoming_meta_actions);
            self.handle_meta_actions(&incoming_meta_actions);
            // attempt to set the next button
            self.button_idx = self
                .find_next_button()
                .expect("we could not find a valid button index!");
        }
        println!("about to send the gameover signal to the hub");
        // the game is ending, so tell that to the hub
        if let Some(hub_addr) = &self.hub_addr {
            // tell the hub that we left
            hub_addr.do_send(GameOver {
                table_name: self.name.clone(),
            });
        }
    }

    /// move the button to the next Player who is not sitting out
    /// if non can be found, then return false
    fn find_next_button(&mut self) -> Result<usize, &'static str> {
        for i in (self.button_idx + 1..9).chain(0..self.button_idx + 1) {
            //self.button_idx += 1;
            //self.button_idx %= 9; // loop back to 0 if we reach the end
            let button_spot = &mut self.players[i];
            if let Some(button_player) = button_spot {
                if button_player.is_sitting_out {
                    println!(
                        "Player at index {} is sitting out so cannot be the button",
                        i
                    );
                } else if button_player.money == 0 {
                    println!("Player at index {} has no money so cannot be the button", i);
                } else {
                    // We found a player who is not sitting out, so it is a valid
                    // button position
                    println!("found the button!");
                    return Ok(i);
                }
            }
        }
        Err("could not find a valid button")
    }

    fn handle_meta_actions(&mut self, incoming_meta_actions: &Arc<Mutex<VecDeque<MetaAction>>>) {
        let mut meta_actions = incoming_meta_actions.lock().unwrap();
        for _ in 0..meta_actions.len() {
            match meta_actions.pop_front().unwrap() {
                MetaAction::Chat(id, text) => {
                    // send the message to all players,
                    // appended by the player name
                    println!("chat message inside the game hand wow!");

                    let name = &self.player_ids_to_configs.get(&id).unwrap().name;
                    let message = object! {
                    msg_type: "chat".to_owned(),
                    player_name: name.clone(),
                    text: text,
                            };

                    PlayerConfig::send_group_message(&message.dump(), &self.player_ids_to_configs);
                }
                MetaAction::Join(player_config, password) => {
                    // add a new player to the game
                    let cloned_config = player_config.clone(); // clone in case we need to send back
                    println!(
                        "handling join meta action for {:?} inside game = {:?}",
                        cloned_config.id, &self.name
                    );
                    match self.add_user(player_config, password) {
                        Ok(index) => {
                            println!("Joining game at index: {}", index);
                        }
                        Err(err) => {
                            // we were unable to add the player
                            println!("unable to join game: {:?}", err);
                            if let Some(hub_addr) = &self.hub_addr {
                                // tell the hub that we left
                                hub_addr.do_send(Returned {
                                    config: cloned_config,
                                    reason: ReturnedReason::FailureToJoin(err),
                                });
                            }
                        }
                    }
                }
                MetaAction::Leave(id) => {
                    println!(
                        "handling leave meta action for {:?} inside game = {:?}",
                        id, &self.name
                    );
                    if let Some(config) = self.player_ids_to_configs.remove(&id) {
                        // note: we don't remove  the player from self.players quite yet,
                        // we use the lack of the config to indicate to the game during a street
                        // that a player has left. If they were active at the time, this information
                        // needs to be taken into account
                        let message = object! {
                            msg_type: "player_left".to_owned(),
                            name: config.name.clone(),
                        };
                        PlayerConfig::send_specific_message(
                            &message.dump(),
                            id,
                            &self.player_ids_to_configs,
                        );

                        if let Some(hub_addr) = &self.hub_addr {
                            // tell the hub that we left
                            hub_addr.do_send(Returned {
                                config,
                                reason: ReturnedReason::Left,
                            });
                        }
                    } else {
                        // should not normally happen, but check for Some() to be safe
                        // Perhaps if the client sent many leave messages before them being responded to
                        println!("\n\nA leave message was received for a player that no longer has a config!")
                    }
                }
                MetaAction::PlayerName(id, new_name) => {
                    PlayerConfig::set_player_name(id, &new_name, &mut self.player_ids_to_configs);
                }
                MetaAction::SitOut(id) => {
                    for player in self.players.iter_mut().flatten() {
                        if player.id == id {
                            println!("player {} being set to is_sitting_out = true", id);
                            player.is_sitting_out = true;
                        }
                    }
                }
                MetaAction::ImBack(id) => {
                    for player in self.players.iter_mut().flatten() {
                        if player.id == id {
                            println!("player {} being set to is_sitting_out = false", id);
                            player.is_sitting_out = false;
                        }
                    }
                }
            }
        }
    }

    fn transition(&mut self, gamehand: &mut GameHand) {
        let pause_duration = time::Duration::from_secs(2);
        thread::sleep(pause_duration);
        match gamehand.street {
            Street::Preflop => {
                gamehand.street = Street::Flop;
                self.deal_flop(gamehand);
                println!(
                    "\n===========================\nFlop = {:?}\n===========================",
                    gamehand.flop
                );
                let message = object! {
                    msg_type: "flop".to_owned(),
                            flop: format!("{}{}{}",
                          gamehand.flop.as_ref().unwrap()[0],
                          gamehand.flop.as_ref().unwrap()[1],
                          gamehand.flop.as_ref().unwrap()[2]),
                };
                PlayerConfig::send_group_message(&message.dump(), &self.player_ids_to_configs);
            }
            Street::Flop => {
                gamehand.street = Street::Turn;
                self.deal_turn(gamehand);
                println!(
                    "\n==========================\nTurn = {:?}\n==========================",
                    gamehand.turn
                );
                let message = object! {
                    msg_type: "turn".to_owned(),
                            turn: format!("{}", gamehand.turn.unwrap())
                };
                PlayerConfig::send_group_message(&message.dump(), &self.player_ids_to_configs);
            }
            Street::Turn => {
                gamehand.street = Street::River;
                self.deal_river(gamehand);
                println!(
                    "\n==========================\nRiver = {:?}\n==========================",
                    gamehand.river
                );
                let message = object! {
                    msg_type: "river".to_owned(),
                            river: format!("{}", gamehand.river.unwrap())
                };
                PlayerConfig::send_group_message(&message.dump(), &self.player_ids_to_configs);
            }
            Street::River => {
                gamehand.street = Street::ShowDown;
                println!(
                    "\n==========================\nShowDown!\n================================"
                );
            }
            Street::ShowDown => (), // we are already in the end street (from players folding during the street)
        }
    }

    fn deal_hands(&mut self) {
        for player in self.players.iter_mut().flatten() {
            if player.is_active {
                for _ in 0..2 {
                    if let Some(card) = self.deck.draw_card() {
                        player.hole_cards.push(card)
                    } else {
                        panic!("The deck is out of cards somehow?");
                    }
                }
                let message = object! {
                    msg_type: "hole_cards".to_owned(),
                    hole_cards: format!("{}{}", player.hole_cards[0], player.hole_cards[1]),
                };
                PlayerConfig::send_specific_message(
                    &message.dump(),
                    player.id,
                    &self.player_ids_to_configs,
                );
            }
        }
    }

    fn deal_flop(&mut self, gamehand: &mut GameHand) {
        let mut flop = Vec::<Card>::with_capacity(3);
        for _ in 0..3 {
            if let Some(card) = self.deck.draw_card() {
                flop.push(card)
            } else {
                panic!("we exhausted the deck somehow");
            }
        }
        gamehand.flop = Some(flop);
    }

    fn deal_turn(&mut self, gamehand: &mut GameHand) {
        gamehand.turn = self.deck.draw_card();
    }

    fn deal_river(&mut self, gamehand: &mut GameHand) {
        gamehand.river = self.deck.draw_card();
    }

    fn finish_hand(&mut self, gamehand: &mut GameHand) {
        // pause for a second for dramatic effect heh
        let pause_duration = time::Duration::from_secs(2);
        thread::sleep(pause_duration);
        if self.player_ids_to_configs.len() < 1 {
            // the game is currently empty, so there is nothing to finish
            return;
        }
        let hand_results: HashMap<Uuid, Option<HandResult>> = self
            .players
            .iter()
            .flatten()
            .map(|player| return (player.id, self.determine_best_hand(player, gamehand)))
            .collect();

        let is_showdown = gamehand.street == Street::ShowDown;
        println!("hand results = {:?}", hand_results);
        if let Street::ShowDown = gamehand.street {
            // if we made it to show down, there are multiple players left, so we need to see who
            // has the best hand.
            println!("Multiple active players made it to showdown!");
            println!("{:?}", gamehand.pot_manager);
            for pot in gamehand.pot_manager.pots.iter() {
                // for each pot, we determine who should get paid out
                // a player can only get paid for a pot that they contributed to
                // so each pot has its own best_hand calculation
                println!("Looking at pot {:?}", pot);
                let mut best_ids = HashSet::<Uuid>::new();
                let mut best_hand: Option<&HandResult> = None;
                for (id, current_opt) in hand_results.iter() {
                    if pot.contributions.get(&id).is_none() {
                        println!("player id {} did not contribute to this pot!", id);
                        continue;
                    }
                    if current_opt.is_none() {
                        continue;
                    }
                    if !self.player_ids_to_configs.contains_key(id) {
                        println!(
                            "player id {} no longer exists in the configs, they must have left!",
                            id
                        );
                        continue;
                    }
                    let current_result = current_opt.as_ref().unwrap();
                    if best_hand.is_none() || current_result > best_hand.unwrap() {
                        println!("new best hand for id {:?}", id);
                        best_hand = Some(current_result);
                        best_ids.clear();
                        best_ids.insert(*id); // only one best hand now
                    } else if current_result == best_hand.unwrap() {
                        println!("equally good hand for id {:?}", id);
                        best_ids.insert(*id); // another index that also has the best hand
                    } else {
                        println!("hand worse for id {:?}", id);
                        continue;
                    }
                }
                // divy the pot to all the winners
                let num_winners = best_ids.len();
                let payout = (pot.money as f64 / num_winners as f64) as u32;
                //self.pot_manager.pots.first_mut().unwrap().money = 0;
                self.pay_players(best_ids, payout, &hand_results, is_showdown);
            }
        } else {
            // the hand ended before Showdown, so we simple find the one active player remaining
            let mut best_ids = HashSet::<Uuid>::new();
            for player in self.players.iter().flatten() {
                if player.is_active {
                    //println!("found an active player remaining");
                    best_ids.insert(player.id);
                } else {
                    println!("found an NON active player remaining");
                }
            }
            // if we didn't make it to show down, there better be only one player left
            assert!(best_ids.len() == 1);
            self.pay_players(
                best_ids,
                gamehand.pot_manager.pots.first().unwrap().money,
                &hand_results,
                is_showdown,
            );
        }

        // take the players' cards
        for player in self.players.iter_mut().flatten() {
            // todo: is there any issue with calling drain if they dont have any cards?
            player.hole_cards.drain(..);
        }
    }

    fn pay_players(
        &mut self,
        best_ids: HashSet<Uuid>,
        payout: u32,
        hand_results: &HashMap<Uuid, Option<HandResult>>,
        is_showdown: bool,
    ) {
        println!("best_indices = {:?}", best_ids);
        for player in self.players.iter_mut().flatten() {
            if best_ids.contains(&player.id) {
                // get the name for messages
                let name: String = if let Some(config) = &self.player_ids_to_configs.get(&player.id)
                {
                    config.name.as_ref().unwrap().clone()
                } else {
                    // it is a bit weird if we made it all the way to the pay stage for a left player
                    "Player who left".to_string()
                };
                let ranking_string =
                    if let Some(hand_result) = hand_results.get(&player.id).unwrap() {
                        hand_result.to_string()
                    } else {
                        "Unknown".to_string()
                    };
                println!(
                    "paying out {:?} to {:?}, with hand result = {:?}",
                    payout, name, ranking_string
                );
                let hole_string = if is_showdown {
                    format!("{}{}", player.hole_cards[0], player.hole_cards[1])
                } else {
                    "Unknown".to_string()
                };

                let message = object! {
                    msg_type: "paying_out".to_owned(),
                    payout: payout,
                    player_name: name,
                    hole_cards: hole_string,
                    hand_result: ranking_string,
                    is_showdown: is_showdown,
                };
                PlayerConfig::send_group_message(&message.dump(), &self.player_ids_to_configs);
                player.pay(payout);
                println!("after payment: {:?}", player);
            }
        }
    }

    /// Given a player, we need to determine which 5 cards make the best hand for this player
    fn determine_best_hand(&self, player: &Player, gamehand: &mut GameHand) -> Option<HandResult> {
        if !player.is_active {
            // if the player isn't active, then can't have a best hand
            return None;
        }

        if let Street::ShowDown = gamehand.street {
            // we look at all possible 7 choose 5 (21) hands from the hole cards, flop, turn, river
            let mut best_result: Option<HandResult> = None;
            let mut hand_count = 0;
            for exclude_idx1 in 0..7 {
                //println!("exclude 1 = {}", exclude_idx1);
                for exclude_idx2 in exclude_idx1 + 1..7 {
                    //println!("exclude 2 = {}", exclude_idx2);
                    let mut possible_hand = Vec::with_capacity(5);
                    hand_count += 1;
                    for (idx, card) in player
                        .hole_cards
                        .iter()
                        .chain(gamehand.flop.as_ref().unwrap().iter())
                        .chain(iter::once(&gamehand.turn.unwrap()))
                        .chain(iter::once(&gamehand.river.unwrap()))
                        .enumerate()
                    {
                        if idx != exclude_idx1 && idx != exclude_idx2 {
                            //println!("pushing!");
                            possible_hand.push(*card);
                        }
                    }
                    // we have built a hand of five cards, now evaluate it
                    let current_result = HandResult::analyze_hand(possible_hand);
                    match best_result {
                        None => best_result = Some(current_result),
                        Some(result) if current_result > result => {
                            best_result = Some(current_result)
                        }
                        _ => (),
                    }
                }
            }
            assert!(hand_count == 21); // 7 choose 5
            println!("player = {:?}", player.id);
            println!("best result = {:?}", best_result);
            best_result
        } else {
            None
        }
    }

    fn play_one_hand(
        &mut self,
        incoming_actions: &Arc<Mutex<HashMap<Uuid, PlayerAction>>>,
        incoming_meta_actions: &Arc<Mutex<VecDeque<MetaAction>>>,
    ) {
        println!("inside of play(). button_idx = {:?}", self.button_idx);
        let mut gamehand = GameHand::default();

        for player in self.players.iter_mut().flatten() {
            if player.is_sitting_out || player.money == 0 {
                player.is_active = false;
            } else {
                player.is_active = true;
            }
        }
        for (i, player_spot) in self.players.iter().enumerate() {
            // display the play positions for the front end to consume
            if let Some(player) = player_spot {
                let mut message = object! {
                    msg_type: "player_info".to_owned(),
                    index: i
                };
                let config = self.player_ids_to_configs.get(&player.id).unwrap();
                let name = config.name.as_ref().unwrap().clone();
                message["player_name"] = name.into();
                message["money"] = player.money.into();
                message["is_active"] = player.is_active.into();
                PlayerConfig::send_group_message(&message.dump(), &self.player_ids_to_configs);
            }
        }

        self.deck.shuffle();
        self.deal_hands();

        println!("players = {:?}", self.players);

        while gamehand.street != Street::ShowDown {
            // pause for a second for dramatic effect heh
            let pause_duration = time::Duration::from_secs(2);
            thread::sleep(pause_duration);
            let finished =
                self.play_street(&incoming_actions, &incoming_meta_actions, &mut gamehand);
            if finished {
                // if the game is over from players folding
                println!("\nGame is ending before showdown!");
                // TODO is a msg here needed?
                //PlayerConfig::send_group_message("\nGame is ending before showdown!", player_ids_to_configs);
                break;
            } else {
                // otherwise we move to the next street
                self.transition(&mut gamehand)
            }
        }
        // now we finish up and pay the pot to the winner
        self.finish_hand(&mut gamehand);
    }

    fn get_starting_idx(&self) -> usize {
        // the starting index is either the person one more from the button on most streets,
        // or 3 down on the preflop (since the blinds already had to buy in)
        // TODO: this needs to be smarter in small games
        let mut starting_idx = self.button_idx + 1;
        if starting_idx as usize >= self.players.len() {
            starting_idx = 0;
        }
        starting_idx
    }

    /// this method returns a bool indicating whether the hand is over or not
    fn play_street(
        &mut self,
        incoming_actions: &Arc<Mutex<HashMap<Uuid, PlayerAction>>>,
        incoming_meta_actions: &Arc<Mutex<VecDeque<MetaAction>>>,
        gamehand: &mut GameHand,
    ) -> bool {
        let mut current_bet: u32 = 0;
        // each index keeps track of that players' contribution this street
        let mut cumulative_bets = vec![0; self.players.len()];

        let starting_idx = self.get_starting_idx(); // which player starts the betting

        // if a player is still active but has no remaining money (i.e. is all-in),
        let mut num_all_in = self
            .players
            .iter()
            .flatten() // skip over None values
            .filter(|player| player.is_all_in())
            .count();

        let mut num_active = self
            .players
            .iter()
            .flatten() // skip over None values
            .filter(|player| player.is_active)
            .count();
        if num_active < 2 {
            println!(
                "num_active players = {}, so we cannot play a hand!",
                num_active
            );
            return true;
        }

        if num_all_in + 1 == num_active {
            println!("only one person is not all in, so don't bother with the street!");
            return false;
        }

        // once every player is either all-in or settled, then we move to the next street
        let mut num_settled = 0; // keep track of how many players have put in enough chips to move on

        println!("num active players = {}", num_active);
        //PlayerConfig::send_group_message(&format!("num active players = {}", num_active), player_ids_to_configs);

        println!("player at index {} starts the betting", starting_idx);
        if num_settled > 0 {
            println!("num settled (i.e. all in players) = {}", num_settled);
            PlayerConfig::send_group_message(
                &format!("num settled (i.e. all in players) = {}", num_settled),
                &self.player_ids_to_configs,
            );
        }
        // iterate over the players from the starting index to the end of the vec,
        // and then from the beginning back to the starting index
        //let (left, right) = players.split_at_mut(starting_idx);
        //for (i, mut player) in right.iter_mut().chain(left.iter_mut()).flatten().enumerate() {
        for i in (starting_idx..9).chain(0..starting_idx).cycle() {
            /*
            println!(
                "start loop index = {}: num_active = {}, num_settled = {}, num_all_in = {}",
                i, num_active, num_settled, num_all_in
            );*/
            if num_active == 1 {
                println!("Only one active player left so lets break the steet loop");
                // end the street and indicate to the caller that the hand is finished
                return true;
            }
            if num_settled + num_all_in == num_active {
                println!(
                    "everyone is ready to go to the next street! num_settled = {}",
                    num_settled
                );
                // end the street and indicate to the caller that the hand is going to the next street
                return false;
            }

            if self.players[i].is_none() {
                // no one sitting in this spot
                continue;
            }

            // we clone() the current player so that we can use its information
            // while also possibly updating players (if a player leaves or joins the game in handle_meta_actions)
            // if we handle_meta_actions BEFORE accessing the current player, then we will have to wait
            // a long time between user messages, which is a worse user experience
            // the Player struct is not super heavy to clone.
            let player = self.players[i].clone().unwrap();
            let player_cumulative = cumulative_bets[i];
            println!("Current pot = {:?}, Current size of the bet = {:?}, and this player has put in {:?} so far",
		     gamehand.pot_manager,
		     current_bet,
		     player_cumulative);
            println!("Player = {:?}, i = {}", player.id, i);
            if !(player.is_active && player.money > 0) {
                continue;
            }
            // get the name for messages
            let name = if let Some(config) = &self.player_ids_to_configs.get(&player.id) {
                config.name.as_ref().unwrap().clone()
            } else {
                "Player who left".to_string()
            };

            let message = object! {
            msg_type: "player_to_act".to_owned(),
            index: i,
            player_name: name.clone(),
            };

            PlayerConfig::send_group_message(&message.dump(), &self.player_ids_to_configs);

            let action = self.get_and_validate_action(
                &incoming_actions,
                &incoming_meta_actions,
                &player,
                current_bet,
                player_cumulative,
                gamehand,
            );

            let mut message = object! {
            msg_type: "player_action".to_owned(),
            index: i,
            player_name: name
            };

            // now that we have gotten the current player's action and handled
            // any meta actions, we are free to respond and mutate the player
            // so we re-borrow it as mutable
            let player = self.players[i].as_mut().unwrap();
            match action {
                PlayerAction::PostSmallBlind(amount) => {
                    message["action"] = "small blind".into();
                    message["amount"] = amount.into();
                    cumulative_bets[i] += amount;
                    gamehand.total_contributions[i] += amount;
                    player.money -= amount;
                    // regardless if the player couldn't afford it, the new street bet is the big blind
                    current_bet = self.small_blind;
                    let all_in = if player.is_all_in() {
                        num_all_in += 1;
                        true
                    } else {
                        false
                    };
                    gamehand.pot_manager.contribute(player.id, amount, all_in);
                }
                PlayerAction::PostBigBlind(amount) => {
                    message["action"] = "big blind".into();
                    message["amount"] = amount.into();
                    cumulative_bets[i] += amount;
                    gamehand.total_contributions[i] += amount;
                    player.money -= amount;
                    // regardless if the player couldn't afford it, the new street bet is the big blind
                    current_bet = self.big_blind;
                    let all_in = if player.is_all_in() {
                        num_all_in += 1;
                        true
                    } else {
                        false
                    };
                    gamehand.pot_manager.contribute(player.id, amount, all_in);
                    // note: we dont count the big blind as a "settled" player,
                    // since they still get a chance to act after the small blind
                }
                PlayerAction::Fold => {
                    message["action"] = "fold".into();
                    player.deactivate();
                    num_active -= 1;
                }
                PlayerAction::Check => {
                    message["action"] = "check".into();
                    num_settled += 1;
                }
                PlayerAction::Call => {
                    message["action"] = "call".into();
                    let difference = current_bet - player_cumulative;
                    if difference >= player.money {
                        println!("you have to put in the rest of your chips");
                        gamehand
                            .pot_manager
                            .contribute(player.id, player.money, true);
                        cumulative_bets[i] += player.money;
                        gamehand.total_contributions[i] += player.money;
                        message["amount"] = player.money.into();
                        player.money = 0;
                        num_all_in += 1;
                    } else {
                        gamehand
                            .pot_manager
                            .contribute(player.id, difference, false);
                        cumulative_bets[i] += difference;
                        gamehand.total_contributions[i] += difference;
                        message["amount"] = difference.into();
                        player.money -= difference;
                        num_settled += 1;
                    }
                }
                PlayerAction::Bet(new_bet) => {
                    let difference = new_bet - player_cumulative;
                    println!("difference = {}", difference);
                    player.money -= difference;
                    current_bet = new_bet;
                    cumulative_bets[i] += difference;
                    println!("sup {:?}", player);
                    gamehand.total_contributions[i] += difference;
                    let all_in = if player.is_all_in() {
                        println!("Just bet the rest of our money!");
                        num_all_in += 1;
                        num_settled = 0;
                        true
                    } else {
                        num_settled = 1;
                        false
                    };
                    gamehand
                        .pot_manager
                        .contribute(player.id, difference, all_in);
                    message["action"] = "bet".into();
                    message["amount"] = new_bet.into();
                }
            }
            message["money"] = player.money.into();
            message["pots"] = gamehand.pot_manager.simple_repr().into();
            message["is_active"] = player.is_active.into();
            message["street_contributions"] = cumulative_bets[i].into();
            message["current_bet"] = current_bet.into();

            println!("{}", message.dump());
            PlayerConfig::send_group_message(&message.dump(), &self.player_ids_to_configs);

            // double check at the end of the loop if any players left as a meta-action during the current
            // player's turn. They should no longer be considered as active or all_in
            for player_spot in self.players.iter_mut() {
                if let Some(player) = player_spot {
                    if !self.player_ids_to_configs.contains_key(&player.id) {
                        println!("player is no longer in the config at the end of the loop");
                        if player.is_all_in() {
                            num_all_in -= 1;
                        }
                        if player.is_active {
                            player.deactivate(); // technically redundant I guess since setting to None later
                            num_active -= 1;
                        }
                        *player_spot = None;
                    }
                }
            }
        }
        true // we can't actually get to this line
    }

    /// if the player is a human, then we look for their action in the incoming_actions hashmap
    /// this value is set by the game hub when handling a message from a player client
    fn get_action_from_player(
        &self,
        incoming_actions: &Arc<Mutex<HashMap<Uuid, PlayerAction>>>,
        player: &Player,
    ) -> Option<PlayerAction> {
        if player.human_controlled {
            let mut actions = incoming_actions.lock().unwrap();
            println!("incoming_actions = {:?}", actions);
            if let Some(action) = actions.get_mut(&player.id) {
                println!("Player: {:?} has action {:?}", player.id, action);
                let value = *action;
                actions.remove(&player.id); // wipe this action so we don't repeat it next time
                Some(value)
            } else {
                None
            }
        } else {
            let num = rand::thread_rng().gen_range(0..100);
            match num {
                0..=20 => Some(PlayerAction::Fold),
                21..=55 => Some(PlayerAction::Check),
                56..=70 => {
                    let amount: u32 = if player.money <= 100 {
                        // just go all in if we are at 10% starting
                        player.money
                    } else {
                        rand::thread_rng().gen_range(1..player.money / 2 as u32)
                    };
                    Some(PlayerAction::Bet(amount))
                }
                _ => Some(PlayerAction::Call),
            }
        }
    }

    fn get_and_validate_action(
        &mut self,
        incoming_actions: &Arc<Mutex<HashMap<Uuid, PlayerAction>>>,
        incoming_meta_actions: &Arc<Mutex<VecDeque<MetaAction>>>,
        player: &Player,
        current_bet: u32,
        player_cumulative: u32,
        gamehand: &mut GameHand,
    ) -> PlayerAction {
        // if it isnt valid based on the current bet and the amount the player has already contributed,
        // then it loops
        // position is our spot in the order, with 0 == small blind, etc

        // we sleep a little bit each time so that the output doesnt flood the user at one moment
        let pause_duration = time::Duration::from_secs(1);
        thread::sleep(pause_duration);

        if gamehand.street == Street::Preflop && current_bet == 0 {
            // collect small blind!
            return PlayerAction::PostSmallBlind(cmp::min(self.small_blind, player.money));
        } else if gamehand.street == Street::Preflop && current_bet == self.small_blind {
            // collect big blind!
            return PlayerAction::PostBigBlind(cmp::min(self.big_blind, player.money));
        }
        let prompt = if current_bet > player_cumulative {
            let diff = current_bet - player_cumulative;
            format!("Enter action ({} to call): ", diff)
        } else {
            format!("Enter action (current bet = {}): ", current_bet)
        };
        let message = object! {
            msg_type: "prompt".to_owned(),
            prompt: prompt,
        };
        PlayerConfig::send_specific_message(
            &message.dump(),
            player.id,
            &self.player_ids_to_configs,
        );

        let mut action = None;
        let mut attempts = 0;
        let retry_duration = time::Duration::from_secs(1); // how long to wait between trying again
        while attempts < 10000 && action.is_none() {
            // the first thing we do on each loop is handle meta action
            // this lets us display messages in real-time without having to wait until after the
            // current player gives their action
            self.handle_meta_actions(&incoming_meta_actions);

            if player.human_controlled {
                // we don't need to count the attempts at getting a response from a computer
                // TODO: the computer can give a better than random guess at a move
                // Currently it might try to check when it has to call for example,
                attempts += 1;
            }
            if player.is_sitting_out {
                println!("player is sitting out, so fold");
                action = Some(PlayerAction::Fold);
                break;
            }
            if !self.player_ids_to_configs.contains_key(&player.id) {
                // the config no longer exists for this player, so they must have left
                println!("player config no longer exists, so the player must have left");
                action = Some(PlayerAction::Fold);
                break;
            }

            println!("Attempting to get player action on attempt {:?}", attempts);
            match self.get_action_from_player(&incoming_actions, player) {
                None => {
                    // println!("No action is set for the player {:?}", player.id);
                    // we give the user a second to place their action
                    thread::sleep(retry_duration);
                }

                Some(PlayerAction::Fold) => {
                    if current_bet <= player_cumulative {
                        // if the player has put in enough then no sense folding
                        if player.human_controlled {
                            println!("you said fold but we will let you check!");
                            let message = json::object! {
                            msg_type: "error".to_owned(),
                            error: "invalid_action".to_owned(),
                            reason: "You said fold but we will let you check!".to_owned(),
                            };
                            PlayerConfig::send_specific_message(
                                &message.dump(),
                                player.id,
                                &self.player_ids_to_configs,
                            );
                        }
                        action = Some(PlayerAction::Check);
                    } else {
                        action = Some(PlayerAction::Fold);
                    }
                }
                Some(PlayerAction::Check) => {
                    //println!("Player checks!");
                    if current_bet > player_cumulative {
                        // if the current bet is higher than this player's bet
                        if player.human_controlled {
                            let message = json::object! {
                            msg_type: "error".to_owned(),
                            error: "invalid_action".to_owned(),
                            reason: "You can't check since there is a bet!!".to_owned(),
                            };
                            PlayerConfig::send_specific_message(
                                &message.dump(),
                                player.id,
                                &self.player_ids_to_configs,
                            );
                        }
                        continue;
                    }
                    action = Some(PlayerAction::Check);
                }
                Some(PlayerAction::Call) => {
                    if current_bet <= player_cumulative {
                        if current_bet != 0 {
                            // if the street bet isn't 0 then this makes no sense
                            println!("should we even be here???!");
                        }
                        let message = json::object! {
                            msg_type: "error".to_owned(),
                            error: "invalid_action".to_owned(),
                            reason: "There is nothing for you to call!".to_owned()
                        };
                        PlayerConfig::send_specific_message(
                            &message.dump(),
                            player.id,
                            &self.player_ids_to_configs,
                        );
                        // we COULD let them check, but better to wait for a better action
                        continue;
                    }
                    action = Some(PlayerAction::Call);
                }
                Some(PlayerAction::Bet(new_bet)) => {
                    if current_bet < player_cumulative {
                        // will this case happen?
                        println!("this should not happen!");
                        continue;
                    }
                    // TODO: this line blew up
                    // thread '<unnamed>' panicked at 'attempt to subtract with overflow', src/logic/game.rs:738:24
                    // note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
                    // I think we need to change the money amounts to be signed
                    // OR it might be better to look into/fix the bet logic
                    // like should new_bet just be a standalone thing above the current bet?
                    // do we need to add raising?

                    // NOTE ---> I changed it now
                    if new_bet > player.money + player_cumulative {
                        println!("cant bet more than you have");
                        let message = json::object! {
                            msg_type: "error".to_owned(),
                            error: "invalid_action".to_owned(),
                            reason:"You can't bet more than you have!!".to_owned(),
                        };
                        PlayerConfig::send_specific_message(
                            &message.dump(),
                            player.id,
                            &self.player_ids_to_configs,
                        );
                        continue;
                    }
                    if new_bet <= current_bet {
                        println!("new bet must be larger than current");
                        let message = json::object! {
                            msg_type: "error".to_owned(),
                            error: "invalid_action".to_owned(),
                            reason: "the new bet must be larger than the current bet!".to_owned(),
                        };
                        PlayerConfig::send_specific_message(
                            &message.dump(),
                            player.id,
                            &self.player_ids_to_configs,
                        );
                        continue;
                    }
                    action = Some(PlayerAction::Bet(new_bet));
                }
                other => {
                    action = other;
                }
            }
        }
        // if we got a valid action, then we can return it,
        // otherwise, we just return Fold
        if action.is_some() {
            action.unwrap()
        } else {
            PlayerAction::Fold
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::card::{Card, Rank, RiggedDeck, Suit};
    use std::collections::HashMap;

    #[test]
    fn add_bot() {
        let mut game = Game::default();
        let name = "Mr Bot".to_string();
        let index = game.add_bot(name);
        assert_eq!(index.unwrap(), 0); // the first position to be added to is index 0
        assert_eq!(game.players.len(), 9);
        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 1);
        assert!(!game.players[0].as_ref().unwrap().human_controlled);
    }

    #[test]
    fn add_user() {
        let mut game = Game::default();
        let id = uuid::Uuid::new_v4();
        let name = "Human".to_string();
        let settings = PlayerConfig::new(id, Some(name), None);
        game.add_user(settings, None).expect("could not add user");
        assert_eq!(game.players.len(), 9);
        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 1);
        assert!(game.players[0].as_ref().unwrap().human_controlled);
    }

    /// test that a player can join with the correct password
    #[test]
    fn add_user_password_success() {
        let mut game = Game::default();
        //let _incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        //let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        let password = "123".to_string();
        game.password = Some(password.clone());

        let id = uuid::Uuid::new_v4();
        let name = "Human".to_string();
        let settings = PlayerConfig::new(id, Some(name), None);

        incoming_meta_actions
            .lock()
            .unwrap()
            .push_back(MetaAction::Join(settings, Some(password)));

        game.handle_meta_actions(&cloned_meta_actions);

        assert_eq!(game.players.len(), 9);
        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 1);
        assert!(game.players[0].as_ref().unwrap().human_controlled);
    }

    /// test that a player can NOT join with the incorrect password
    #[test]
    fn add_user_password_fail() {
        let mut game = Game::default();
        //let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));

        game.password = Some("123".to_string());

        let id = uuid::Uuid::new_v4();
        let name = "Human".to_string();
        let settings = PlayerConfig::new(id, Some(name), None);

        incoming_meta_actions
            .lock()
            .unwrap()
            .push_back(MetaAction::Join(settings, Some("345".to_string())));

        game.handle_meta_actions(&incoming_meta_actions);

        assert_eq!(game.players.len(), 9);
        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 0); // did not make it in
    }

    /// test that a player can NOT join without providing a password
    #[test]
    fn add_user_password_missing() {
        let mut game = Game::default();
        //let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));

        game.password = Some("123".to_string());

        let id = uuid::Uuid::new_v4();
        let name = "Human".to_string();
        let settings = PlayerConfig::new(id, Some(name), None);

        incoming_meta_actions
            .lock()
            .unwrap()
            .push_back(MetaAction::Join(settings, None)); // no password passed in

        game.handle_meta_actions(&incoming_meta_actions);

        assert_eq!(game.players.len(), 9);
        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 0); // did not make it in
    }

    /// if we set max_players, then trying to add anyone past that point will
    /// not work
    #[test]
    fn max_players_in_game_() {
        let mut game = Game::default();
        let max_players = 3;
        game.max_players = max_players;

        // we TRY to add 5 bots
        for i in 0..5 {
            let name = format!("Bot {}", i);
            let index = game.add_bot(name);
            if i < max_players {
                assert_eq!(index.unwrap() as u8, i);
            } else {
                // above max_players, the returned index should be None
                // i.e. the player was not added to the game
                assert!(index.is_err());
            }
        }
        assert_eq!(game.players.len(), 9); // len of players always simply 9

        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        // but only max_players players are in the game at the end
        assert_eq!(some_players as u8, max_players);
    }

    /// the small blind folds, so the big blind should win and get paid
    #[test]
    fn instant_fold() {
        let mut game = Game::default();
        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        // player1 will start as the button
        let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1, None).unwrap();

        // player2 will start as the small blind
        let id2 = uuid::Uuid::new_v4();
        let name2 = "Human1".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2, None).unwrap();
        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);

        let handler = std::thread::spawn(move || {
            game.play_one_hand(&cloned_actions, &cloned_meta_actions);
            game // return the game back
        });

        // set the action that player2 folds
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Fold);

        // get the game back from the thread
        let game = handler.join().unwrap();

        // check that the money changed hands
        assert_eq!(game.players[0].as_ref().unwrap().money, 1004);
        assert_eq!(game.players[1].as_ref().unwrap().money, 996);
    }

    /// the small blind calls, the big blind checks to the flop
    /// the small blind bets on the flop, and the big blind folds
    #[test]
    fn call_check_bet_fold() {
        let mut game = Game::default();
        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        // player1 will start as the button
        let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1, None).unwrap();

        // player2 will start as the small blind
        let id2 = uuid::Uuid::new_v4();
        let name2 = "Human1".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2, None).unwrap();
        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);

        let handler = std::thread::spawn(move || {
            game.play_one_hand(&cloned_actions, &cloned_meta_actions);
            game // return the game back
        });

        // set the action that player2 calls
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Call);
        // player1 checks
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Check);

        // wait for the flop
        let wait_duration = time::Duration::from_secs(7);
        thread::sleep(wait_duration);

        // player2 bets on the flop
        println!("now sending the flop actions");
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Bet(10));
        // player1 folds
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Fold);

        // get the game back from the thread
        let game = handler.join().unwrap();

        // check that the money changed hands
        assert_eq!(game.players[0].as_ref().unwrap().money, 992);
        assert_eq!(game.players[1].as_ref().unwrap().money, 1008);
    }

    /// the small blind bets, the big blind folds
    #[test]
    fn pre_flop_bet_fold() {
        let mut game = Game::default();
        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        // player1 will start as the button
        let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1, None).unwrap();

        // player2 will start as the small blind
        let id2 = uuid::Uuid::new_v4();
        let name2 = "Human1".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2, None).unwrap();
        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);

        let handler = std::thread::spawn(move || {
            game.play_one_hand(&cloned_actions, &cloned_meta_actions);
            game // return the game back
        });

        // set the action that player2 bets
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Bet(22));
        // player1 folds
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Fold);

        // get the game back from the thread
        let game = handler.join().unwrap();

        // check that the money changed hands
        assert_eq!(game.players[0].as_ref().unwrap().money, 992);
        assert_eq!(game.players[1].as_ref().unwrap().money, 1008);
    }

    /// if the big blind player doesn't have enough to post the big blind amount,
    /// the current bet still goes up to the big blind
    #[test]
    fn big_blind_not_enough_money() {
        let mut deck = RiggedDeck::new();

        // we want the button/big blind to win
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Heart,
        });
        // now the small blind's hole cards
        deck.push(Card {
            rank: Rank::Two,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Three,
            suit: Suit::Club,
        });
        // now the full run out
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Diamond,
        });
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Spade,
        });
        deck.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::King,
            suit: Suit::Heart,
        });
        deck.push(Card {
            rank: Rank::Queen,
            suit: Suit::Club,
        });

        let mut game = Game::default();
        game.deck = Box::new(deck);

        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        // player1 will start as the button/big blind
        let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1, None).unwrap();
        game.players[0].as_mut().unwrap().money = 3; // set the player to have less than the norm 8 BB

        // player2 will start as the small blind
        let id2 = uuid::Uuid::new_v4();
        let name2 = "Human1".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2, None).unwrap();
        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);

        let handler = std::thread::spawn(move || {
            game.play_one_hand(&cloned_actions, &cloned_meta_actions);
            game // return the game back
        });

        // set the action that player (small blind) bets,
        // even though player1 is already all-in, so the BB can only 3 win bucks
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Bet(22));

        // get the game back from the thread
        let game = handler.join().unwrap();

        // check that the money changed hands
        assert_eq!(game.players[0].as_ref().unwrap().money, 6);
        assert_eq!(game.players[1].as_ref().unwrap().money, 997);
    }

    /// the small blind bets, the big blind calls
    /// the small blind bets on the flop, and the big blind folds
    #[test]
    fn bet_call_bet_fold() {
        let mut game = Game::default();
        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        // player1 will start as the button
        let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1, None).unwrap();

        // player2 will start as the small blind
        let id2 = uuid::Uuid::new_v4();
        let name2 = "Human1".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2, None).unwrap();
        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);

        let handler = std::thread::spawn(move || {
            game.play_one_hand(&cloned_actions, &cloned_meta_actions);
            game // return the game back
        });

        // set the action that player2 bets
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Bet(22));
        // player1 calls
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Call);

        // wait for the flop
        let wait_duration = time::Duration::from_secs(7);
        thread::sleep(wait_duration);

        // player2 bets on the flop
        println!("now sending the flop actions");
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Bet(10));
        // player1 folds
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Fold);

        // get the game back from the thread
        let game = handler.join().unwrap();

        // check that the money changed hands
        assert_eq!(game.players[0].as_ref().unwrap().money, 978);
        assert_eq!(game.players[1].as_ref().unwrap().money, 1022);
    }

    /// the small blind goes all in and the big blind calls
    #[test]
    fn all_in_call() {
        let mut deck = RiggedDeck::new();

        // we want the button/big blind to lose for testing purposes
        deck.push(Card {
            rank: Rank::Two,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Three,
            suit: Suit::Club,
        });

        // now the small blind's hole cards
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Heart,
        });

        // now the full run out
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Diamond,
        });
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Spade,
        });
        deck.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::King,
            suit: Suit::Heart,
        });
        deck.push(Card {
            rank: Rank::Queen,
            suit: Suit::Club,
        });

        let mut game = Game::default();
        game.deck = Box::new(deck);
        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        // player1 will start as the button
        let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1, None).unwrap();

        // player2 will start as the small blind
        let id2 = uuid::Uuid::new_v4();
        let name2 = "Human1".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2, None).unwrap();
        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);

        let handler = std::thread::spawn(move || {
            game.play_one_hand(&cloned_actions, &cloned_meta_actions);
            game // return the game back
        });

        // set the action that player2 bets
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Bet(1000));
        // player1 calls
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Call);

        // get the game back from the thread
        let game = handler.join().unwrap();

        // the small blind won
        assert_eq!(game.players[0].as_ref().unwrap().money, 0);
        assert_eq!(game.players[1].as_ref().unwrap().money, 2000);
    }

    /// the small blind bets and the big blind calls
    /// this call makes the big blind go all-in
    #[test]
    fn call_all_in() {
        let mut deck = RiggedDeck::new();

        // we want the button/big blind to lose for testing purposes
        deck.push(Card {
            rank: Rank::Two,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Three,
            suit: Suit::Club,
        });

        // now the small blind's hole cards
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Heart,
        });

        // now the full run out
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Diamond,
        });
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Spade,
        });
        deck.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::King,
            suit: Suit::Heart,
        });
        deck.push(Card {
            rank: Rank::Queen,
            suit: Suit::Club,
        });

        let mut game = Game::default();
        game.deck = Box::new(deck);
        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        // player1 will start as the button
        let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1, None).unwrap();

        game.players[0].as_mut().unwrap().money = 500; // set the player to have less money

        // player2 will start as the small blind
        let id2 = uuid::Uuid::new_v4();
        let name2 = "Human1".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2, None).unwrap();
        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);

        let handler = std::thread::spawn(move || {
            game.play_one_hand(&cloned_actions, &cloned_meta_actions);
            game // return the game back
        });

        // set the action that player2 bets
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Bet(500));
        // player1 calls
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Call);

        // get the game back from the thread
        let game = handler.join().unwrap();

        // the small blind won
        assert_eq!(game.players[0].as_ref().unwrap().money, 0);
        assert_eq!(game.players[1].as_ref().unwrap().money, 1500);
    }

    /// the small blind bets and the big blind calls
    /// this call makes the big blind go all-in
    /// In this test, the original bet is more than the big blind even has,
    /// and the big blind wins only the amount it puts in (500)
    #[test]
    fn small_stack_call_all_in() {
        let mut deck = RiggedDeck::new();

        // we want the button/big blind to win for testing purposes
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Heart,
        });

        // now the small blind's losing hole cards
        deck.push(Card {
            rank: Rank::Two,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Three,
            suit: Suit::Club,
        });

        // now the full run out
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Diamond,
        });
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Spade,
        });
        deck.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::King,
            suit: Suit::Heart,
        });
        deck.push(Card {
            rank: Rank::Queen,
            suit: Suit::Club,
        });

        let mut game = Game::default();
        game.deck = Box::new(deck);
        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        // player1 will start as the button/big
        let id1 = uuid::Uuid::new_v4();
        let name1 = "Big".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1, None).unwrap();

        game.players[0].as_mut().unwrap().money = 500; // set the player to have less money

        // player2 will start as the small blind
        let id2 = uuid::Uuid::new_v4();
        let name2 = "Small".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2, None).unwrap();
        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);

        let handler = std::thread::spawn(move || {
            game.play_one_hand(&cloned_actions, &cloned_meta_actions);
            game // return the game back
        });

        // set the action that player2 bets a bunch
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Bet(1000));
        // player1 calls
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Call);

        // get the game back from the thread
        let game = handler.join().unwrap();

        // the big blind caller won, but only doubles its money
        assert_eq!(game.players[0].as_ref().unwrap().money, 1000);

        // the small blind only loses half
        assert_eq!(game.players[1].as_ref().unwrap().money, 500);
    }

    /// if a player goes all-in, then can only win as much as is called up to that amount,
    /// even if other players keep playing and betting during this hand
    /// In this test, the side pot is won by the short stack, then the remaining is won
    /// by another player
    #[test]
    fn outright_side_pot() {
        let mut deck = RiggedDeck::new();

        // we want the button to win his side pot
        deck.push(Card {
            rank: Rank::Ace,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Ace,
            suit: Suit::Diamond,
        });

        // the small blind will win the main pot against the big blind
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Ten,
            suit: Suit::Heart,
        });

        // the big blind loses
        deck.push(Card {
            rank: Rank::Two,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Four,
            suit: Suit::Heart,
        });

        // now the full run out
        deck.push(Card {
            rank: Rank::Three,
            suit: Suit::Diamond,
        });
        deck.push(Card {
            rank: Rank::Eight,
            suit: Suit::Spade,
        });
        deck.push(Card {
            rank: Rank::Nine,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::King,
            suit: Suit::Heart,
        });
        deck.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });

        let mut game = Game::default();
        game.deck = Box::new(deck);
        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        // player1 will start as the button
        let id1 = uuid::Uuid::new_v4();
        let name1 = "Button".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1, None).unwrap();
        // set the button to have less money so there is a side pot
        game.players[0].as_mut().unwrap().money = 500;

        // player2 will start as the small blind
        let id2 = uuid::Uuid::new_v4();
        let name2 = "Small".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2, None).unwrap();

        // player3 will start as the big blind
        let id3 = uuid::Uuid::new_v4();
        let name3 = "Big".to_string();
        let settings3 = PlayerConfig::new(id3, Some(name3), None);
        game.add_user(settings3, None).unwrap();

        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 3);
        assert!(game.players[0].as_ref().unwrap().human_controlled);
        assert!(game.players[1].as_ref().unwrap().human_controlled);
        assert!(game.players[2].as_ref().unwrap().human_controlled);

        let handler = std::thread::spawn(move || {
            game.play_one_hand(&cloned_actions, &cloned_meta_actions);
            game // return the game back
        });

        // the button goes all in with the short stack
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Bet(500));
        // the small blind goes all in with a full stack
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Bet(1000));
        // the big blind calls the full all-in
        incoming_actions
            .lock()
            .unwrap()
            .insert(id3, PlayerAction::Call);

        // get the game back from the thread
        let game = handler.join().unwrap();

        // the button won the side pot
        assert_eq!(game.players[0].as_ref().unwrap().money, 1500);

        // the small blind won the remainder
        assert_eq!(game.players[1].as_ref().unwrap().money, 1000);

        // the big blind lost everything
        assert_eq!(game.players[2].as_ref().unwrap().money, 0);
    }

    /// if a player goes all-in, then can only win as much as is called up to that amount,
    /// even if other players keep playing and betting during this hand
    /// In this test, the small stack ties with one of the other players, so the main spot should be split
    /// This other player beats the third player in the side pot
    #[test]
    fn tie_side_pot() {
        let mut deck = RiggedDeck::new();

        // we want the button to win the main pot
        deck.push(Card {
            rank: Rank::Ace,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Ace,
            suit: Suit::Diamond,
        });

        // the small blind will tie the main and win the side pot against the big blind
        deck.push(Card {
            rank: Rank::Ace,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Ace,
            suit: Suit::Heart,
        });

        // the big blind loses
        deck.push(Card {
            rank: Rank::Two,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Four,
            suit: Suit::Heart,
        });

        // now the full run out
        deck.push(Card {
            rank: Rank::Three,
            suit: Suit::Diamond,
        });
        deck.push(Card {
            rank: Rank::Eight,
            suit: Suit::Spade,
        });
        deck.push(Card {
            rank: Rank::Nine,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::King,
            suit: Suit::Heart,
        });
        deck.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });

        let mut game = Game::default();
        game.deck = Box::new(deck);
        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        // player1 will start as the button
        let id1 = uuid::Uuid::new_v4();
        let name1 = "Button".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1, None).unwrap();
        // set the button to have less money so there is a side pot
        game.players[0].as_mut().unwrap().money = 500;

        // player2 will start as the small blind
        let id2 = uuid::Uuid::new_v4();
        let name2 = "Small".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2, None).unwrap();

        // player3 will start as the big blind
        let id3 = uuid::Uuid::new_v4();
        let name3 = "Big".to_string();
        let settings3 = PlayerConfig::new(id3, Some(name3), None);
        game.add_user(settings3, None).unwrap();

        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 3);
        assert!(game.players[0].as_ref().unwrap().human_controlled);
        assert!(game.players[1].as_ref().unwrap().human_controlled);
        assert!(game.players[2].as_ref().unwrap().human_controlled);

        let handler = std::thread::spawn(move || {
            game.play_one_hand(&cloned_actions, &cloned_meta_actions);
            game // return the game back
        });

        // the button goes all in with the short stack
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Bet(500));
        // the small blind goes all in with a full stack
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Bet(1000));
        // the big blind calls the full all-in
        incoming_actions
            .lock()
            .unwrap()
            .insert(id3, PlayerAction::Call);

        // get the game back from the thread
        let game = handler.join().unwrap();

        // the button won the side pot
        assert_eq!(game.players[0].as_ref().unwrap().money, 750);

        // the small blind won the remainder
        assert_eq!(game.players[1].as_ref().unwrap().money, 1750);

        // the big blind lost everything
        assert_eq!(game.players[2].as_ref().unwrap().money, 0);
    }

    /// if a player goes all-in, then can only win as much as is called up to that amount,
    /// even if other players keep playing and betting during this hand
    /// In this test, the main pot is won by the small stack, then medium stack wins a separate
    /// side pot, and finally, the rest of the chips are won by a third player

    #[test]
    fn multiple_side_pots() {
        let mut deck = RiggedDeck::new();

        // we want the button to win the main pot
        deck.push(Card {
            rank: Rank::Ace,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Ace,
            suit: Suit::Diamond,
        });

        // the small blind will win the remaining
        deck.push(Card {
            rank: Rank::Six,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Six,
            suit: Suit::Heart,
        });

        // the big blind loses
        deck.push(Card {
            rank: Rank::Two,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Four,
            suit: Suit::Heart,
        });

        // UTG wins the second side pot
        deck.push(Card {
            rank: Rank::Queen,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Queen,
            suit: Suit::Heart,
        });

        // now the full run out
        deck.push(Card {
            rank: Rank::Three,
            suit: Suit::Diamond,
        });
        deck.push(Card {
            rank: Rank::Eight,
            suit: Suit::Spade,
        });
        deck.push(Card {
            rank: Rank::Nine,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::King,
            suit: Suit::Heart,
        });
        deck.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });

        let mut game = Game::default();
        game.deck = Box::new(deck);
        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        // player1 will start as the button
        let id1 = uuid::Uuid::new_v4();
        let name1 = "Button".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1, None).unwrap();
        // set the button to have less money so there is a side pot
        game.players[0].as_mut().unwrap().money = 500;

        // player2 will start as the small blind
        let id2 = uuid::Uuid::new_v4();
        let name2 = "Small".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2, None).unwrap();

        // player3 will start as the big blind
        let id3 = uuid::Uuid::new_v4();
        let name3 = "Big".to_string();
        let settings3 = PlayerConfig::new(id3, Some(name3), None);
        game.add_user(settings3, None).unwrap();

        // player4 will start as UTG
        let id4 = uuid::Uuid::new_v4();
        let name4 = "UTG".to_string();
        let settings4 = PlayerConfig::new(id4, Some(name4), None);
        game.add_user(settings4, None).unwrap();
        // set UTG to have medium money so there is a second side pot
        game.players[3].as_mut().unwrap().money = 750;

        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 4);
        assert!(game.players[0].as_ref().unwrap().human_controlled);
        assert!(game.players[1].as_ref().unwrap().human_controlled);
        assert!(game.players[2].as_ref().unwrap().human_controlled);
        assert!(game.players[3].as_ref().unwrap().human_controlled);

        let handler = std::thread::spawn(move || {
            game.play_one_hand(&cloned_actions, &cloned_meta_actions);
            game // return the game back
        });

        // UTG goes all in with the medium stack
        incoming_actions
            .lock()
            .unwrap()
            .insert(id4, PlayerAction::Bet(750));
        // the button calls (and thus goes all in with the short stack)
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Call);
        // the small blind goes all in with a full stack
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Bet(1000));
        // the big blind calls the full all-in
        incoming_actions
            .lock()
            .unwrap()
            .insert(id3, PlayerAction::Call);

        // get the game back from the thread
        let game = handler.join().unwrap();

        // the button won the side pot
        assert_eq!(game.players[0].as_ref().unwrap().money, 2000);

        // the small blind won the remainder
        assert_eq!(game.players[1].as_ref().unwrap().money, 500);

        // the big blind lost everything
        assert_eq!(game.players[2].as_ref().unwrap().money, 0);

        // UTG won the second side pot
        assert_eq!(game.players[3].as_ref().unwrap().money, 750);
    }

    /// can we pass a hand limit of 2 and the game comes to an end
    #[test]
    fn hand_limit() {
        let mut game = Game::default();
        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        // player1 will start as the button
        let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1, None).unwrap();

        // player2 will start as the small blind
        let id2 = uuid::Uuid::new_v4();
        let name2 = "Human1".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2, None).unwrap();
        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);

        let handler = std::thread::spawn(move || {
            game.play(&cloned_actions, &cloned_meta_actions, Some(2));
            game // return the game back
        });

        // set the action that player2 folds
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Fold);

        // then player1 folds next hand
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Fold);

        // get the game back from the thread
        let game = handler.join().unwrap();

        // check that the money balances out
        assert_eq!(game.players[0].as_ref().unwrap().money, 1000);
        assert_eq!(game.players[1].as_ref().unwrap().money, 1000);
    }
    /// the game should end after N hands if there are no human players in the game
    /// even if there is no hand limit or a high hand limit
    /// Note: in this test there are no players period, but the game will still count each check
    /// as a hand "plsyed", so we can check that the game ends with the proper count
    #[test]
    fn end_early() {
        let mut game = Game::default();
        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        let handler = std::thread::spawn(move || {
            // we start the game with None hand limit!
            game.play(&cloned_actions, &cloned_meta_actions, None);
            game // return the game back
        });

        // get the game back from the thread
        let game = handler.join().unwrap();

        // check that the game ended with 1 more than the limit turns "played"
        assert_eq!(game.hands_played, NON_HUMAN_HANDS_LIMIT + 1);
    }

    /// check that the button moves around properly
    /// we play 4 hands with 3 players with everyone folding whenever it gets to them,
    /// Note: we sleep several seconds in the test to let the game finish its hand in its thread,
    /// so the test is brittle to changes in wait durations within the game.
    /// If this test starts failing in the future, it is likely just a matter of tweaking the sleep
    /// durations
    #[test]
    fn button_movement() {
        let mut game = Game::default();
        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1, None).unwrap();

        let id2 = uuid::Uuid::new_v4();
        let name2 = "Human2".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2, None).unwrap();

        let id3 = uuid::Uuid::new_v4();
        let name3 = "Human3".to_string();
        let settings3 = PlayerConfig::new(id3, Some(name3), None);
        game.add_user(settings3, None).unwrap();

        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 3);
        assert!(game.players[0].as_ref().unwrap().human_controlled);

        let num_hands = 4;
        let handler = std::thread::spawn(move || {
            game.play(&cloned_actions, &cloned_meta_actions, Some(num_hands));
            game // return the game back
        });

        // id3 should not have to act as the big blind
        println!("\n\nsetting 1!");
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Fold);
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Fold);
        //incoming_actions.lock().unwrap().insert(id4, PlayerAction::Fold);

        // wait for next hand
        let wait_duration = time::Duration::from_secs(8);
        thread::sleep(wait_duration);

        println!("\n\nsetting 2!");
        // id1 should not have to act as the big blind
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Fold);
        incoming_actions
            .lock()
            .unwrap()
            .insert(id3, PlayerAction::Fold);

        // wait for next hand
        thread::sleep(wait_duration);

        println!("\n\nsetting 3!");
        // id2 should not have to act as the big blind
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Fold);
        incoming_actions
            .lock()
            .unwrap()
            .insert(id3, PlayerAction::Fold);

        // wait for next hand
        thread::sleep(wait_duration);

        // We should be back to the beginning with the button,
        // so id1 should be the button, and id3 should be the big blind
        // id3 should not have to act as the big blind
        println!("\n\nsetting 4!");
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Fold);
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Fold);
        //incoming_actions.lock().unwrap().insert(id4, PlayerAction::Fold);

        let game = handler.join().unwrap();

        // Everyone lost their small blind and won someone else's small blind
        // then in the last hand, id3 won the small blind from id2
        assert_eq!(game.players[0].as_ref().unwrap().money, 1000);
        assert_eq!(game.players[1].as_ref().unwrap().money, 996);
        assert_eq!(game.players[2].as_ref().unwrap().money, 1004);
    }

    /// the small blind calls, the big blind checks to the flop
    /// the small blind bets on the flop, and the big blind folds
    /// a player joins during the hand, and it works fine
    #[test]
    fn join_mid_hand() {
        let mut game = Game::default();
        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        // player1 will start as the button
        let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1, None).unwrap();

        // player2 will start as the small blind
        let id2 = uuid::Uuid::new_v4();
        let name2 = "Human2".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2, None).unwrap();
        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);

        let handler = std::thread::spawn(move || {
            game.play_one_hand(&cloned_actions, &cloned_meta_actions);
            game // return the game back
        });

        // set the action that player2 calls
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Call);
        // player1 checks
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Check);

        // a new player joins the game
        let id3 = uuid::Uuid::new_v4();
        let name3 = "Human3".to_string();
        let settings3 = PlayerConfig::new(id3, Some(name3), None);

        incoming_meta_actions
            .lock()
            .unwrap()
            .push_back(MetaAction::Join(settings3, None)); // no password needed

        // wait for the flop
        let wait_duration = time::Duration::from_secs(8);
        thread::sleep(wait_duration);

        // player2 bets on the flop
        println!("now sending the flop actions");
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Bet(10));
        // player1 folds
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Fold);

        // get the game back from the thread
        let game = handler.join().unwrap();

        // there is another player now
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 3);

        // check that the money changed hands
        assert_eq!(game.players[0].as_ref().unwrap().money, 992);
        assert_eq!(game.players[1].as_ref().unwrap().money, 1008);
        assert_eq!(game.players[2].as_ref().unwrap().money, 1000);
        assert!(!game.players[2].as_ref().unwrap().is_active);
    }

    /// player1 has the best hand, but chooses to sit out mid hand,
    /// This leads to a fold and player2 winning the pot
    /// It doesn't actually matter what the hands are, since it doesn't go to showdown
    #[test]
    fn sit_out() {
        let mut deck = RiggedDeck::new();

        // we want the button to have the best hand
        deck.push(Card {
            rank: Rank::Ace,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Ace,
            suit: Suit::Diamond,
        });

        // the small blind player2 wins regardless
        deck.push(Card {
            rank: Rank::Six,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Five,
            suit: Suit::Heart,
        });

        // the flop
        deck.push(Card {
            rank: Rank::Ace,
            suit: Suit::Heart,
        });
        deck.push(Card {
            rank: Rank::Ace,
            suit: Suit::Spade,
        });
        deck.push(Card {
            rank: Rank::King,
            suit: Suit::Heart,
        });

        let mut game = Game::default();
        game.deck = Box::new(deck);
        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        // player1 will start as the button
        let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1, None).unwrap();

        // player2 will start as the small blind
        let id2 = uuid::Uuid::new_v4();
        let name2 = "Human2".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2, None).unwrap();

        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);

        // both players not sitting out to start
        let not_sitting_out = game
            .players
            .iter()
            .flatten()
            .filter(|x| !x.is_sitting_out)
            .count();
        assert_eq!(not_sitting_out, 2);

        let handler = std::thread::spawn(move || {
            game.play_one_hand(&cloned_actions, &cloned_meta_actions);
            game // return the game back
        });

        // set the action that player2 calls
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Call);
        // player1 checks
        incoming_actions
            .lock()
            .unwrap()
            .insert(id1, PlayerAction::Check);

        // wait for the flop
        let wait_duration = time::Duration::from_secs(8);
        thread::sleep(wait_duration);

        // player2 bets on the flop
        println!("now sending the flop actions");
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Bet(10));

        // player1 sits out, which folds and moves on
        incoming_meta_actions
            .lock()
            .unwrap()
            .push_back(MetaAction::SitOut(id1));

        // get the game back from the thread
        let game = handler.join().unwrap();

        // one player sitting out
        let not_sitting_out = game
            .players
            .iter()
            .flatten()
            .filter(|x| !x.is_sitting_out)
            .count();
        assert_eq!(not_sitting_out, 1);

        // check that the money changed hands
        assert_eq!(game.players[0].as_ref().unwrap().money, 992);
        assert_eq!(game.players[1].as_ref().unwrap().money, 1008);
        assert!(!game.players[0].as_ref().unwrap().is_active);
    }
    /// player1 has the best hand, but chooses to leave out mid hand,
    /// This leads to a fold and player2 winning the pot
    /// It doesn't actually matter what the hands are, since it doesn't go to showdown
    #[test]
    fn leave() {
        let mut deck = RiggedDeck::new();

        // we want the button to have the best hand
        deck.push(Card {
            rank: Rank::Ace,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Ace,
            suit: Suit::Diamond,
        });

        // the small blind player2 wins regardless
        deck.push(Card {
            rank: Rank::Six,
            suit: Suit::Club,
        });
        deck.push(Card {
            rank: Rank::Five,
            suit: Suit::Heart,
        });

        // the flop
        deck.push(Card {
            rank: Rank::Ace,
            suit: Suit::Heart,
        });
        deck.push(Card {
            rank: Rank::Ace,
            suit: Suit::Spade,
        });
        deck.push(Card {
            rank: Rank::King,
            suit: Suit::Heart,
        });

        let mut game = Game::default();
        game.deck = Box::new(deck);
        let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));
        let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));
        let cloned_actions = incoming_actions.clone();
        let cloned_meta_actions = incoming_meta_actions.clone();

        // player1 will start as the button
        let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1, None).unwrap();

        // player2 will start as the small blind
        let id2 = uuid::Uuid::new_v4();
        let name2 = "Human2".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2, None).unwrap();

        // flatten to get all the Some() players
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);

        let handler = std::thread::spawn(move || {
            game.play_one_hand(&cloned_actions, &cloned_meta_actions);
            game // return the game back
        });

        // set the action that player2 calls
        incoming_actions
            .lock()
            .unwrap()
            .insert(id2, PlayerAction::Call);

        // player1 leave, which folds and ends the hand
        incoming_meta_actions
            .lock()
            .unwrap()
            .push_back(MetaAction::Leave(id1));

        // get the game back from the thread
        let game = handler.join().unwrap();

        // flatten to get all the Some() players
        // now there are only one
        let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 1);
        assert_eq!(game.player_ids_to_configs.len(), 1);

        // check that the money changed hands
        assert!(game.players[0].is_none()); // the spot is empty now
        assert_eq!(game.players[1].as_ref().unwrap().money, 1008);
    }
}
