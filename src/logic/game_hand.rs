use std::fmt;
use std::collections::{HashMap, HashSet};

use super::card::{Card, HandResult};
use super::player::{Player, PlayerConfig};
use super::pots::PotManager;

use json::object;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum Street {
    Preflop,
    Flop,
    Turn,
    River,
    ShowDown,
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
			
#[derive(Debug)]
pub struct GameHand {
    pub street: Street,
    pot_manager: PotManager,
    pub street_contributions: HashMap<Street, [u32; 9]>, // how much a player contributed to the pot during each street
    pub current_bet: u32, // the current street bet at any moment
    pub flop: Option<Vec<Card>>,
    pub turn: Option<Card>,
    pub river: Option<Card>,
    pub index_to_act: Option<usize>,
}

impl GameHand {
    pub fn default() -> Self {
        GameHand {
            street: Street::Preflop,
            pot_manager: PotManager::new(),
            street_contributions: HashMap::new(),
	    current_bet: 0,
            flop: None,
            turn: None,
            river: None,
	    index_to_act: None,
        }
    }

    pub fn pot_repr(&self) -> Vec<u32> {
	self.pot_manager.simple_repr()
    }
    
    pub fn is_showdown(&self) -> bool {
	Street::ShowDown == self.street
    }

    pub fn contribute(&mut self, index: usize, player_id: Uuid, amount: u32, all_in: bool) {
	let current_contributions = self.street_contributions.get_mut(&self.street).unwrap();	
        current_contributions[index] += amount;	
        self.pot_manager.contribute(player_id, amount, all_in);	    
    }
	
    /// The hand is over, so give all money within each pot to the player who deserves it
    /// If we did not get to show down, then there is one active player who deserves all the money.
    /// Otherwise, we need to figure out who has the best hand.
    /// Each pot needs its own calculation
    pub fn divvy_pots(&self,
		  players: &mut [Option<Player>; 9],
		  player_ids_to_configs: &HashMap::<Uuid, PlayerConfig>)
    -> Vec<json::JsonValue> {
        let hand_results: HashMap<Uuid, Option<HandResult>> = players
            .iter()
            .flatten()
            .filter(|player| player_ids_to_configs.contains_key(&player.id)) // make sure still in the game
            .map(|player| (player.id, player.determine_best_hand(self)))
            .collect();
        let is_showdown = self.is_showdown();
        let mut pay_outs: Vec<json::JsonValue> = vec![];	
        println!("hand results = {:?}", hand_results);
	for pot in self.pot_manager.iter() {
	    // for each pot, we determine who should get paid out
	    // a player can only get paid for a pot that they contributed to
	    // so each pot has its own best_hand calculation
            let (best_ids, best_hand, amount) = if is_showdown {
		// if we made it to show down, there are multiple players left, so we need to see who
		// has the best hand.
		println!("Multiple active players made it to showdown!");
		println!("Looking at pot {:?}", pot);
		let mut best_ids = HashSet::<Uuid>::new();
		let mut best_hand: Option<&HandResult> = None;
		for (id, current_opt) in hand_results
		    .iter()
		    .filter(|(id, opt)| pot.is_elligible(&id) && opt.is_some()) {
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
		let amount = (pot.get_money() as f64 / num_winners as f64) as u32;
		(best_ids, best_hand, amount)
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
		(best_ids, best_hand, amount)
            };
	    GameHand::pay_players(&mut pay_outs, players, player_ids_to_configs,
				  best_ids, amount, best_hand, is_showdown);
	    
	}
	pay_outs
    }

    /// iterate through the players, and any with an id in best_ids gets thei money increaed by amount
    /// Moreover, construct a json payout message for each one of these payouts, and add it to the given pay_outs vec
    fn pay_players(
	pay_outs: &mut Vec<json::JsonValue>,
	players: &mut [Option<Player>; 9],
	player_ids_to_configs: &HashMap::<Uuid, PlayerConfig>,
        best_ids: HashSet<Uuid>,
        amount: u32,
        best_hand: Option<&HandResult>,
        is_showdown: bool,
    ) {
        for (i, player_spot) in players.iter_mut().enumerate() {
            if player_spot.is_some() {
                let player = player_spot.as_mut().unwrap();
                if best_ids.contains(&player.id) {
                    // get the name for messages
                    let name: String = if let Some(config) = &player_ids_to_configs.get(&player.id)
                    {
                        config.name.as_ref().unwrap().clone()
                    } else {
                        // it is a bit weird if we made it all the way to the pay stage for a left player
                        "Player who left".to_string()
                    };

                    let mut message = object! {
                        payout: amount,
                        index: i,
                        player_name: name,
                        is_showdown: is_showdown,
                    };
		    
		    if let Some(hand_result) = best_hand {
                        message["hand_result"] = hand_result.hand_ranking_string().into();			
			message["constituent_cards"] = hand_result.constituent_cards_string().into();
			message["kickers"] = hand_result.kickers_string().into();						
		    }
                    println!(
                        "paying out {:?} to {:?}, with hand result = {:?}",
                        amount, player.id, best_hand
                    );
		    if is_showdown {
			let hole_string = format!("{}{}", player.hole_cards[0], player.hole_cards[1]);
                        message["hole_cards"] = hole_string.into();			
		    }
                    pay_outs.push(message);
                    player.pay(amount);
                }
            }
        }
    }
    
}
