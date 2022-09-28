use rand::Rng;
use std::cmp;
use std::collections::HashSet;
use std::io;
use std::iter;

use super::card::{Card, Deck, HandResult};

#[derive(Debug)]
enum PlayerAction {
    PostSmallBlind(f64),
    PostBigBlind(f64),
    Fold,
    Check,
    Bet(f64),
    Call,
    //Raise(u32), // i guess a raise is just a bet really?
}

#[derive(Debug)]
struct Player {
    name: String,
    hole_cards: Vec<Card>,
    is_active: bool,      // is still playing the current hand
    is_sitting_out: bool, // if sitting out, then they are not active for any future hand
    money: f64,
    human_controlled: bool, // do we need user input or let the computer control it
}

impl Player {
    fn new(name: String, human_controlled: bool) -> Self {
        Player {
            name,
            hole_cards: Vec::<Card>::with_capacity(2),
            is_active: true,
            is_sitting_out: false,
            money: 1000.0, // let them start with 1000 for now,
            human_controlled,
        }
    }

    fn pay(&mut self, payment: f64) {
        self.money += payment;
    }

    fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// If the player has put all their money in, but has not folded (is_active),
    /// then they are all-in
    fn is_all_in(&self) -> bool {
        self.is_active && self.money == 0.0
    }
}

#[derive(Debug, PartialEq)]
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
    players: &'a mut Vec<Player>,
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
        players: &'a mut Vec<Player>,
        button_idx: usize,
        small_blind: f64,
        big_blind: f64,
    ) -> Self {
        let num_active = players.iter().filter(|player| player.is_active).count(); // active to start the hand
        GameHand {
            deck,
            players,
            num_active,
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
                println!("\n========================================\nFlop = {:?}\n========================================", self.flop);
            }
            Street::Flop => {
                self.street = Street::Turn;
                self.deal_turn();
                println!("\n========================================\nTurn = {:?}\n========================================", self.turn);
            }
            Street::Turn => {
                self.street = Street::River;
                self.deal_river();
                println!("\n========================================\nRiver = {:?}\n========================================", self.river);
            }
            Street::River => {
                self.street = Street::ShowDown;
                println!("\n========================================\nShowDown!\n========================================");
            }
            Street::ShowDown => (), // we are already in the end street (from players folding during the street)
        }
    }

    fn deal_hands(&mut self) {
        for player in self.players.iter_mut() {
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

    fn finish(&mut self) {
        let mut best_indices = HashSet::<usize>::new();
        let hand_results = self
            .players
            .iter()
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
            for (i, player) in self.players.iter().enumerate() {
                // TODO: make this more functional/rusty
                if player.is_active {
                    //println!("found an active player remaining");
                    best_indices.insert(i);
                } else {
                    ();
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

        for idx in best_indices.iter() {
            let winning_player = &mut self.players[*idx];
            println!(
                "paying out: {:?} \n  with hand result = {:?}",
                winning_player, hand_results[*idx]
            );
            winning_player.pay(payout);
            println!("after payment: {:?}", winning_player);
        }

        // take the players' cards
        for player in self.players.iter_mut() {
            // todo: is there any issue with calling drain if they dont have any cards?
            player.hole_cards.drain(..);
            if !player.is_sitting_out {
                if player.money == 0.0 {
                    println!(
                        "Player {} is out of money so is no longer playing in the game!",
                        player.name
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
            println!("player = {}", player.name);
            println!("best result = {:?}", best_result);
            best_result
        } else {
            None
        }
    }

    fn play(&mut self) {
        println!("inside of play(). button_idx = {}", self.button_idx);
        if self.num_active < 2 {
            println!(
                "num_active players = {}, so we cannot play a hand!",
                self.num_active
            );
            return;
        }
        self.deck.shuffle();
        //for card in self.deck.cards.iter() {
        //   println!("{:?}", card);
        //}
        self.deal_hands();

        println!("self.players = {:?}", self.players);
        while self.street != Street::ShowDown {
            //println!("\nStreet is {:?}", self.street);
            self.play_street();
            if self.num_active == 1 {
                // if the game is over from players folding
                println!("\nGame is ending before showdown!");
                break;
            } else {
                // otherwise we move to the next street
                self.transition();
            }
        }
        // now we finish up and pay the pot to the winner
        self.finish();
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

    fn play_street(&mut self) {
        let mut street_bet: f64 = 0.0;
	// each index keeps track of that players' contribution this street	
        let mut cumulative_bets = vec![0.0; self.players.len()]; 

        let starting_idx = self.get_starting_idx(); // which player starts the betting
        //let mut num_settled = 0;
	// keeps track of how many players have either checked through or called the last bet (or made the last bet)

        // if a player is still active but has no remaining money (i.e. is all-in),
	// then they are settled and ready to go to the end
        let mut num_all_in = self
            .players
            .iter()
            .filter(|player| player.is_all_in())
            .count();
        let mut num_settled = num_all_in;

        println!("Current pot = {}", self.pot);
        println!("num active players = {}", self.num_active);
        println!("player at index {} starts the betting", starting_idx);
        if num_settled > 0 {
            println!("num settled (i.e. all in players) = {}", num_settled);
        }
        let mut loop_count = 0;
        'street: loop {
            loop_count += 1;
            if loop_count > 5 {
                println!("\n\n\n\n\nTOO MANY LOOPS MUST BE A BUG!\n\n\n\n\n");
                panic!("too many loops");
            }
            // iterate over the players from the starting index to the end of the vec, and then from the beginning back to the starting index
            let (left, right) = self.players.split_at_mut(starting_idx);
            for (i, mut player) in right.iter_mut().chain(left.iter_mut()).enumerate() {
                let player_cumulative = cumulative_bets[i];
                println!("Current pot = {:?}, Current size of the bet = {:?}, and this player has put in {:?} so far",
			 self.pot,
			 street_bet,
			 player_cumulative);
                println!("Player = {:?}, i = {}", player.name, i);
                if player.is_active && player.money > 0.0 {
                    let action;
                    if self.street == Street::Preflop && street_bet == 0.0 {
                        // collect small blind!
                        action = PlayerAction::PostSmallBlind(cmp::min(
                            self.small_blind as u32,
                            player.money as u32,
                        ) as f64);
                    } else if self.street == Street::Preflop && street_bet == self.small_blind {
                        // collect big blind!
                        action = PlayerAction::PostBigBlind(cmp::min(
                            self.big_blind as u32,
                            player.money as u32,
                        ) as f64);
                    } else {
                        action = GameHand::get_and_validate_action(
                            player,
                            street_bet,
                            player_cumulative,
                        );
                    }

                    match action {
                        PlayerAction::PostSmallBlind(amount) => {
                            println!("Player posts small blind of {}", amount);
                            self.pot += amount;
                            cumulative_bets[i] += amount;
                            player.money -= amount;
                            // regardless if the player couldn't afford it, the new street bet is the big blind
                            street_bet = self.small_blind;
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
                            street_bet = self.big_blind;
                            if player.is_all_in() {
                                num_all_in += 1;
                                num_settled = num_all_in;
                            } else {
                                num_settled = num_all_in + 1;
                            }
                        }
                        PlayerAction::Fold => {
                            println!("Player {:?} folds!", player.name);
                            player.deactivate();
                            self.num_active -= 1;
                        }
                        PlayerAction::Check => {
                            println!("Player checks!");
                            num_settled += 1;
                        }
                        PlayerAction::Call => {
                            println!("Player calls!");
                            let difference = street_bet - player_cumulative;
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
                            street_bet = new_bet;
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

    fn get_action_from_user(player: &Player) -> PlayerAction {
        // will need UI here
        // for now do a random action
        if player.human_controlled {
            println!("You: {:?}", player);
            println!("Please enter your action (f, ca, ch, b): ");
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to get console input");
            input = input.to_string().trim().to_string();
            match input.as_str() {
                "f" => PlayerAction::Fold,
                "ch" => PlayerAction::Check,
                "ca" => PlayerAction::Call,
                "b" => {
                    println!("How much to bet: ");
                    let mut amount = String::new();
                    io::stdin()
                        .read_line(&mut amount)
                        .expect("Failed to get console input");
                    amount = amount.to_string().trim().to_string();
                    println!("about to parse [{}] as a f64", amount);
                    let mut bet_amount = amount.parse::<f64>().unwrap();
                    if bet_amount > player.money {
                        println!("You are trying to bet more than you have. Lets just go all in!");
                        bet_amount = player.money;
                    }
                    PlayerAction::Bet(bet_amount)
                }
                _ => {
                    println!("Unknown input. We will attempt to check");
                    PlayerAction::Check
                }
            }
        } else {
            let num = rand::thread_rng().gen_range(0..100);
            match num {
                0..=20 => PlayerAction::Fold,
                21..=55 => PlayerAction::Check,
                56..=70 => {
                    let amount: f64 = if player.money <= 100.0 {
                        // just go all in if we are at 10% starting
                        player.money as f64
                    } else {
                        rand::thread_rng().gen_range(1..player.money as u32) as f64
                    };
                    PlayerAction::Bet(amount)
                }
                _ => PlayerAction::Call,
            }
        }
    }

    fn get_and_validate_action(
        player: &Player,
        street_bet: f64,
        player_cumulative: f64,
    ) -> PlayerAction {
        // if it isnt valid based on the current bet and the amount the player has already contributed, then it loops
        // position is our spot in the order, with 0 == small blind, etc
        let mut action;

        'valid_check: loop {
            // not a blind, so get an actual choice
            action = GameHand::get_action_from_user(player);
            match action {
                PlayerAction::Fold => {
                    if street_bet <= player_cumulative {
                        // if the player has put in enough then no sense folding
                        if player.human_controlled {
                            println!("you said fold but we will let you check!");
                        }
                        action = PlayerAction::Check;
                    }
                    break 'valid_check;
                }
                PlayerAction::Check => {
                    //println!("Player checks!");
                    if street_bet > player_cumulative {
                        // if the current bet is higher than this players bet
                        if player.human_controlled {
                            println!("you cant check since there is a bet!");
                        }
                        continue;
                    }
                    break 'valid_check;
                }
                PlayerAction::Call => {
                    if street_bet <= player_cumulative {
                        if street_bet != 0.0 {
                            // if the street bet isn't 0 then this makes no sense
                            println!("should we even be here???!");
                        }
                        continue;
                    }
                    break 'valid_check;
                }
                PlayerAction::Bet(new_bet) => {
                    //println!("Player bets {}!", new_bet);
                    if street_bet < player_cumulative {
                        // will this case happen?
                        println!("this should not happen!");
                        continue;
                    }
                    if new_bet - player_cumulative > player.money {
                        //println!("you cannot bet more than you have!");
                        continue;
                    }
                    if new_bet <= street_bet {
                        //println!("the new bet has to be larger than the current bet!");
                        continue;
                    }
                    break 'valid_check;
                }
                _ => println!("Action {:?} is not valid at this time!", action),
            }
        }
        action
    }
}

#[derive(Debug)]
pub struct Game {
    deck: Deck,
    players: Vec<Player>,
    button_idx: usize, // index of the player with the button
    small_blind: f64,
    big_blind: f64,
}

impl Game {
    pub fn new() -> Self {
        Game {
            deck: Deck::new(),
            players: Vec::<Player>::with_capacity(9),
            small_blind: 4.0,
            big_blind: 8.0,
            button_idx: 0,
        }
    }

    pub fn add_user(&mut self, name: String) {
        self.players.push(Player::new(name, true))
    }

    pub fn add_bot(&mut self, name: String) {
        self.players.push(Player::new(name, false))
    }

    fn play_one_hand(&mut self) {
        let mut game_hand = GameHand::new(
            &mut self.deck,
            &mut self.players,
            self.button_idx,
            self.small_blind,
            self.big_blind,
        );
        game_hand.play();
    }

    pub fn play(&mut self) {
        let mut hand_count = 0;
        loop {
            hand_count += 1;
            println!(
                "\n\n\n=================================================\n\nplaying hand {}",
                hand_count
            );

            self.play_one_hand();
            // TODO: do we need to add or remove any players?

            println!("\nContinue playing? (y/n): ");
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to get console input");
            input = input.to_string().trim().to_string();
            match input.as_str() {
                "y" => (),
                "n" => std::process::exit(0),
                _ => {
                    println!("Unknown response. We will take that as a yes");
                }
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
                if self.players[self.button_idx].is_sitting_out {
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
