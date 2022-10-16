use rand::Rng;
use std::cmp;
use std::collections::{HashMap, HashSet};
use std::iter;
use std::sync::{Arc, Mutex};

use super::card::{Card, Deck, HandResult};
use super::player::{Player, PlayerAction, PlayerConfig};
use crate::messages::WsMessage;

use std::{thread, time};

use uuid::Uuid;

#[derive(Debug, PartialEq, Copy, Clone)]
enum Street {
    Preflop,
    Flop,
    Turn,
    River,
    ShowDown,
}

#[derive(Debug)]
struct GameHand<'a> {
    deck: &'a mut Deck,
    num_active: usize,
    button_idx: usize, // the button index dictates where the action starts
    small_blind: f64,
    big_blind: f64,
    street: Street,
    pot: f64, // current size of the pot
    flop: Option<Vec<Card>>,
    turn: Option<Card>,
    river: Option<Card>,
}

impl<'a> GameHand<'a> {
    fn new(
        deck: &'a mut Deck,
        button_idx: usize,
        small_blind: f64,
        big_blind: f64,
    ) -> Self {
        GameHand {
            deck,
            num_active: 0,
            button_idx,
            small_blind,
            big_blind,
            street: Street::Preflop,
            pot: 0.0,
            flop: None,
            turn: None,
            river: None,
        }
    }

    fn transition(&mut self) {
        match self.street {
            Street::Preflop => {
                self.street = Street::Flop;
                self.deal_flop();
                println!(
                    "\n===========================\nFlop = {:?}\n===========================",
                    self.flop
                );
            }
            Street::Flop => {
                self.street = Street::Turn;
                self.deal_turn();
                println!(
                    "\n==========================\nTurn = {:?}\n==========================",
                    self.turn
                );
            }
            Street::Turn => {
                self.street = Street::River;
                self.deal_river();
                println!(
                    "\n==========================\nRiver = {:?}\n==========================",
                    self.river
                );
            }
            Street::River => {
                self.street = Street::ShowDown;
                println!(
                    "\n==========================\nShowDown!\n================================"
                );
            }
            Street::ShowDown => (), // we are already in the end street (from players folding during the street)
        }
    }

    fn deal_hands(&mut self, players: &mut [Option<Player>]) {
        for player in players.iter_mut().flatten() {
            if player.is_active {
                for _ in 0..2 {
                    if let Some(card) = self.deck.draw_card() {
                        player.hole_cards.push(card)
                    } else {
                        panic!();
                    }
                }
            }
        }
    }

    fn deal_flop(&mut self) {
        let mut flop = Vec::<Card>::with_capacity(3);
        for _ in 0..3 {
            if let Some(card) = self.deck.draw_card() {
                flop.push(card)
            } else {
                panic!();
            }
        }
        self.flop = Some(flop);
    }

    fn deal_turn(&mut self) {
        self.turn = self.deck.draw_card();
    }

    fn deal_river(&mut self) {
        self.river = self.deck.draw_card();
    }

    fn finish(&mut self, players: &mut [Option<Player>]) {
        let mut best_indices = HashSet::<usize>::new();
        let hand_results = players
            .iter()
            .flatten() // skip None values
            .map(|player| self.determine_best_hand(player))
            .collect::<Vec<Option<HandResult>>>();

        if let Street::ShowDown = self.street {
            // if we made it to show down, there are multiple plauers left, so we need to see who
            // has the best hand.
            println!("Multiple active players made it to showdown!");
            let mut best_idx = 0;
            best_indices.insert(best_idx);
            println!("starting best hand_result = {:?}", hand_results[best_idx]);
            // TODO: is there a chance hand_results[0] == None and we blow up?
            for (mut i, current_result) in hand_results.iter().skip(1).enumerate() {
                i += 1; // increment i to get the actual index, since we are skipping the first element at idx 0
                        //println!("Index = {}, Current result = {:?}", i, current_result);

                if current_result.is_none() {
                    //println!("no hand result at index {:?}", i);
                    continue;
                }
                if hand_results[best_idx] == None || *current_result > hand_results[best_idx] {
                    // TODO: this was working before the == None condition, but that seemed weird to me...
                    // anything > None == true in Rust perhaps? I added the check to be clear when reading the code.
                    println!("new best hand at index {:?}", i);
                    best_indices.clear();
                    best_indices.insert(i); // only one best hand now
                    best_idx = i;
                } else if *current_result == hand_results[best_idx] {
                    println!("equally good hand at index {:?}", i);
                    best_indices.insert(i); // another index that also has the best hand
                } else {
                    println!("hand worse at index {:?}", i);
                    continue;
                }
            }
        } else {
            // the hand ended before Showdown, so we simple find the one active player remaining
            for (i, player) in players.iter().flatten().enumerate() {
                // TODO: make this more functional/rusty
                if player.is_active {
                    //println!("found an active player remaining");
                    best_indices.insert(i);
                } else {
                    //println!("found an NON active player remaining");
                }
            }
            assert!(best_indices.len() == 1); // if we didn't make it to show down, there better be only one player left
        }

        // divy the pot to all the winners
        // TODO: if a player was all_in (i.e. money left is 0?) then we need to figure out
        // how much of the pot they actually get to win if multiple other players made it to showdown
        let num_winners = best_indices.len();
        let payout = self.pot as f64 / num_winners as f64;

        for idx in best_indices {
            let winning_spot = &mut players[idx];
	    if let Some(ref mut winning_player) = winning_spot {
		//let b: bool = winning_player;
		println!(
                    "paying out: {:?} \n  with hand result = {:?}",
                    winning_player, hand_results[idx]
		);
		winning_player.pay(payout);
		println!("after payment: {:?}", winning_player);
	    }
        }

        // take the players' cards
        for player in players.iter_mut().flatten() {
            // todo: is there any issue with calling drain if they dont have any cards?
            player.hole_cards.drain(..);
            if !player.is_sitting_out {
                if player.money == 0.0 {
                    println!(
                        "Player {:?} is out of money so is no longer playing in the game!",
                        player.id
                    );
                    player.is_active = false;
                    player.is_sitting_out = true;
                } else {
                    // they will be active in the next hand
                    player.is_active = true;
                }
            }
        }
    }

    /// Given a player, we need to determine which 5 cards make the best hand for this player
    fn determine_best_hand(&self, player: &Player) -> Option<HandResult> {
        if !player.is_active {
            // if the player isn't active, then can't have a best hand
            return None;
        }

        if let Street::ShowDown = self.street {
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
                        .chain(self.flop.as_ref().unwrap().iter())
                        .chain(iter::once(&self.turn.unwrap()))
                        .chain(iter::once(&self.river.unwrap()))
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
                                       //println!("Looked at {} possible hands", hand_count);
            println!("player = {:?}", player.id);
            println!("best result = {:?}", best_result);
            best_result
        } else {
            None
        }
    }

    fn play(&mut self,
	    players: &mut [Option<Player>],
	    player_ids_to_configs: &HashMap<Uuid, PlayerConfig>,
	    incoming_actions: &Arc<Mutex<HashMap<Uuid, PlayerAction>>>,
    ) {
        println!("inside of play(). button_idx = {:?}", self.button_idx);
	// how many active to start the hand
	// note: we flatten so that we don't consider None values (empty seats)
        self.num_active = players.iter().flatten().filter(|player| player.is_active).count(); 
        PlayerConfig::send_group_message(&format!(
            "inside of play(). button_idx = {:?}",
            self.button_idx
        ), player_ids_to_configs);
        if self.num_active < 2 {
            println!(
                "num_active players = {}, so we cannot play a hand!",
                self.num_active
            );
            return;
        }
        self.deck.shuffle();
        self.deal_hands(players);

        println!("players = {:?}", players);
        PlayerConfig::send_group_message(&format!("players = {:?}", players), player_ids_to_configs);
        while self.street != Street::ShowDown {
            self.play_street(players, player_ids_to_configs, &incoming_actions);
            if self.num_active == 1 {
                // if the game is over from players folding
                println!("\nGame is ending before showdown!");
                PlayerConfig::send_group_message("\nGame is ending before showdown!", player_ids_to_configs);
                break;
            } else {
                // otherwise we move to the next street
                self.transition();
            }
        }
        // now we finish up and pay the pot to the winner
        self.finish(players);
    }

    fn get_starting_idx(&self, players: &mut [Option<Player>]) -> usize {
        // the starting index is either the person one more from the button on most streets,
        // or 3 down on the preflop (since the blinds already had to buy in)
        // TODO: this needs to be smarter in small games
        let mut starting_idx = self.button_idx + 1;
        if starting_idx as usize >= players.len() {
            starting_idx = 0;
        }
        starting_idx
    }

    fn play_street(
	&mut self,
	players: &mut [Option<Player>],
	player_ids_to_configs: &HashMap<Uuid, PlayerConfig>,
	incoming_actions: &Arc<Mutex<HashMap<Uuid, PlayerAction>>>,
    ) {
        let mut current_bet: f64 = 0.0;
        // each index keeps track of that players' contribution this street
        let mut cumulative_bets = vec![0.0; players.len()];

        let starting_idx = self.get_starting_idx(players); // which player starts the betting
                                                    // keeps track of how many players have either checked through or called
                                                    // the last bet (or made the last bet)

        // if a player is still active but has no remaining money (i.e. is all-in),
        // then they are settled and ready to go to the end
        let mut num_all_in = players
            .iter()
            .flatten() // skip over None values
            .filter(|player| player.is_all_in())
            .count();
        let mut num_settled = num_all_in;

        println!("Current pot = {}", self.pot);
        PlayerConfig::send_group_message(&format!("Current pot = {}", self.pot), player_ids_to_configs);

        println!("num active players = {}", self.num_active);
        PlayerConfig::send_group_message(&format!("num active players = {}", self.num_active), player_ids_to_configs);

        println!("player at index {} starts the betting", starting_idx);
        PlayerConfig::send_group_message(&format!(
            "player at index {} starts the betting",
            starting_idx
        ), player_ids_to_configs);
        if num_settled > 0 {
            println!("num settled (i.e. all in players) = {}", num_settled);
            PlayerConfig::send_group_message(&format!(
                "num settled (i.e. all in players) = {}",
                num_settled
            ), player_ids_to_configs);
        }
        let mut loop_count = 0;
        'street: loop {
            loop_count += 1;
            if loop_count > 5 {
                println!("\n\n\n\n\nTOO MANY LOOPS MUST BE A BUG!\n\n\n\n\n");
                panic!("too many loops");
            }
            // iterate over the players from the starting index to the end of the vec,
            // and then from the beginning back to the starting index
            let (left, right) = players.split_at_mut(starting_idx);
            for (i, mut player) in right.iter_mut().chain(left.iter_mut()).flatten().enumerate() {
                let player_cumulative = cumulative_bets[i];
                println!("Current pot = {:?}, Current size of the bet = {:?}, and this player has put in {:?} so far",
			 self.pot,
			 current_bet,
			 player_cumulative);

                println!("Player = {:?}, i = {}", player.id, i);
                if player.is_active && player.money > 0.0 {
                    let action = self.get_and_validate_action(
                        player,
                        current_bet,
                        player_cumulative,
			player_ids_to_configs,
			&incoming_actions,
                    );

                    match action {
                        PlayerAction::PostSmallBlind(amount) => {
                            println!("Player posts small blind of {}", amount);
                            self.pot += amount;
                            cumulative_bets[i] += amount;
                            player.money -= amount;
                            // regardless if the player couldn't afford it, the new street bet is the big blind
                            current_bet = self.small_blind;
                            if player.is_all_in() {
                                num_all_in += 1;
                                num_settled = num_all_in;
                            } else {
                                num_settled = num_all_in + 1;
                            }
                        }
                        PlayerAction::PostBigBlind(amount) => {
                            println!("Player posts big blind of {}", amount);
                            self.pot += amount;
                            cumulative_bets[i] += amount;
                            player.money -= amount;
                            // regardless if the player couldn't afford it, the new street bet is the big blind
                            current_bet = self.big_blind;
                            if player.is_all_in() {
                                num_all_in += 1;
                                num_settled = num_all_in;
                            } else {
                                num_settled = num_all_in + 1;
                            }
                        }
                        PlayerAction::Fold => {
                            println!("Player {:?} folds!", player.id);
                            player.deactivate();
                            self.num_active -= 1;
                        }
                        PlayerAction::Check => {
                            println!("Player checks!");
                            num_settled += 1;
                        }
                        PlayerAction::Call => {
                            println!("Player calls!");
                            let difference = current_bet - player_cumulative;
                            if difference > player.money {
                                println!("you have to put in the rest of your chips");
                                self.pot += player.money;
                                cumulative_bets[i] += player.money;
                                player.money = 0.0;
                                num_all_in += 1;
                            } else {
                                self.pot += difference;
                                cumulative_bets[i] += difference;
                                player.money -= difference;
                            }
                            num_settled += 1;
                        }
                        PlayerAction::Bet(new_bet) => {
                            println!("Player bets {}!", new_bet);
                            let difference = new_bet - player_cumulative;
                            self.pot += difference;
                            player.money -= difference;
                            current_bet = new_bet;
                            cumulative_bets[i] = new_bet;
                            if player.is_all_in() {
                                println!("Just bet the rest of our money!");
                                num_all_in += 1;
                                num_settled = num_all_in;
                            } else {
                                // since we just bet more, we are the only settled player (aside from the all-ins)
                                num_settled = num_all_in + 1;
                            }
                        }
                    }
                }

                //println!("after player: num_active = {}, num_settled = {}", self.num_active, num_settled);
                if self.num_active == 1 {
                    println!("Only one active player left so lets break the steet loop");
                    break 'street;
                }
                if num_settled == self.num_active {
                    // every active player is ready to move onto the next street
                    println!(
                        "everyone is ready to go to the next street! num_settled = {}",
                        num_settled
                    );
                    break 'street;
                }
            }
        }
    }

    /// if the player is a human, then we look for their action in the incoming_actions hashmap
    /// this value is set by the game hub when handling a message from a player client
    fn get_action_from_player(
	player: &mut Player,
	incoming_actions: &Arc<Mutex<HashMap<Uuid, PlayerAction>>>)
	-> Option<PlayerAction>
    {
        if player.human_controlled {
	    let mut actions = incoming_actions.lock().unwrap();	    
	    println!("incoming_actions = {:?}", actions);	    
            if let Some(action) = actions.get_mut(&player.id) {
                println!(
                    "Player: {:?} has action {:?}",
                    player.id, action
                );
		let value = *action;
		actions.remove(&player.id);  // wipe this action so we don't repeat it next time
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
                    let amount: f64 = if player.money <= 100.0 {
                        // just go all in if we are at 10% starting
                        player.money as f64
                    } else {
                        rand::thread_rng().gen_range(1..player.money as u32) as f64
                    };
                    Some(PlayerAction::Bet(amount))
                }
                _ => Some(PlayerAction::Call),
            }
        }
    }

    fn get_and_validate_action(
	&self, 
        player: &mut Player,
        current_bet: f64,
        player_cumulative: f64,
	player_ids_to_configs: &HashMap<Uuid, PlayerConfig>,
	incoming_actions: &Arc<Mutex<HashMap<Uuid, PlayerAction>>>,
    ) -> PlayerAction {
        // if it isnt valid based on the current bet and the amount the player has already contributed,
        // then it loops
        // position is our spot in the order, with 0 == small blind, etc
        if self.street == Street::Preflop && current_bet == 0.0 {
            // collect small blind!
            return PlayerAction::PostSmallBlind(cmp::min(
                self.small_blind as u32,
                player.money as u32,
            ) as f64);
        } else if self.street == Street::Preflop && current_bet == self.small_blind {
            // collect big blind!
            return PlayerAction::PostBigBlind(
                cmp::min(self.big_blind as u32, player.money as u32) as f64,
            );
        }
	PlayerConfig::send_specific_message(
	    &"Please enter your action!".to_owned(),
	    player.id,
	    player_ids_to_configs
	);
	
        let mut action = None;
        let mut attempts = 0;
        let retry_duration = time::Duration::from_secs(3); // how long to wait between trying again
        while attempts < 8 && action.is_none() {
            // not a blind, so get an actual choice
            if player.human_controlled {
                // we don't need to count the attempts at getting a response from a computer
                // TODO: the computer can give a better than random guess at a move
                // Currently it might try to check when it has to call for example,
                attempts += 1;
            }
            //println!("Attempting to get player action on attempt {:?}", attempts);
            match GameHand::get_action_from_player(player, incoming_actions) {
		None => {
                    println!("No action is set for the player {:?}", player.id);
		    // TODO: send a message to the player
                    // we give the user a second to place their action
                    thread::sleep(retry_duration);
		}
		
                Some(PlayerAction::Fold) => {
                    if current_bet <= player_cumulative {
                        // if the player has put in enough then no sense folding
                        if player.human_controlled {
                            println!("you said fold but we will let you check!");
                        }
                        action = Some(PlayerAction::Check);
                    } else {
                        action = Some(PlayerAction::Fold);
                    }
                }
                Some(PlayerAction::Check) => {
                    //println!("Player checks!");
                    if current_bet > player_cumulative {
                        // if the current bet is higher than this players bet
                        if player.human_controlled {
			    PlayerConfig::send_specific_message(
				&"You can't check since there is a bet!!".to_owned(),
				player.id,
				player_ids_to_configs
			    );
                        }
                        continue;
                    }
                    action = Some(PlayerAction::Check);
                }
                Some(PlayerAction::Call) => {
                    if current_bet <= player_cumulative {
                        if current_bet != 0.0 {
                            // if the street bet isn't 0 then this makes no sense
                            println!("should we even be here???!");
                        }
			// we can let them check
                        action = Some(PlayerAction::Check);			
                    }
                    action = Some(PlayerAction::Call);
                }
                Some(PlayerAction::Bet(new_bet)) => {
                    //println!("Player bets {}!", new_bet);
                    if current_bet < player_cumulative {
                        // will this case happen?
                        println!("this should not happen!");
                        continue;
                    }
                    if new_bet - player_cumulative > player.money {
			PlayerConfig::send_specific_message(
			    &"You can't bet more than you have!!".to_owned(),
			    player.id,
			    player_ids_to_configs
			    );
                        continue;
                    }
                    if new_bet <= current_bet {
                        //println!("the new bet has to be larger than the current bet!");
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
        if let Some(final_action) = action {
            final_action
        } else {
            PlayerAction::Fold
        }
    }
}

#[derive(Debug)]
pub struct Game {
    deck: Deck,
    players: [Option<Player>; 9], // 9 spots where players can sit
    player_ids_to_configs: HashMap<Uuid, PlayerConfig>,
    button_idx: usize, // index of the player with the button
    small_blind: f64,
    big_blind: f64,
}

impl Game {
    pub fn new() -> Self {
        Game {
            deck: Deck::new(),
            players: Default::default(), 
	    player_ids_to_configs: HashMap::<Uuid, PlayerConfig>::new(),
            small_blind: 4.0,
            big_blind: 8.0,
            button_idx: 0,
        }
    }

    /// add a given playerconfig to an empty seat
    /// TODO: eventually we wanmt the player to select an open seat I guess
    pub fn add_user(&mut self, player_config: PlayerConfig) {
	self.add_player(player_config, true);
    }

    pub fn add_bot(&mut self, name: String) {
	let new_bot = Player::new_bot();
	let new_config = PlayerConfig::new(new_bot.id, Some(name), None);
	self.add_player(new_config, false);
    }
    
    fn add_player(&mut self, player_config: PlayerConfig, human_controlled: bool) {
	let mut added = false;
	for player_spot in self.players.iter_mut() {
	    if player_spot.is_none() {
		*player_spot = Some(Player::new(player_config.id, human_controlled));
		self.player_ids_to_configs.insert(player_config.id, player_config);
		added = true;
		break;
	    }
	}
	if added {
	    // TODO: send a message to the player and the table
	} else {
	    // we were unable to add a player
	    // or should we just keep count of the number of players at any moment?
	}

    }
    
    /// remove the player from the vec of players with the given id
    /// and remove it from the id to config mapping
    pub fn remove_player(&mut self, id: Uuid) {
	for player_spot in self.players.iter_mut() {
	    if let Some(player) = player_spot{
		if player.id == id {
		    *player_spot = None;
		    self.player_ids_to_configs.remove(&id);		    
		}
	    }
	}
    }

    /// find a player with the given id, and set their name to be the given name
    pub fn set_player_name(&mut self, id: Uuid, name: &str) {
	if let Some(player_config) = self.player_ids_to_configs.get_mut(&id) {
            player_config.name = Some(name.to_string());	    
	}
    }
    
    /// find a player with the given id, and set their action to be the given Playeraction
    //TODO: i think soon i am gouing to need to worry about a mutex or something?
    //maybe the action shoouldnt live on each player, but instead in a HashMap that the hub can access?
    //Because atm the Handle<PlayerAction> only happens AFTER game.play() ends
    pub fn set_player_action(&mut self, id: Uuid, action: PlayerAction) {
	println!("inside set player action: {:?}, {:?}", id, action);
	for player in self.players.iter_mut().flatten() {
	    if player.id == id {
		player.current_action = Some(action);
		break;
	    }
	}
    }
    
    /// send a given message to all the players at the tabel
    pub fn send_message(&self, message: &str) {
        PlayerConfig::send_group_message(message, &self.player_ids_to_configs);
    }
    
    pub fn play_one_hand(
	&mut self,
	incoming_actions: &Arc<Mutex<HashMap<Uuid, PlayerAction>>>,		     
    ) {
        let mut game_hand = GameHand::new(
            &mut self.deck,
            self.button_idx,
            self.small_blind,
            self.big_blind,
        );
        game_hand.play(&mut self.players, &self.player_ids_to_configs, incoming_actions);
    }

    pub fn play(&mut self, incoming_actions: &Arc<Mutex<HashMap<Uuid, PlayerAction>>>) {
        let mut hand_count = 0;
        loop {
            hand_count += 1;
            println!(
                "\n\n\n=================================================\n\nplaying hand {}",
                hand_count
            );
	    println!("self.incoming_actions = {:?}", incoming_actions.lock().unwrap());
            self.play_one_hand(incoming_actions);
            // TODO: do we need to add or remove any players?


	    if hand_count > 1 {
		break; // TODO: remove this break eventually
	    }
            let mut loop_count = 0;
            'find_button: loop {
                loop_count += 1;
                if loop_count >= 5 {
                    // couldn't find a valid button position. how does this happen?
                    break 'find_button;
                }
                self.button_idx += 1; // and modulo length
                if self.button_idx as usize >= self.players.len() {
                    self.button_idx = 0;
                }
		let button_spot = &mut self.players[self.button_idx];
		if let Some(button_player) = button_spot {
                    if button_player.is_sitting_out {
			println!(
                            "Player at index {} is sitting out so cannot be the button",
                            self.button_idx
			);
			continue;
                    } else {
			// We found a player who is not sitting out, so it is a valid
			// button position
			break 'find_button;		    
		    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PlayerConfig;
    use super::*;

    #[test]
    fn add_bot() {
        let mut game = Game::new();
        let name = "Mr Bot".to_string();
        game.add_bot(name);
        assert_eq!(game.players.len(), 1);
        assert!(!game.players[0].human_controlled);
    }

    #[test]
    fn add_user_no_connection() {
        let mut game = Game::new();
        let id = uuid::Uuid::new_v4();
        let name = "Human".to_string();
        let settings = PlayerConfig::new(id, Some(name), None);
        game.add_user(settings);
        assert_eq!(game.players.len(), 1);
        assert!(game.players[0].human_controlled);
    }

    #[test]
    fn remove_player() {
        let mut game = Game::new();
        let id = uuid::Uuid::new_v4();
        let name = "Human".to_string();
        let settings = PlayerConfig::new(id, Some(name), None);
        game.add_user(settings);
        assert_eq!(game.players.len(), 1);
        game.remove_player(id);
        assert!(game.players.is_empty());
    }
}


