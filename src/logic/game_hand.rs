use std::fmt;
use std::collections::{HashMap, HashSet};

use super::card::{Card, Rank};
use super::hand_analysis::HandResult;

use super::player::{Player, PlayerConfig, PlayerAction};
use super::pot::PotManager;

use json::object;
use uuid::Uuid;

use strum_macros::EnumIter;
use strum::IntoEnumIterator;

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone, EnumIter)]
pub enum Street {
    Preflop = 0,
    Flop = 1,
    Turn = 2,
    River = 3,
    ShowDown = 4,
}

impl fmt::Display for Street {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
	let output = match self {
	    Street::Preflop => "preflop".to_owned(),
	    Street::Flop => "flop".to_owned(),
	    Street::Turn => "turn".to_owned(),
	    Street::River => "river".to_owned(),
	    Street::ShowDown => "showdown".to_owned(),
	};
        write!(f, "{}", output)
    }
}

pub enum HandStatus {
    KeepPlaying,
    NextStreet,
    HandOver,
}

#[derive(Debug)]
pub struct GameHand {
    pub big_blind: u32,
    pub num_starting_players: usize,
    pub street: Street,
    pot_manager: PotManager,
    pub street_contributions: HashMap<Street, [u32; 9]>, // how much a player contributed to the pot during each street
    pub street_num_bets: [u32; 5], // how many bets per street. each index is for a given street
    pub last_action: Option<PlayerAction>, // the last thing anyone did (or None)	
    pub current_bet: u32, // the current street bet at any moment
    pub min_raise: u32, // the minimum amount that the next raise must be
    pub flop: Option<Vec<Card>>,
    pub turn: Option<Card>,
    pub river: Option<Card>,
    pub index_to_act: Option<usize>,
}

impl GameHand {

    /// a new() constructor when we know the blind and the starting players
    pub fn new(big_blind: u32, players: &[Option<Player>]) -> Self {
	let mut street_contributions = HashMap::new();
	for street in Street::iter() {
            street_contributions.insert(street, [0;9]);
	}
	let num_starting_players = players.iter().flatten().filter(|p| p.money > 0).count();
        GameHand {
	    big_blind,
	    num_starting_players,
            street: Street::Preflop,
            pot_manager: PotManager::new(),
            street_contributions,
	    street_num_bets: [0;5],
	    last_action: None,
	    current_bet: 0,
	    min_raise: big_blind,
            flop: None,
            turn: None,
            river: None,
	    index_to_act: None,
        }
    }

    /// go through a given list of players, and returns a tuple with:
    /// (num_active, num_settled, num_all_in)
    pub fn count_player_categories(&self, players: &[Option<Player>; 9]) -> (u32, u32, u32) {
	let current_contributions = self.street_contributions.get(&self.street).unwrap();	
	let mut num_active = 0;    
	let mut num_settled = 0;
	let mut num_all_in = 0;	    	    
        for (i, player_spot) in players.iter().enumerate() {		
	    if let Some(player) = player_spot {
		if player.is_active {
		    num_active += 1;
		}
		if player.is_all_in() {
		    num_all_in += 1;
		} else {
		    if let Some(PlayerAction::PostBigBlind(_)) = player.last_action {
			// posting the big blind does not count as being "settled",
			// since they get a chance to raise again.
			continue
		    }
		    if player.last_action.is_some() {
			// players can only be settled if they have dome something this street at least
			let player_cont = current_contributions[i];
			if player_cont >= self.current_bet {
			    num_settled += 1;
			}
		    }
		}
	    }
        }
	(num_active, num_settled, num_all_in)
    }
    
    pub fn get_hand_status(&self, players: &mut [Option<Player>; 9]) -> HandStatus {
	let (num_active, num_settled, num_all_in) = self.count_player_categories(players);	    
        if num_active == 1 {
            println!("Only one active player left so lets end the hand");
            // end the street and indicate to the caller that the hand is finished
	    HandStatus::HandOver
        }
        else if num_settled + num_all_in == num_active {
            println!(
                "everyone is ready to go to the next street! num_settled = {}",
                num_settled
            );
            // indicate to the caller that the hand is going to the next street
	    HandStatus::NextStreet
        } else {
	    // otherwise, there is more action to be had this street
	    HandStatus::KeepPlaying
	}
	
    }
    
    pub fn pot_repr(&self) -> Vec<u32> {
	self.pot_manager.simple_repr()
    }

    pub fn total_money(&self) -> u32 {
	self.pot_manager.total_money()
    }
    
    pub fn is_showdown(&self) -> bool {
	Street::ShowDown == self.street
    }
    
    pub fn is_preflop(&self) -> bool {
	Street::Preflop == self.street
    }

    pub fn get_current_contributions_for_index(&self, index: usize) -> u32 {
	let current_contributions = self.street_contributions.get(&self.street).unwrap();
	current_contributions[index]
    }
    
    pub fn get_current_num_bets(&self)  -> u32 {
	self.street_num_bets[self.street as usize]
    }
    
    pub fn contribute(&mut self, index: usize, player_id: Uuid, amount: u32, all_in: bool, is_raise: bool) {
	let current_contributions = self.street_contributions.get_mut(&self.street).unwrap();	
        current_contributions[index] += amount;
	if is_raise {
	    self.street_num_bets[self.street as usize] += 1;
	}
        self.pot_manager.contribute(player_id, amount, all_in);	    
    }
	
    /// The hand is over, so give all money within each pot to the player who deserves it
    /// If we did not get to show down, then there is one active player who deserves all the money.
    /// Otherwise, we need to figure out who has the best hand.
    /// Each pot needs its own calculation
    /// Returns a list of settlements of the paid (or active at showdown) players.
    /// A settlement shows the payout and hole cards of winning players, OR possibly the hole cards
    /// of losing players (if they had to show in the final reveal order of cards - starting with most aggression)
    pub fn divvy_pots(
	&self,
	players: &mut [Option<Player>; 9],
	player_ids_to_configs: &HashMap::<Uuid, PlayerConfig>,
	starting_idx: usize
    )
    -> Vec<json::JsonValue> {
        let hand_results: HashMap<Uuid, Option<HandResult>> = players
            .iter()
            .flatten()
            .filter(|player| player_ids_to_configs.contains_key(&player.id)) // make sure still in the game
            .map(|player| (player.id, player.determine_best_hand(self)))
            .collect();
	
        let is_showdown = self.is_showdown();
        let mut settlements: Vec<json::JsonValue> = vec![];	
        println!("hand results = {:?}", hand_results);
	let showdown_starting_idx = GameHand::get_showdown_starting_idx(players, starting_idx);
	for (pot_idx, pot) in self.pot_manager.iter().enumerate().filter(|(_, pot)| pot.money > 0) {
	    // for each pot, we determine who should get paid out
	    // a player can only get paid for a pot that they contributed to
	    // so each pot has its own best_hand calculation
            let (best_ids, best_hand, amount, showing_ids, elligible_ids) = if is_showdown {
		// if we made it to show down, there are multiple players left, so we need to see who
		// has the best hand.
		println!("Multiple active players made it to showdown!");
		println!("Looking at pot {:?}", pot);
		let mut best_ids = HashSet::<Uuid>::new(); // who is a winner of the pot
		let mut showing_ids = HashSet::<Uuid>::new(); // who needs to show their cards
		let mut elligible_ids = HashSet::<Uuid>::new(); // who was even in the pot (and should get a settlement)
		let mut best_hand: Option<&HandResult> = None;
		for i in (showdown_starting_idx..9).chain(0..showdown_starting_idx) {
		    if let Some(player) = &mut players[i]  {
			if pot.is_elligible(&player.id) && hand_results.get(&player.id).is_some() {
			    let current_opt = hand_results.get(&player.id).unwrap();
			    if current_opt.is_none() {
				continue;
			    }
			    elligible_ids.insert(player.id); // indicates we looked at them even for this pot
			    let current_result = current_opt.as_ref().unwrap();
			    if best_hand.is_none() || current_result > best_hand.unwrap() {
				println!("new best hand for id {:?}", player.id);
				best_hand = Some(&current_result);
				best_ids.clear();
				best_ids.insert(player.id); // only one best hand now
				showing_ids.insert(player.id); // they need to show since a potential winner at this point
			    } else if current_result == best_hand.unwrap() {
				println!("equally good hand for id {:?}", player.id);
				best_ids.insert(player.id); // another index that also has the best hand
				showing_ids.insert(player.id); // they need to show since a potential winner at this point
			    } else {
				println!("hand worse for id {:?}", player.id);
				continue;
			    }
			}
		    }
		}
		// divy the pot to all the winners
		let num_winners = best_ids.len();
		let amount = (pot.get_money() as f64 / num_winners as f64) as u32;
		(best_ids, best_hand, amount, showing_ids, elligible_ids)
            } else {
		// the hand ended before Showdown, so we simple find the one active player remaining
		let best_ids:  HashSet::<Uuid> = players
		    .iter()
		    .flatten()
		    .filter(|player| player.is_active)
		    .map(|player| player.id).collect();
		// if we didn't make it to show down, there better be only one player left
		assert!(best_ids.len() == 1);
		let best_hand = None;
		    let amount = self.pot_manager.iter().next().unwrap().get_money();
		let showing_ids = best_ids.clone();
		let elligible_ids = best_ids.clone();		
		(best_ids, best_hand, amount, showing_ids, elligible_ids)
            };
	    self.settle_players(&mut settlements, players, player_ids_to_configs, &hand_results, pot_idx,
				     best_ids, best_hand, amount, showing_ids, elligible_ids, showdown_starting_idx);
	    
	}
	settlements
    }

    /// iterate through the players, and any with an id in best_ids gets their money increased by amount.
    /// Moreover, construct a json settlement message for each one of these payouts,
    /// and add it to the given settlements vec (if they need to show)
    fn settle_players(
	&self, 
	settlements: &mut Vec<json::JsonValue>,
	players: &mut [Option<Player>; 9],
	player_ids_to_configs: &HashMap::<Uuid, PlayerConfig>,
	hand_results: &HashMap<Uuid, Option<HandResult>>,	
	pot_idx: usize,
        best_ids: HashSet<Uuid>,
        best_hand: Option<&HandResult>,
        amount: u32,
	showing_ids: HashSet<Uuid>,
	elligible_ids: HashSet<Uuid>,	
	showdown_starting_idx: usize,
    ) {
        let is_showdown = self.is_showdown();
        for i in (showdown_starting_idx..9).chain(0..showdown_starting_idx) {
	    if let Some(player) = &mut players[i]  {
		if !elligible_ids.contains(&player.id) {
                    continue;
		}
		let name: String = if let Some(config) = &player_ids_to_configs.get(&player.id)
		{
		    config.name.as_ref().unwrap().clone()
		} else {
		    // it is a bit weird if we made it all the way to the pay stage for a left player
		    "Player who left".to_string()
		};

		let mut message = object! {
		    index: i,
		    player_name: name,
		    is_showdown: is_showdown,
		    pot_index: pot_idx,
		};
		
		if best_ids.contains(&player.id) {
		    message["winner"] = true.into();		    
		    message["payout"] = amount.into();
		    println!(
			"paying out {:?} to {:?}, with hand result = {:?}",
			amount, player.id, best_hand
		    );
		    player.pay(amount);		    
		} else {
		    message["winner"] = false.into();
		}
		if is_showdown && showing_ids.contains(&player.id) {		    
		    let hole_string = format!("{}{}", player.hole_cards[0], player.hole_cards[1]);
		    message["hole_cards"] = hole_string.into();
		    if let Some(hand_result) = hand_results.get(&player.id).unwrap() {
			message["hand_result"] = hand_result.hand_ranking_string().into();			
			message["constituent_cards"] = hand_result.constituent_cards_string().into();
			message["kickers"] = hand_result.kickers_string().into();
		    }
		    
		}
		settlements.push(message);
            }
        }
    }

    pub fn highest_rank(&self) -> Option<Rank> {
	if self.flop.is_none() {
	    // no cards present yet
	    return None;
	}
	let high = self.flop.as_ref().unwrap().iter().map(|c| Some(c))
	    .chain(std::iter::once(self.turn.as_ref()))
	    .chain(std::iter::once(self.river.as_ref())).flatten().map(|c| c.rank).max();
	high
    }
    
    // determine where to start the showing of cards
    // if there is a last-aggressor, then it starts with them,
    // otherwise, it defaults to the street starting idx    
    fn get_showdown_starting_idx(
	players: &mut [Option<Player>; 9],
	starting_idx: usize,
    ) -> usize {
	let mut showdown_starting_idx = starting_idx;
        for (i, player_spot) in players.iter_mut().enumerate() {
	    if player_spot.is_some() {
                let player = player_spot.as_mut().unwrap();
		if let Some(PlayerAction::Bet(_)) = player.last_action {
		    // if the player's last action was a bet, then they we the last aggressor,
		    // and hence has to show first
		    showdown_starting_idx = i;
		    break;
		}
	    }
	}
	showdown_starting_idx
    }
}


