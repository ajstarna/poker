use rand::Rng;
use std::cmp;
use std::collections::{HashMap, HashSet, VecDeque};
use std::iter;
use std::sync::{Arc, Mutex};
use actix::Addr;

use super::card::{Card, Deck, StandardDeck, HandResult};
use super::player::{Player, PlayerAction, PlayerConfig};
use crate::messages::{MetaAction, Removed};
use crate::hub::GameHub;

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

/// A pot keeps track of the total money, and which player (indices) contributed
/// A game hand can have multiple pots, when players go all-in, and bettingcontinues
#[derive(Debug)]
struct Pot {
    money: u32, // total amount in this pot
    eligible_players: HashSet<usize>, // which players have contributed to it
    cap: u32, // the most that any one player can put in. money should be eligible_players.len() * cap?
    Not sure if this is gunna work? if one guy goes all in for 1000, then someone bets 2000, then someone calls all-in
	at 500, this pot needs to be capped at 500. Does (1000-500) and (2000 - 500) need to be moved out or something?
	Do we need these pots afterall? Or does it simply come down to the total_contributions for each player by the end? hmmmmm
}

impl Pot {
    fn new() -> Self {
	Self {
	    money: 0,
	    eligible_players: HashSet::new(),
	}
    }
}

#[derive(Debug)]
struct GameHand {
    button_idx: usize, // the button index dictates where the action starts
    small_blind: u32,
    big_blind: u32,
    street: Street,
    pots: Vec<Pot>,
    total_contributions: [u32; 9], // keep track of how many a given player contributed to the pot during the whole hand
    flop: Option<Vec<Card>>,
    turn: Option<Card>,
    river: Option<Card>,
}

impl GameHand {
    fn new(
        button_idx: usize,
        small_blind: u32,
        big_blind: u32,
    ) -> Self {
        GameHand {
            button_idx,
            small_blind,
            big_blind,
            street: Street::Preflop,
            pots: vec![Pot::new()],
	    total_contributions: [0; 9],
            flop: None,
            turn: None,
            river: None,
        }
    }

    fn transition(&mut self, deck: &mut Box<dyn Deck>,  player_ids_to_configs: &HashMap<Uuid, PlayerConfig>) {
        match self.street {
            Street::Preflop => {
                self.street = Street::Flop;
                self.deal_flop(deck);
                println!(
                    "\n===========================\nFlop = {:?}\n===========================",
                    self.flop
                );
                PlayerConfig::send_group_message(
                    &format!("Flop: {}{}{}",
			     self.flop.as_ref().unwrap()[0],
			     self.flop.as_ref().unwrap()[1],
			     self.flop.as_ref().unwrap()[2]),
		    player_ids_to_configs);		
            }
            Street::Flop => {
                self.street = Street::Turn;
                self.deal_turn(deck);
                println!(
                    "\n==========================\nTurn = {:?}\n==========================",
                    self.turn
                );
                PlayerConfig::send_group_message(
                    &format!("Turn: {}", self.turn.unwrap()),
		    player_ids_to_configs);		
            }
            Street::Turn => {
                self.street = Street::River;
                self.deal_river(deck);
                println!(
                    "\n==========================\nRiver = {:?}\n==========================",
                    self.river
                );
                PlayerConfig::send_group_message(
                    &format!("River: {}", self.river.unwrap()),
		    player_ids_to_configs);		
            }
            Street::River => {
                self.street = Street::ShowDown;
                println!(
                    "\n==========================\nShowDown!\n================================"
                );
                PlayerConfig::send_group_message(
                    &format!("\n===========================\nShowDown!\n==========================="),
		    player_ids_to_configs);						
            }
            Street::ShowDown => (), // we are already in the end street (from players folding during the street)
        }
    }

    fn deal_hands(&mut self, deck: &mut Box<dyn Deck>,
		  players: &mut [Option<Player>], player_ids_to_configs: &HashMap<Uuid, PlayerConfig>) {
        for player in players.iter_mut().flatten() {
            if player.is_active {
                for _ in 0..2 {
                    if let Some(card) = deck.draw_card() {
                        player.hole_cards.push(card)
                    } else {
                        panic!();
                    }		    
                }
		PlayerConfig::send_specific_message(
		    &format!("Hole Cards: {}{}", player.hole_cards[0], player.hole_cards[1]),
		    player.id,
		    player_ids_to_configs
		);
		PlayerConfig::send_specific_message(
		    &format!("Money: {}", player.money),
		    player.id,
		    player_ids_to_configs
		);		
            }
        }
    }

    fn deal_flop(&mut self, deck: &mut Box<dyn Deck>) {
        let mut flop = Vec::<Card>::with_capacity(3);
        for _ in 0..3 {
            if let Some(card) = deck.draw_card() {
                flop.push(card)
            } else {
                panic!();
            }
        }
        self.flop = Some(flop);
    }

    fn deal_turn(&mut self, deck: &mut Box<dyn Deck>) {
        self.turn = deck.draw_card();
    }

    fn deal_river(&mut self, deck: &mut Box<dyn Deck>) {
        self.river = deck.draw_card();
    }

    fn finish(&mut self, players: &mut [Option<Player>],
	      player_ids_to_configs: &HashMap<Uuid, PlayerConfig>,
    ) {
        let mut hand_results = players
            .iter()
            .map(|player| self.determine_best_hand(player.as_ref()))
            .collect::<Vec<Option<HandResult>>>();

	println!("hand results = {:?}", hand_results);
        if let Street::ShowDown = self.street {
            // if we made it to show down, there are multiple plauers left, so we need to see who
            // has the best hand.
            println!("Multiple active players made it to showdown!");

	    while self.pots.last().unwrap().money > 0 {
		let mut best_indices = HashSet::<usize>::new();		
		let mut best_idx = 0;		
		best_indices.insert(best_idx);
		println!("starting best hand_result = {:?}", hand_results[best_idx]);
			for (mut i, current_result) in hand_results.iter().skip(1).enumerate() {
                    i += 1; // increment i to get the actual index, since we are skipping the first element at idx 0
                    //println!("Index = {}, Current result = {:?}", i, current_result);
		    
                    if current_result.is_none() {
			//println!("no hand result at index {:?}", i);
			continue;
                    }
                    if hand_results[best_idx] == None || *current_result > hand_results[best_idx] {
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
		// divy the pot to all the winners
		let num_winners = best_indices.len();
		let payout = (self.pots.last().unwrap().money as f64 / num_winners as f64) as u32;
		self.pots.last_mut().unwrap().money = 0;		
		self.pay_players(players, player_ids_to_configs, best_indices, payout, &mut hand_results);
	    }
        } else {
            // the hand ended before Showdown, so we simple find the one active player remaining
            let mut best_indices = HashSet::<usize>::new();	    
            for (i, player) in players.iter().enumerate() {
		if player.is_none() {
		    continue;
		}
                if player.as_ref().unwrap().is_active {
                    //println!("found an active player remaining");
                    best_indices.insert(i);
                } else {
                    println!("found an NON active player remaining");
                }
            }
	    // if we didn't make it to show down, there better be only one player left	    
            assert!(best_indices.len() == 1);
	    self.pay_players(players, player_ids_to_configs, best_indices,
			     self.pots.last().unwrap().money, &mut hand_results);
        }

        // take the players' cards
        for player in players.iter_mut().flatten() {
            // todo: is there any issue with calling drain if they dont have any cards?
            player.hole_cards.drain(..);
            if !player.is_sitting_out {
                if player.money == 0 {
                    println!(
                        "Player {:?} is out of money so is no longer playing in the game!",
                        player.id
                    );
                    player.is_sitting_out = true;
                }
            }
        }
    }


    fn pay_players(
	&mut self,
	players: &mut [Option<Player>],
	player_ids_to_configs: &HashMap<Uuid, PlayerConfig>,		   
	best_indices: HashSet::<usize>,
	payout: u32,
	hand_results: &mut Vec<Option<HandResult>>,
    ) {
	println!("best_indices = {:?}", best_indices);
        for idx in best_indices {
            let winning_spot = &mut players[idx];
	    if let Some(ref mut winning_player) = winning_spot {
		let name = &player_ids_to_configs.get(&winning_player.id).unwrap().name; // get the name for message
		println!(
                    "paying out: {:?} \n  with hand result = {:?}",
                    name, hand_results[idx]
		);
		PlayerConfig::send_group_message(
		    &format!("paying out {:?} to {:?}, with hand result = {:?}",
			     payout, name.as_ref().unwrap(), hand_results[idx]),
		    &player_ids_to_configs);			
		
		winning_player.pay(payout);
		println!("after payment: {:?}", winning_player);
	    } else {
		panic!("we did not find a player at one of the suposed best indices!");
	    }
        }
    }
    
    /// Given a player, we need to determine which 5 cards make the best hand for this player
    /// If a player spot is empty, i.e. player.is_none(), then we simply return None
    fn determine_best_hand(&self, player_opt: Option<&Player>) -> Option<HandResult> {
	let player = if player_opt.is_none() {
	    return None;
	} else {
	    player_opt.unwrap()
	};
	
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
            deck: &mut Box<dyn Deck>,	    
	    players: &mut [Option<Player>],
	    player_ids_to_configs: &mut HashMap<Uuid, PlayerConfig>,
	    incoming_actions: &Arc<Mutex<HashMap<Uuid, PlayerAction>>>,
	    incoming_meta_actions: &Arc<Mutex<VecDeque<MetaAction>>>,
	    hub_addr: Option<&Addr<GameHub>>,	    
    ) {
        println!("inside of play(). button_idx = {:?}", self.button_idx);
	for player in players.iter_mut().flatten() {
	    if player.is_sitting_out || player.money == 0 {
		// if a player has no money they should be sitting out, but to be safe check both
		player.is_active = false;		
	    } else {
		player.is_active = true;
	    }
	}
	/*
        PlayerConfig::send_group_message(&format!(
            "inside of play(). button_idx = {:?}",
            self.button_idx
        ), player_ids_to_configs);
	 */
        deck.shuffle();
        self.deal_hands(deck, players, player_ids_to_configs);

        println!("players = {:?}", players);
        //PlayerConfig::send_group_message(&format!("players = {:?}", players), player_ids_to_configs);
        while self.street != Street::ShowDown {
            let finished = self.play_street(players, player_ids_to_configs,
					    incoming_actions, incoming_meta_actions, hub_addr);
	    if finished {
                // if the game is over from players folding
                println!("\nGame is ending before showdown!");
                PlayerConfig::send_group_message("\nGame is ending before showdown!", player_ids_to_configs);
                break;
            } else {
                // otherwise we move to the next street
                self.transition(deck, player_ids_to_configs);
            }
        }
        // now we finish up and pay the pot to the winner
        self.finish(players, player_ids_to_configs);
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

    /// this method returns a bool indicating whether the hand is over or not
    fn play_street(
	&mut self,
	players: &mut [Option<Player>],
	player_ids_to_configs: &mut HashMap<Uuid, PlayerConfig>,
	incoming_actions: &Arc<Mutex<HashMap<Uuid, PlayerAction>>>,
	incoming_meta_actions: &Arc<Mutex<VecDeque<MetaAction>>>,
	hub_addr: Option<&Addr<GameHub>>,
    ) -> bool {
        let mut current_bet: u32 = 0;
        // each index keeps track of that players' contribution this street
        let mut cumulative_bets = vec![0; players.len()];

        let starting_idx = self.get_starting_idx(players); // which player starts the betting

        // if a player is still active but has no remaining money (i.e. is all-in),
        let mut num_all_in = players
            .iter()
            .flatten() // skip over None values
            .filter(|player| player.is_all_in())
            .count();

        let mut num_active = players
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
	
        println!("Current pot = {:?}", self.pots.last());
        PlayerConfig::send_group_message(&format!("Current pot = {:?}", self.pots.last()), player_ids_to_configs);

        println!("num active players = {}", num_active);
        PlayerConfig::send_group_message(&format!("num active players = {}", num_active), player_ids_to_configs);

	
        println!("player at index {} starts the betting", starting_idx);
        if num_settled > 0 {
            println!("num settled (i.e. all in players) = {}", num_settled);
            PlayerConfig::send_group_message(&format!(
                "num settled (i.e. all in players) = {}",
                num_settled
            ), player_ids_to_configs);
        }
        'street: loop {
            // iterate over the players from the starting index to the end of the vec,
            // and then from the beginning back to the starting index
	    // TODO: can i change this to loop over indixes instead? then we don't need to borrow the
	    // players the whole time? then we can loop over the players inside this loop as needed perhaps?
            let (left, right) = players.split_at_mut(starting_idx);
            for (i, mut player) in right.iter_mut().chain(left.iter_mut()).flatten().enumerate() {
		/*
		I think this was redundant if we are just gunna fold further down
		if !player_ids_to_configs.contains_key(&player.id) {
		    println!("no player config so we are deactivating the player");
		    player.deactivate();
		}*/

                println!("start loop: num_active = {}, num_settled = {}, num_all_in = {}",
			 num_active, num_settled, num_all_in);
		
                if num_active == 1 {
                    println!("Only one active player left so lets break the steet loop");
		    // end the street and indicate to the caller that the hand is finished
                    break 'street true;
                }
                if num_settled + num_all_in == num_active {
                    println!(
                        "everyone is ready to go to the next street! num_settled = {}",
                        num_settled
                    );
		    // end the street and indicate to the caller that the hand is going to the next street
                    break 'street false;
                }
		
                let player_cumulative = cumulative_bets[i];
                println!("Current pot = {:?}, Current size of the bet = {:?}, and this player has put in {:?} so far",
			 self.pots,
			 current_bet,
			 player_cumulative);

                println!("Player = {:?}, i = {}", player.id, i);
                if player.is_active && player.money > 0 {
		    // get the name for messages		    
		    let name = if let Some(config) = &player_ids_to_configs.get(&player.id) {
			config.name.as_ref().unwrap().clone()
		    } else {
			"Player who left".to_string()
		    };
		    
                    let action = self.get_and_validate_action(
                        player,
                        current_bet,
                        player_cumulative,
			player_ids_to_configs,
			incoming_actions,
			incoming_meta_actions,
			hub_addr,
                    );
		    
                    match action {
                        PlayerAction::PostSmallBlind(amount) => {
                            println!("Player {:?} posts small blind of {}", name, amount);
			    PlayerConfig::send_group_message(
				&format!("Player {:?} posts small blind of {}", name, amount),
				player_ids_to_configs);
			    
                            self.pots.last_mut().unwrap().money += amount;
                            cumulative_bets[i] += amount;
			    self.total_contributions[i] += amount;
                            player.money -= amount;
                            // regardless if the player couldn't afford it, the new street bet is the big blind
                            current_bet = self.small_blind;
                            if player.is_all_in() {
                                num_all_in += 1;
			    }
                        }
                        PlayerAction::PostBigBlind(amount) => {
                            println!("Player {:?} posts big blind of {}", name, amount);			    
			    PlayerConfig::send_group_message(
				&format!("Player {:?} posts big blind of {}", name, amount),
				player_ids_to_configs);
			    
                            self.pots.last_mut().unwrap().money += amount;
                            cumulative_bets[i] += amount;
			    self.total_contributions[i] += amount;			    
                            player.money -= amount;
                            // regardless if the player couldn't afford it, the new street bet is the big blind
                            current_bet = self.big_blind;
                            if player.is_all_in() {
                                num_all_in += 1;
			    }
			    // note: we dont count the big blind as a "settled" player,
			    // since they still get a chance to act after the small blind
                        }
                        PlayerAction::Fold => {
                            println!("Player {:?} folds!", name);
			    PlayerConfig::send_group_message(
				&format!("Player {:?} folds", name),
				player_ids_to_configs);			    
                            player.deactivate();
                            num_active -= 1;
                        }
                        PlayerAction::Check => {
                            println!("Player checks!");
			    PlayerConfig::send_group_message(
				&format!("Player {:?} checks", name),
				player_ids_to_configs);			    
                            num_settled += 1;
                        }
                        PlayerAction::Call => {
                            println!("Player calls!");
			    PlayerConfig::send_group_message(
				&format!("Player {:?} calls", name),
				player_ids_to_configs);			    
                            let difference = current_bet - player_cumulative;
                            if difference >= player.money {
                                println!("you have to put in the rest of your chips");
                                self.pots.last_mut().unwrap().money += player.money;
                                cumulative_bets[i] += player.money;
				self.total_contributions[i] += player.money;				
                                player.money = 0;
                                num_all_in += 1;
                            } else {
                                self.pots.last_mut().unwrap().money += difference;
                                cumulative_bets[i] += difference;
				self.total_contributions[i] += difference;
                                player.money -= difference;
				num_settled += 1;				
                            }
                        }
                        PlayerAction::Bet(new_bet) => {
                            println!("Player bets {}!", new_bet);
			    PlayerConfig::send_group_message(
				&format!("Player {:?} bets {:?}", name, new_bet),
				player_ids_to_configs);			    			    
                            let difference = new_bet - player_cumulative;
                            self.pots.last_mut().unwrap().money += difference;
                            player.money -= difference;
                            current_bet = new_bet;
                            cumulative_bets[i] += difference;
			    self.total_contributions[i] += difference;
                            if player.is_all_in() {
                                println!("Just bet the rest of our money!");
                                num_all_in += 1;
				num_settled = 0;
			    } else {
				num_settled = 1;
			    }
                        }
                    }
                }

		// send a money message so the client can update accordingly
		PlayerConfig::send_specific_message(
		    &format!("Money: {}", player.money),
		    player.id,
		    player_ids_to_configs
		);

                println!("after player: num_active = {}, num_settled = {}, num_all_in = {}",
			 num_active, num_settled, num_all_in);
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
                    let amount: u32 = if player.money <= 100 {
                        // just go all in if we are at 10% starting
                        player.money
                    } else {
                        rand::thread_rng().gen_range(1..player.money/2 as u32)
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
        current_bet: u32,
        player_cumulative: u32,
	player_ids_to_configs: &mut HashMap<Uuid, PlayerConfig>,
	incoming_actions: &Arc<Mutex<HashMap<Uuid, PlayerAction>>>,
	incoming_meta_actions: &Arc<Mutex<VecDeque<MetaAction>>>,
	hub_addr: Option<&Addr<GameHub>>,
    ) -> PlayerAction {
        // if it isnt valid based on the current bet and the amount the player has already contributed,
        // then it loops
        // position is our spot in the order, with 0 == small blind, etc

	// we sleep a little bit each time so that the output doesnt flood the user at one moment
        let pause_duration = time::Duration::from_secs(1); 	
        thread::sleep(pause_duration);
	
        if self.street == Street::Preflop && current_bet == 0 {
            // collect small blind!
            return PlayerAction::PostSmallBlind(cmp::min(
                self.small_blind,
                player.money,
            ));
        } else if self.street == Street::Preflop && current_bet == self.small_blind {
            // collect big blind!
            return PlayerAction::PostBigBlind(
                cmp::min(self.big_blind, player.money),
            );
        }
	PlayerConfig::send_specific_message(
	    &format!("Please enter your action (current bet = {}): ", current_bet),
	    player.id,
	    player_ids_to_configs
	);
	
        let mut action = None;
        let mut attempts = 0;
        let retry_duration = time::Duration::from_secs(2); // how long to wait between trying again
        while attempts < 10000 && action.is_none() {
	    // the first thing we do on each loop is handle meta action
	    Game::handle_meta_actions(player_ids_to_configs, incoming_meta_actions, hub_addr);
		
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
	    if !player_ids_to_configs.contains_key(&player.id) {
		// the config no longer exists for this player, so they must have left
		println!("player config no longer exists, so the player must have left");
		action = Some(PlayerAction::Fold);
		break;		
	    }
	    
            println!("Attempting to get player action on attempt {:?}", attempts);
            match GameHand::get_action_from_player(player, incoming_actions) {
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
			    PlayerConfig::send_specific_message(			    
				&"You said fold but we will let you check!".to_owned(),
				player.id,
				player_ids_to_configs
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
                        if current_bet != 0 {
                            // if the street bet isn't 0 then this makes no sense
                            println!("should we even be here???!");
                        }
			// we can let them check
			PlayerConfig::send_specific_message(
			    &"There is nothing for you to call!!".to_owned(),
			    player.id,
			    player_ids_to_configs
			);
			
                        action = Some(PlayerAction::Check);			
                    } else {
			action = Some(PlayerAction::Call);
		    }
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
			PlayerConfig::send_specific_message(
			    &"You can't bet more than you have!!".to_owned(),
			    player.id,
			    player_ids_to_configs
			);
                        continue;
                    }
                    if new_bet <= current_bet {
			println!("new bet must be larger than current");
			PlayerConfig::send_specific_message(
			    &"the new bet has to be larger than the current bet!".to_owned(),
			    player.id,
			    player_ids_to_configs
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

#[derive(Debug)]
pub struct Game {
    hub_addr: Option<Addr<GameHub>>, // needs to be able to communicate back to the hub sometimes
    deck: Box<dyn Deck>,
    players: [Option<Player>; 9], // 9 spots where players can sit
    player_ids_to_configs: HashMap<Uuid, PlayerConfig>,
    button_idx: usize, // index of the player with the button
    small_blind: u32,
    big_blind: u32,
}

impl Game {

    /// the address of the GameHub is optional so that unit tests need not worry about it
    /// We can pass in a custom Deck object, but if not, we will just construct a StandardDeck
    pub fn new(hub_addr: Option<Addr<GameHub>>, deck_opt: Option<Box<dyn Deck>>) -> Self {
	let deck = if deck_opt.is_some() {
	    deck_opt.unwrap()
	} else {
	    Box::new(StandardDeck::new())
	};
        Game {
	    hub_addr,
            deck,
            players: Default::default(), 
	    player_ids_to_configs: HashMap::<Uuid, PlayerConfig>::new(),
            small_blind: 4,
            big_blind: 8,
            button_idx: 0,
        }
    }

    /// add a given playerconfig to an empty seat
    /// TODO: eventually we wanmt the player to select an open seat I guess
    pub fn add_user(&mut self, player_config: PlayerConfig) -> bool {
	self.add_player(player_config, true)
    }

    pub fn add_bot(&mut self, name: String) -> bool {
	let new_bot = Player::new_bot();
	let new_config = PlayerConfig::new(new_bot.id, Some(name), None);
	self.add_player(new_config, false)
    }
    
    fn add_player(&mut self, player_config: PlayerConfig, human_controlled: bool) -> bool{
	let mut added = false;
	for player_spot in self.players.iter_mut() {
	    if player_spot.is_none() {
		*player_spot = Some(Player::new(player_config.id, human_controlled));
		self.player_ids_to_configs.insert(player_config.id, player_config);
		added = true;
		break;
	    }
	}
	added
    }
            
    /// send a given message to all the players at the tabel
    pub fn send_message(&self, message: &str) {
        PlayerConfig::send_group_message(message, &self.player_ids_to_configs);
    }
    
    pub fn play_one_hand(
	&mut self,
	incoming_actions: &Arc<Mutex<HashMap<Uuid, PlayerAction>>>,
	incoming_meta_actions: &Arc<Mutex<VecDeque<MetaAction>>>,
    ) {
        let mut game_hand = GameHand::new(
            self.button_idx,
            self.small_blind,
            self.big_blind,
        );
        game_hand.play(
	    &mut self.deck,	    
	    &mut self.players,
	    &mut self.player_ids_to_configs,
	    incoming_actions,
	    incoming_meta_actions,
	    self.hub_addr.as_ref());
    }

    pub fn play(
	&mut self,
	incoming_actions: &Arc<Mutex<HashMap<Uuid, PlayerAction>>>,
	incoming_meta_actions: &Arc<Mutex<VecDeque<MetaAction>>>,
    ) {
        let mut hand_count = 0;
        loop {
            hand_count += 1;
            println!(
                "\n\n\nPlaying hand {}",
                hand_count
            );
	    PlayerConfig::send_group_message(&format!("Playing hand {}", hand_count),
					     &self.player_ids_to_configs);			
	    
	    
	    println!("self.incoming_actions = {:?}", incoming_actions.lock().unwrap());
            self.play_one_hand(incoming_actions, incoming_meta_actions);

	    // check if any player is now missing from the config mapping,
	    // this implies that the player left mid-hand, so they should fully be removed from the game
	    for player_spot in self.players.iter_mut() {
		if let Some(player) = player_spot {
		    if !self.player_ids_to_configs.contains_key(&player.id) {
			// the player is no more
			println!("removing player {:?} since no longer in the config between hands", player);
			*player_spot = None;
		    }
		}
	    }
	    
	    
	    let mut meta_actions = incoming_meta_actions.lock().unwrap();
	    println!("meta_actions = {:?}", meta_actions);
	    while !meta_actions.is_empty() {
		match meta_actions.pop_front().unwrap() {
		    MetaAction::Chat(id, text) => {
			// send the message to all players,
			// appended by the player name
			// TODO we want chat to happen more in real-time, not between hands
			let name = &self.player_ids_to_configs.get(&id).unwrap().name;
			PlayerConfig::send_group_message(&format!("{:?}: {:?}", name, text ),
							 &self.player_ids_to_configs);			
			
		    },
		    MetaAction::Join(player_config) => {
			// add a new player to the game
			let id = player_config.id; // copy the id so we can use to send a message later
			let added = self.add_user(player_config);
			if !added {
			    // we were unable to add the player
			    PlayerConfig::send_specific_message(
				&"Unable to join game, it must be full!".to_owned(),
				id,
				&self.player_ids_to_configs,
			    );
			    
			    
			}
		    },
		    MetaAction::Leave(id) => {
			for player_spot in self.players.iter_mut() {
			    if let Some(player) = player_spot {
				if player.id == id {
				    *player_spot = None;
				}
			    }
			}
			// grab the associated config, and send it back to
			// the game hub
			let config = self.player_ids_to_configs.remove(&id).unwrap();
			PlayerConfig::send_group_message(&format!("{:?} has left the game", config.name),
							 &self.player_ids_to_configs);
			if self.hub_addr.is_some() {
			    self.hub_addr.as_ref().unwrap().do_send(Removed{config});
			}
		    },
		    MetaAction::PlayerName(id, new_name) => {
			PlayerConfig::set_player_name(id, &new_name, &mut self.player_ids_to_configs);
		    }
		}
		
	    }
            let mut loop_count = 0;
            'find_button: loop {
                loop_count += 1;
                if loop_count >= 5 {
                    // couldn't find a valid button position. how does this happen?
		    println!("could not find a button spot!");
                    break 'find_button;
                }
                self.button_idx += 1; // and modulo length
		// TODO there is a bug here. should not use players.len()
		// should look at the number of active?, or we should do this at the START of the next one?
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

    fn handle_meta_actions(
	player_ids_to_configs: &mut HashMap<Uuid, PlayerConfig>,	
	incoming_meta_actions: &Arc<Mutex<VecDeque<MetaAction>>>,
	hub_addr: Option<&Addr<GameHub>>,
    ) {
	let mut meta_actions = incoming_meta_actions.lock().unwrap();
	//println!("meta_actions = {:?}", meta_actions);
	for _ in 0..meta_actions.len() {
	    match meta_actions.pop_front().unwrap() {
		MetaAction::Chat(id, text) => {
		    // send the message to all players,
		    // appended by the player name
		    println!("chat message inside the game hand wow!");
		    let name = &player_ids_to_configs.get(&id).unwrap().name;
		    PlayerConfig::send_group_message(&format!("{:?}: {:?}", name, text ),
						     &player_ids_to_configs);			
		},
		MetaAction::Leave(id) => {
		    // TODO figure this out
		    // I think we can just set the player to be sitting out for now
		    // and release the player config to the hub?
		    // and then maybe at the veryu end of game hand when we finish() we can remove the
		    // player that left officially?
		    // Does the Game object even need to know this happened if the player spot is now None
		    // and the config is gone?		    
		    // grab the associated config, and send it back to
		    // the game hub
		    println!("handling leave meta action");
		    let config = player_ids_to_configs.remove(&id).unwrap();
		    PlayerConfig::send_group_message(&format!("{:?} has left the game", config.name),
		    				     &player_ids_to_configs);
		    if hub_addr.is_some() {
			// tell the hub that we left			
			hub_addr.unwrap().do_send(Removed{config});
		    }
		},
		MetaAction::PlayerName(id, new_name) => {
		    PlayerConfig::set_player_name(id, &new_name, player_ids_to_configs);
		}
		other => {
		    // put it back onto thhe queue
		    meta_actions.push_back(other);
		}
	    }
	
	}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::card::{Card, Suit, Rank, RiggedDeck};
    use std::collections::{HashMap};
    
    #[test]
    fn add_bot() {
        let mut game = Game::new(None, None);
        let name = "Mr Bot".to_string();
        game.add_bot(name);
        assert_eq!(game.players.len(), 9);
	// flatten to get all the Some() players
	let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 1);
        assert!(!game.players[0].as_ref().unwrap().human_controlled);
    }
    
    #[test]
    fn add_user_no_connection() {
        let mut game = Game::new(None, None);	
        let id = uuid::Uuid::new_v4();
        let name = "Human".to_string();
        let settings = PlayerConfig::new(id, Some(name), None);
        game.add_user(settings);
        assert_eq!(game.players.len(), 9);
	// flatten to get all the Some() players
	let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 1);
        assert!(game.players[0].as_ref().unwrap().human_controlled);
    }


    /// the small blind folds, so the big blind should win and get paid
    #[test]
    fn instant_fold() {
        let mut game = Game::new(None, None);	


	// player1 will start as the button
	let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1);

	// player2 will start as the small blind
	let id2 = uuid::Uuid::new_v4();
        let name2 = "Human1".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2);
	// flatten to get all the Some() players
	let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);
	
	let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));	
	let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));

	let cloned_actions = incoming_actions.clone();	    
	let cloned_meta_actions = incoming_meta_actions.clone();
	let handler = thread::spawn( move || {
	    game.play_one_hand(&cloned_actions, &cloned_meta_actions);
	    game // return the game back
	});
	
	// set the action that player2 folds
	incoming_actions.lock().unwrap().insert(id2, PlayerAction::Fold);

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
        let mut game = Game::new(None, None);	

	// player1 will start as the button
	let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1);

	// player2 will start as the small blind
	let id2 = uuid::Uuid::new_v4();
        let name2 = "Human1".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2);
	// flatten to get all the Some() players
	let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);
	
	let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));	
	let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));


	let cloned_actions = incoming_actions.clone();	    
	let cloned_meta_actions = incoming_meta_actions.clone();
	let handler = thread::spawn( move || {
	    game.play_one_hand(&cloned_actions, &cloned_meta_actions);
	    game // return the game back
	});
	
	// set the action that player2 calls
	incoming_actions.lock().unwrap().insert(id2, PlayerAction::Call);
	// player1 checks
	incoming_actions.lock().unwrap().insert(id1, PlayerAction::Check);


	// wait for the flop
        let wait_duration = time::Duration::from_secs(7);
	std::thread::sleep(wait_duration);

	// player2 bets on the flop
	println!("now sending the flop actions");	
	incoming_actions.lock().unwrap().insert(id2, PlayerAction::Bet(10));
	// player1 folds
	incoming_actions.lock().unwrap().insert(id1, PlayerAction::Fold);
	
	// get the game back from the thread
	let game = handler.join().unwrap();
	
	// check that the money changed hands
	assert_eq!(game.players[0].as_ref().unwrap().money, 992);
	assert_eq!(game.players[1].as_ref().unwrap().money, 1008);	
    }

    /// the small blind bets, the big blind folds
    #[test]
    fn pre_flop_bet_fold() {
        let mut game = Game::new(None, None);	

	// player1 will start as the button
	let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1);

	// player2 will start as the small blind
	let id2 = uuid::Uuid::new_v4();
        let name2 = "Human1".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2);
	// flatten to get all the Some() players
	let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);
	
	let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));	
	let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));


	let cloned_actions = incoming_actions.clone();	    
	let cloned_meta_actions = incoming_meta_actions.clone();
	let handler = thread::spawn( move || {
	    game.play_one_hand(&cloned_actions, &cloned_meta_actions);
	    game // return the game back
	});
	
	// set the action that player2 bets
	incoming_actions.lock().unwrap().insert(id2, PlayerAction::Bet(22));
	// player1 folds
	incoming_actions.lock().unwrap().insert(id1, PlayerAction::Fold);
	
	// get the game back from the thread
	let game = handler.join().unwrap();
	
	// check that the money changed hands
	assert_eq!(game.players[0].as_ref().unwrap().money, 992);
	assert_eq!(game.players[1].as_ref().unwrap().money, 1008);	
    }

    /// the small blind bets, the big blind calls
    /// the small blind bets on the flop, and the big blind folds
    #[test]
    fn bet_call_bet_fold() {
        let mut game = Game::new(None, None);	

	// player1 will start as the button
	let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1);

	// player2 will start as the small blind
	let id2 = uuid::Uuid::new_v4();
        let name2 = "Human1".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2);
	// flatten to get all the Some() players
	let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);
	
	let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));	
	let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));


	let cloned_actions = incoming_actions.clone();	    
	let cloned_meta_actions = incoming_meta_actions.clone();
	let handler = thread::spawn( move || {
	    game.play_one_hand(&cloned_actions, &cloned_meta_actions);
	    game // return the game back
	});
	
	// set the action that player2 bets
	incoming_actions.lock().unwrap().insert(id2, PlayerAction::Bet(22));
	// player1 calls
	incoming_actions.lock().unwrap().insert(id1, PlayerAction::Call);

	// wait for the flop
        let wait_duration = time::Duration::from_secs(7);
	std::thread::sleep(wait_duration);

	// player2 bets on the flop
	println!("now sending the flop actions");	
	incoming_actions.lock().unwrap().insert(id2, PlayerAction::Bet(10));
	// player1 folds
	incoming_actions.lock().unwrap().insert(id1, PlayerAction::Fold);
	
	// get the game back from the thread
	let game = handler.join().unwrap();
	
	// check that the money changed hands
	assert_eq!(game.players[0].as_ref().unwrap().money, 978);
	assert_eq!(game.players[1].as_ref().unwrap().money, 1022);	
    }

    /// the small blind goes all in and the big blind calls
    /// the rest will just happen automatically
    #[test]
    fn all_in_call() {
	let mut deck = RiggedDeck::new();

	// we want the button/big blind to lose for testing purposes
	deck.push(Card{rank: Rank::Two, suit: Suit::Club});
	deck.push(Card{rank: Rank::Three, suit: Suit::Club});	

	// now the small blind's hole cards
	deck.push(Card{rank: Rank::Ten, suit: Suit::Club});
	deck.push(Card{rank: Rank::Ten, suit: Suit::Heart});
	
	// now the full run out
	deck.push(Card{rank: Rank::Ten, suit: Suit::Diamond});
	deck.push(Card{rank: Rank::Ten, suit: Suit::Spade});	
	deck.push(Card{rank: Rank::King, suit: Suit::Club});
	deck.push(Card{rank: Rank::King, suit: Suit::Heart});	
	deck.push(Card{rank: Rank::Queen, suit: Suit::Club});
	
        let mut game = Game::new(None, Some(Box::new(deck)));	

	// player1 will start as the button
	let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1);

	// player2 will start as the small blind
	let id2 = uuid::Uuid::new_v4();
        let name2 = "Human1".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2);
	// flatten to get all the Some() players
	let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);
	
	let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));	
	let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));


	let cloned_actions = incoming_actions.clone();	    
	let cloned_meta_actions = incoming_meta_actions.clone();
	let handler = thread::spawn( move || {
	    game.play_one_hand(&cloned_actions, &cloned_meta_actions);
	    game // return the game back
	});
	
	// set the action that player2 bets
	incoming_actions.lock().unwrap().insert(id2, PlayerAction::Bet(1000));
	// player1 calls
	incoming_actions.lock().unwrap().insert(id1, PlayerAction::Call);
	
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
	deck.push(Card{rank: Rank::Two, suit: Suit::Club});
	deck.push(Card{rank: Rank::Three, suit: Suit::Club});	

	// now the small blind's hole cards
	deck.push(Card{rank: Rank::Ten, suit: Suit::Club});
	deck.push(Card{rank: Rank::Ten, suit: Suit::Heart});
	
	// now the full run out
	deck.push(Card{rank: Rank::Ten, suit: Suit::Diamond});
	deck.push(Card{rank: Rank::Ten, suit: Suit::Spade});	
	deck.push(Card{rank: Rank::King, suit: Suit::Club});
	deck.push(Card{rank: Rank::King, suit: Suit::Heart});	
	deck.push(Card{rank: Rank::Queen, suit: Suit::Club});

	
        let mut game = Game::new(None, Some(Box::new(deck)));	
	

	// player1 will start as the button
	let id1 = uuid::Uuid::new_v4();
        let name1 = "Human1".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1);

	game.players[0].as_mut().unwrap().money = 500; // set the player to have less money
    
	// player2 will start as the small blind
	let id2 = uuid::Uuid::new_v4();
        let name2 = "Human1".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2);
	// flatten to get all the Some() players
	let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 2);
        assert!(game.players[0].as_ref().unwrap().human_controlled);
	
	let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));	
	let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));


	let cloned_actions = incoming_actions.clone();	    
	let cloned_meta_actions = incoming_meta_actions.clone();
	let handler = thread::spawn( move || {
	    game.play_one_hand(&cloned_actions, &cloned_meta_actions);
	    game // return the game back
	});
	
	// set the action that player2 bets
	incoming_actions.lock().unwrap().insert(id2, PlayerAction::Bet(500));
	// player1 calls
	incoming_actions.lock().unwrap().insert(id1, PlayerAction::Call);
	
	// get the game back from the thread
	let game = handler.join().unwrap();

	// the small blind won
	assert_eq!(game.players[0].as_ref().unwrap().money, 0);
	assert_eq!(game.players[1].as_ref().unwrap().money, 1500);	
    }

    /// if a player goes all-in, then can only win as much as is called up to that amount,
    /// even if other players keep playing and betting during this hand
    /// In this test, the side point is won by the short stack, then the remaining is won
    /// by another player
    #[test]
    fn outright_side_pot() {
	let mut deck = RiggedDeck::new();

	// we want the button to win his side pot
	deck.push(Card{rank: Rank::Ace, suit: Suit::Club});
	deck.push(Card{rank: Rank::Ace, suit: Suit::Diamond});	

	// the small blind will win the main pot against the big blind
	deck.push(Card{rank: Rank::Ten, suit: Suit::Club});
	deck.push(Card{rank: Rank::Ten, suit: Suit::Heart});

	// the big blind loses
	deck.push(Card{rank: Rank::Two, suit: Suit::Club});
	deck.push(Card{rank: Rank::Four, suit: Suit::Heart});
	
	// now the full run out
	deck.push(Card{rank: Rank::Three, suit: Suit::Diamond});
	deck.push(Card{rank: Rank::Eight, suit: Suit::Spade});	
	deck.push(Card{rank: Rank::Nine, suit: Suit::Club});
	deck.push(Card{rank: Rank::King, suit: Suit::Heart});	
	deck.push(Card{rank: Rank::King, suit: Suit::Club});
	
        let mut game = Game::new(None, Some(Box::new(deck)));		

	// player1 will start as the button
	let id1 = uuid::Uuid::new_v4();
        let name1 = "Button".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1);
	// set the button to have less money so there is a side pot	
	game.players[0].as_mut().unwrap().money = 500; 
	
	// player2 will start as the small blind
	let id2 = uuid::Uuid::new_v4();
        let name2 = "Small".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2);

	// player3 will start as the big blind
	let id3 = uuid::Uuid::new_v4();
        let name3 = "Big".to_string();
        let settings3 = PlayerConfig::new(id3, Some(name3), None);
        game.add_user(settings3);
	
	// flatten to get all the Some() players
	let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 3);
        assert!(game.players[0].as_ref().unwrap().human_controlled);
        assert!(game.players[1].as_ref().unwrap().human_controlled);
        assert!(game.players[2].as_ref().unwrap().human_controlled);	
	
	let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));	
	let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));

	let cloned_actions = incoming_actions.clone();	    
	let cloned_meta_actions = incoming_meta_actions.clone();
	let handler = thread::spawn( move || {
	    game.play_one_hand(&cloned_actions, &cloned_meta_actions);
	    game // return the game back
	});
	
	// the button goes all in with the short stack
	incoming_actions.lock().unwrap().insert(id1, PlayerAction::Bet(500));
	// the small blind goes all in with a full stack
	incoming_actions.lock().unwrap().insert(id2, PlayerAction::Bet(1000));	
	// the big blind calls the full all-in
	incoming_actions.lock().unwrap().insert(id3, PlayerAction::Call);
	
	// get the game back from the thread
	let game = handler.join().unwrap();

	// the button won the side pot
	assert_eq!(game.players[0].as_ref().unwrap().money, 1500);

	// the small blind won the remainder
	assert_eq!(game.players[1].as_ref().unwrap().money, 1000);
	
	// the big blind lost everything
	assert_eq!(game.players[1].as_ref().unwrap().money, 0);	
    }

    /// if a player goes all-in, then can only win as much as is called up to that amount,
    /// even if other players keep playing and betting during this hand
    /// In this test, the small stack ties with one of the other players, so the side spot should be split
    /// This other player beats the third player
    #[test]
    fn tie_side_pot() {
	let mut deck = RiggedDeck::new();

	// we want the button to win his side pot
	deck.push(Card{rank: Rank::Ace, suit: Suit::Club});
	deck.push(Card{rank: Rank::Ace, suit: Suit::Diamond});	

	// the small blind will tie the sidepot and win the main pot against the big blind
	deck.push(Card{rank: Rank::Ace, suit: Suit::Club});
	deck.push(Card{rank: Rank::Ace, suit: Suit::Heart});

	// the big blind loses
	deck.push(Card{rank: Rank::Two, suit: Suit::Club});
	deck.push(Card{rank: Rank::Four, suit: Suit::Heart});
	
	// now the full run out
	deck.push(Card{rank: Rank::Three, suit: Suit::Diamond});
	deck.push(Card{rank: Rank::Eight, suit: Suit::Spade});	
	deck.push(Card{rank: Rank::Nine, suit: Suit::Club});
	deck.push(Card{rank: Rank::King, suit: Suit::Heart});	
	deck.push(Card{rank: Rank::King, suit: Suit::Club});
	
        let mut game = Game::new(None, Some(Box::new(deck)));		

	// player1 will start as the button
	let id1 = uuid::Uuid::new_v4();
        let name1 = "Button".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1);
	// set the button to have less money so there is a side pot	
	game.players[0].as_mut().unwrap().money = 500; 
	
	// player2 will start as the small blind
	let id2 = uuid::Uuid::new_v4();
        let name2 = "Small".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2);

	// player3 will start as the big blind
	let id3 = uuid::Uuid::new_v4();
        let name3 = "Big".to_string();
        let settings3 = PlayerConfig::new(id3, Some(name3), None);
        game.add_user(settings3);
	
	// flatten to get all the Some() players
	let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 3);
        assert!(game.players[0].as_ref().unwrap().human_controlled);
        assert!(game.players[1].as_ref().unwrap().human_controlled);
        assert!(game.players[2].as_ref().unwrap().human_controlled);	
	
	let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));	
	let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));

	let cloned_actions = incoming_actions.clone();	    
	let cloned_meta_actions = incoming_meta_actions.clone();
	let handler = thread::spawn( move || {
	    game.play_one_hand(&cloned_actions, &cloned_meta_actions);
	    game // return the game back
	});
	
	// the button goes all in with the short stack
	incoming_actions.lock().unwrap().insert(id1, PlayerAction::Bet(500));
	// the small blind goes all in with a full stack
	incoming_actions.lock().unwrap().insert(id2, PlayerAction::Bet(1000));	
	// the big blind calls the full all-in
	incoming_actions.lock().unwrap().insert(id3, PlayerAction::Call);
	
	// get the game back from the thread
	let game = handler.join().unwrap();

	// the button won the side pot
	assert_eq!(game.players[0].as_ref().unwrap().money, 750);

	// the small blind won the remainder
	assert_eq!(game.players[1].as_ref().unwrap().money, 1750);
	
	// the big blind lost everything
	assert_eq!(game.players[1].as_ref().unwrap().money, 0);	
    }

    /// if a player goes all-in, then can only win as much as is called up to that amount,
    /// even if other players keep playing and betting during this hand
    /// In this test, the side point is won by the small stack, then medium stack wins a separate
    /// side pot, and finally, the rest of the chips are won by a third player
    
    #[test]
    fn multiple_side_pots() {
	let mut deck = RiggedDeck::new();

	// we want the button to win his side pot
	deck.push(Card{rank: Rank::Ace, suit: Suit::Club});
	deck.push(Card{rank: Rank::Ace, suit: Suit::Diamond});	

	// the small blind will win the remaining
	deck.push(Card{rank: Rank::Six, suit: Suit::Club});
	deck.push(Card{rank: Rank::Six  , suit: Suit::Heart});

	// the big blind loses
	deck.push(Card{rank: Rank::Two, suit: Suit::Club});
	deck.push(Card{rank: Rank::Four, suit: Suit::Heart});

	// UTG wins the second side pot
	deck.push(Card{rank: Rank::Queen, suit: Suit::Club});
	deck.push(Card{rank: Rank::Queen, suit: Suit::Heart});
	
	// now the full run out
	deck.push(Card{rank: Rank::Three, suit: Suit::Diamond});
	deck.push(Card{rank: Rank::Eight, suit: Suit::Spade});	
	deck.push(Card{rank: Rank::Nine, suit: Suit::Club});
	deck.push(Card{rank: Rank::King, suit: Suit::Heart});	
	deck.push(Card{rank: Rank::King, suit: Suit::Club});
	
        let mut game = Game::new(None, Some(Box::new(deck)));		

	// player1 will start as the button
	let id1 = uuid::Uuid::new_v4();
        let name1 = "Button".to_string();
        let settings1 = PlayerConfig::new(id1, Some(name1), None);
        game.add_user(settings1);
	// set the button to have less money so there is a side pot	
	game.players[0].as_mut().unwrap().money = 500; 
	
	// player2 will start as the small blind
	let id2 = uuid::Uuid::new_v4();
        let name2 = "Small".to_string();
        let settings2 = PlayerConfig::new(id2, Some(name2), None);
        game.add_user(settings2);

	// player3 will start as the big blind
	let id3 = uuid::Uuid::new_v4();
        let name3 = "Big".to_string();
        let settings3 = PlayerConfig::new(id3, Some(name3), None);
        game.add_user(settings3);

	// player4 will start as UTG
	let id4 = uuid::Uuid::new_v4();
        let name4 = "UTG".to_string();
        let settings4 = PlayerConfig::new(id4, Some(name4), None);
        game.add_user(settings4);
	// set UTG to have medium money so there is a second side pot	
	game.players[0].as_mut().unwrap().money = 750; 
	
	// flatten to get all the Some() players
	let some_players = game.players.iter().flatten().count();
        assert_eq!(some_players, 4);
        assert!(game.players[0].as_ref().unwrap().human_controlled);
        assert!(game.players[1].as_ref().unwrap().human_controlled);
        assert!(game.players[2].as_ref().unwrap().human_controlled);
        assert!(game.players[3].as_ref().unwrap().human_controlled);			
	
	let incoming_actions = Arc::new(Mutex::new(HashMap::<Uuid, PlayerAction>::new()));	
	let incoming_meta_actions = Arc::new(Mutex::new(VecDeque::<MetaAction>::new()));

	let cloned_actions = incoming_actions.clone();	    
	let cloned_meta_actions = incoming_meta_actions.clone();
	let handler = thread::spawn( move || {
	    game.play_one_hand(&cloned_actions, &cloned_meta_actions);
	    game // return the game back
	});

	// UTG goes all in with the medium stack
	incoming_actions.lock().unwrap().insert(id1, PlayerAction::Bet(750));
	// the button calls (and thus goes all in with the short stack)
	incoming_actions.lock().unwrap().insert(id1, PlayerAction::Call);
	// the small blind goes all in with a full stack
	incoming_actions.lock().unwrap().insert(id2, PlayerAction::Bet(1000));	
	// the big blind calls the full all-in
	incoming_actions.lock().unwrap().insert(id3, PlayerAction::Call);
	
	// get the game back from the thread
	let game = handler.join().unwrap();

	// the button won the side pot
	assert_eq!(game.players[0].as_ref().unwrap().money, 2000);

	// the small blind won the remainder
	assert_eq!(game.players[1].as_ref().unwrap().money, 500);
	
	// the big blind lost everything
	assert_eq!(game.players[1].as_ref().unwrap().money, 0);
	
	// UTG won the second side pot
	assert_eq!(game.players[4].as_ref().unwrap().money, 750);	
    }
    
    
}


