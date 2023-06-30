use super::card::{Card, Suit};
use super::hand_analysis::{DrawType, DrawAnalysis, HandResult};
use super::game_hand::GameHand;
use crate::messages::WsMessage;
use actix::prelude::Recipient;
use std::collections::HashMap;
use std::iter;

use std::collections::HashSet;

use std::time::{Duration, Instant};
use uuid::Uuid;
use std::fmt;

/// the player timeout is how long without doing anything (player action, text messages, etc)
/// before we remove them from any game AND the hub.
pub const PLAYER_TIMEOUT: Duration = Duration::from_secs(1800);

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PlayerAction {
    PostSmallBlind(u32),
    PostBigBlind(u32),
    Fold,
    SitOut,    
    Check,
    Bet(u32),
    Call,
    //Raise(u32), // i guess a raise is just a bet really?
}
impl fmt::Display for PlayerAction {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
	let output = match self {
	    Self::PostSmallBlind(amount) => format!("small_blind:{}", amount),
	    Self::PostBigBlind(amount) => format!("big_blind:{}", amount),
	    Self::Fold => "fold".to_owned(),
	    Self::SitOut => "sit out".to_owned(),	    
	    Self::Check => "check".to_owned(),
	    Self::Bet(amount) => format!("bet:{}", amount),
	    Self::Call => "call".to_owned(),
	};
        write!(f, "{}", output)
    }
}

/// this struct holds the player name and recipient address
#[derive(Debug, Clone)]
pub struct PlayerConfig {
    pub id: Uuid,
    pub name: Option<String>,
    pub player_addr: Option<Recipient<WsMessage>>,
    // the heart_beat indicates the last time the player was "active"
    // inside the game, any player action updates the heartbeat, as well as text messages or ImBack
    // If a player times out inside a game, the game returns the config to the game hub and removes the Player
    // The game hub checks on an interval for failed-heart-beat configs in the lobby, and removes them
    // Moreover, the WsPlayerSession also maintains a heartbeat, and on time out, stops itself
    // This should remove all memory of this player and session from the system (unless I missed something lol)
    pub heart_beat: Instant, 
}

impl PlayerConfig {
    pub fn new(id: Uuid, name: Option<String>, player_addr: Option<Recipient<WsMessage>>) -> Self {
        Self {
            id,
            name,
            player_addr,
	    heart_beat: Instant::now(),
        }
    }

    /// given a message, send it to all players in the HashMap that have a Recipient address    
    pub fn send_group_message(message: &str, ids_to_configs: &HashMap<Uuid, PlayerConfig>) {
        for player_config in ids_to_configs.values() {
            if let Some(addr) = &player_config.player_addr {
                addr.do_send(WsMessage(message.to_owned()));
            }
        }
    }

    /// send a given message to one player
    pub fn send_specific_message(
        message: &str,
        id: Uuid,
        ids_to_configs: &HashMap<Uuid, PlayerConfig>,
    ) {
        if let Some(player_config) = ids_to_configs.get(&id) {
            if let Some(addr) = &player_config.player_addr {
                addr.do_send(WsMessage(message.to_owned()));
            }
        }
    }

    /// find a player with the given id, and send a message with their name to their address
    pub fn send_player_name(&self) {
	if let Some(player_addr) = &self.player_addr {
            let mut message = json::object! {
		msg_type: "player_name".to_owned(),
            };
	    if let Some(name) = &self.name {
		message["player_name"] = name.to_owned().into();
	    } else {
		message["player_name"] = json::Null;
	    }
	    player_addr.do_send(WsMessage(message.dump()));
        }
    }
    
    /// find a player with the given id, and set their address to be the given address
    pub fn set_player_address(
	id: Uuid,
	addr: Recipient<WsMessage>,
	ids_to_configs: &mut HashMap<Uuid, PlayerConfig>) {
        if let Some(player_config) = ids_to_configs.get_mut(&id) {
            player_config.player_addr = Some(addr);
        }
    }

    /// returns a bool indicating of the player has done something
    /// within the past PLAYER_TIMEOUT time.
    /// This indicates if a player needs to be removed from a game or the main lobby, so that
    /// a player can't sit in a table indefinitely.
    pub fn has_active_heart_beat(&self) -> bool {
	let gap = Instant::now().duration_since(self.heart_beat);
	if gap > PLAYER_TIMEOUT {
            // heartbeat timed out
            println!("player timed out!");
	    false
	} else {
	    true
	}
	
    }
}

#[derive(Debug, Clone)]
pub struct Player {
    pub id: Uuid,
    pub index: Option<usize>, // index at the table
    pub human_controlled: bool, // do we need user input or let the computer control it
    pub money: u32,
    pub is_active: bool,      // is still playing the current hand
    pub is_sitting_out: bool, // if sitting out, then they are not active for any future hand
    pub hole_cards: Vec<Card>,
    pub last_action: Option<PlayerAction>, // the last thing they did (or None)
}

impl Player {
    pub fn new(id: Uuid, human_controlled: bool, money: u32) -> Self {
        Player {
            id,
	    index: None,
            human_controlled,
            money,
            is_active: false, // a branch new player is not active in a hand
            is_sitting_out: false,
            hole_cards: Vec::<Card>::with_capacity(2),
	    last_action: None,
        }
    }

    /// create a new bot from scratch
    pub fn new_bot(money: u32) -> Self {
        let bot_id = Uuid::new_v4(); // can just gen a new arbitrary id for the bot
        Self::new(bot_id, false, money)
    }

    pub fn pay(&mut self, payment: u32) {
        self.money += payment;
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// If the player has put all their money in, but has not folded (is_active),
    /// then they are all-in
    pub fn is_all_in(&self) -> bool {
        self.is_active && self.money == 0
    }

    /// Given a gamehand,
    /// we need to determine which 5 cards make the best hand for this player
    /// If the player is not active or it is preflop, return None as the optional best hand.
    pub fn determine_best_hand(&self, gamehand: &GameHand) -> Option<HandResult> {
        if !self.is_active {
            // if the player isn't active, then can't have a best hand
            return None;
        }
	if gamehand.is_preflop() {
	    // there is no "best hand" if we didn't even make it to the flop
	    return None;
	}
	// we look at all possible 7 choose 5 (21) hands from the hole cards, flop, turn, river
	let mut best_result: Option<HandResult> = None;
	let mut hand_count = 0;	
	for exclude_idx1 in 0..7 {
	    for exclude_idx2 in exclude_idx1+1..7 {
		let mut possible_hand = Vec::with_capacity(5);
		for (idx, card) in self
		    .hole_cards.iter().map(|c| Some(c))
		    .chain(gamehand.flop.as_ref().unwrap().iter().map(|c| Some(c)))
		    .chain(iter::once(gamehand.turn.as_ref()))
		    .chain(iter::once(gamehand.river.as_ref()))
		    .enumerate()
		{
		    //println!("sup {:?}, card = {:?}", idx, card);
		    if let Some(card) = card {
			if idx != exclude_idx1 && idx != exclude_idx2 {
			    //println!("pushing!");
			    possible_hand.push(*card);
			}
		    }
		}
		if possible_hand.len() != 5 {
		    continue;
		}
		hand_count += 1;		
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
	println!("hand_count = {:?}", hand_count);
	//assert!(hand_count == 21); // 7 choose 5
	best_result
    }


    /*
    pub fn analyze_for_best_hand_and_draw_analysis(&self, gamehand: &GameHand)
						   -> Option<(HandResult, DrawAnalysis)> {
	let best_hand = self.determine_best_hand(gamehand);
	if let Some(hand) = best_hand {
	    let draw_analysis = self.determine_draw_analysis(gamehand);
	    match hand.hand_ranking {
		HandRanking::StraightFlush => {
		    // we actually have a straight flush, so remove straights and flushes from the draws
		    draw_analysis.retain(|&d| d
		}
		HandRanking::Flush => {
		    // we actually have aflush, so remove flushes from the draws
		}
		HandRanking::Straight => {
		    // we actually have a straight, so remove straights from the draws
		}
	    }
	    return Some((hand, draw_analysis));
	}
	None
    }*/
    
    /// returns a tuple of two DrawAnalysis types. the First is for the player, and the
    /// latter is for the board itself (currently only works on the River)
    pub fn determine_draw_analysis(&self, gamehand: &GameHand) -> DrawAnalysis {
	let mut my_draws = HashSet::<DrawType>::new();
	let mut board_draws = HashSet::<DrawType>::new();	
	
        if !self.is_active {
            // if the player isn't active, then can't have a best hand
	    return DrawAnalysis::from_draws(my_draws, board_draws);
        }
	if gamehand.flop.is_none() {
	    // no draws by definition at preflop
	    return DrawAnalysis::from_draws(my_draws, board_draws);	    
	}

	let top_rank = gamehand.highest_rank().unwrap();
	if self.hole_cards[0].rank >= top_rank && self.hole_cards[1].rank >= top_rank {
	    // we have two over cards
	    my_draws.insert(DrawType::TwoOvers);
	}
	    
	for exclude_idx1 in 0..7 {
	    for exclude_idx2 in exclude_idx1+1..7 {
		let mut possible_hand = Vec::with_capacity(5);
		for (idx, card) in self
		    .hole_cards.iter().map(|c| Some(c))
		    .chain(gamehand.flop.as_ref().unwrap().iter().map(|c| Some(c)))
		    .chain(iter::once(gamehand.turn.as_ref()))
		    .chain(iter::once(gamehand.river.as_ref()))
		    .enumerate()
		{
		    if let Some(card) = card {
			if idx != exclude_idx1 && idx != exclude_idx2 {
			    //println!("pushing!");
			    possible_hand.push(*card);
			}
		    }
		}
		if possible_hand.len() < 3 {
		    println!("should this small possible hand even happen");
		    continue;
		}
		// we have built a hand of five cards, now evaluate it
		let current_draws = HandResult::determine_draw_types(possible_hand);
		for draw in current_draws {
                    // dont add flush draws if neither of our hole cards have this suit
                    // (could be smarter and see how many are ours, but this is something at least)
                    if let DrawType::FourToAFlush(suit) = draw {
			if self.hole_cards[0].suit != suit && self.hole_cards[1].suit != suit {
			    board_draws.insert(draw);
                            continue;
			}
                    }
                    if let DrawType::ThreeToAFlush(suit) = draw {
			if self.hole_cards[0].suit != suit && self.hole_cards[1].suit != suit {
			    board_draws.insert(draw);
			    continue;
			}
		    }
		    if exclude_idx1 == 0 && exclude_idx2 == 1 {
			// this draw comes completely from the board
			// i.e. when there are < 5 cards to look at
			board_draws.insert(draw);
		    } else {
			my_draws.insert(draw);
		    }
		}

	    }
	}

	// do some cleaning
	// if we have four to a flush we clearly also have three to a flush, so remove it
	let mut suits_to_prune = HashSet::<Suit>::new();
	for draw in &my_draws {
	    if let DrawType::FourToAFlush(suit) = draw {
		suits_to_prune.insert(*suit);
	    }
	}
	for suit in suits_to_prune {
	    my_draws.remove(&DrawType::ThreeToAFlush(suit));
	}
	if my_draws.contains(&DrawType::OpenEndedStraight) {
	    my_draws.remove(&DrawType::GutshotStraight);
	}
	DrawAnalysis::from_draws(my_draws, board_draws)
    }    
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::card::{Card, Rank, Suit};
    use crate::logic::game_hand::Street;

    #[test]
    fn flop_four_flush_draw() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
	bot0.index = Some(0);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::Five,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Queen,
            suit: Suit::Club,
        });

        let mut gamehand = GameHand::new(2, &[Some(bot0.clone())]);

	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Nine,
		suit: Suit::Club,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Club,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Diamond,
            }
	]);

	let draw_analysis = bot0.determine_draw_analysis(&gamehand);	
	assert_eq!(
	    draw_analysis,
	    DrawAnalysis {
		my_draws: HashSet::from([DrawType::FourToAFlush(Suit::Club)]),
		board_draws: HashSet::from([]),		
		good_draw: true,
		weak_draw: false,
		board_good_draw: false,
		board_weak_draw: false,
		
	    }
	);
    }

    #[test]
    fn turn_four_flush_draw() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
	bot0.index = Some(0);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::Five,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Queen,
            suit: Suit::Club,
        });

        let mut gamehand = GameHand::new(2, &[Some(bot0.clone())]);
	
	gamehand.street = Street::Turn;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Nine,
		suit: Suit::Club,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Spade,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Heart,
            }
	]);
	gamehand.turn = Some(
	    Card {
		rank: Rank::Nine,
		suit: Suit::Club,
            }
	);

	let draw_analysis = bot0.determine_draw_analysis(&gamehand);	
	assert_eq!(
	    draw_analysis,
	    DrawAnalysis {
		my_draws: HashSet::from([DrawType::FourToAFlush(Suit::Club)]),
		board_draws: HashSet::from([]),		
		good_draw: true,
		weak_draw: false,
		board_good_draw: false,
		board_weak_draw: false,
		
	    }
	);
    }

    #[test]
    fn turn_three_flush_draw() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
	bot0.index = Some(0);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::Five,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Queen,
            suit: Suit::Club,
        });

        let mut gamehand = GameHand::new(2, &[Some(bot0.clone())]);
	
	gamehand.street = Street::Turn;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Nine,
		suit: Suit::Spade,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Heart,
            }
	]);
	gamehand.turn = Some(
	    Card {
		rank: Rank::Nine,
		suit: Suit::Club,
            }
	);

	let draw_analysis = bot0.determine_draw_analysis(&gamehand);	
	assert_eq!(
	    draw_analysis,
	    DrawAnalysis {
		my_draws: HashSet::from([DrawType::ThreeToAFlush(Suit::Club)]),
		board_draws: HashSet::from([]),		
		good_draw: false,
		weak_draw: true,
		board_good_draw: false,
		board_weak_draw: false,
		
	    }
	);
    }
    
    #[test]
    fn river_with_actual_flush() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
	bot0.index = Some(0);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Queen,
            suit: Suit::Club,
        });

        let mut gamehand = GameHand::new(2, &[Some(bot0.clone())]);	

	gamehand.street = Street::River;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Three,
		suit: Suit::Club,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Club,
            },
	    Card {
		rank: Rank::Five,
		suit: Suit::Heart,
            }
	]);
	gamehand.turn = Some(
	    Card {
		rank: Rank::Nine,
		suit: Suit::Club,
            }
	);
	gamehand.river = Some(
	    Card {
		rank: Rank::Two,
		suit: Suit::Diamond,
            }
	);
	
	let draw_analysis = bot0.determine_draw_analysis(&gamehand);
	assert_eq!(
	    draw_analysis,
	    DrawAnalysis {
		my_draws: HashSet::from([DrawType::TwoOvers, DrawType::FourToAFlush(Suit::Club)]),
		board_draws: HashSet::from([DrawType::OpenEndedStraight, DrawType::ThreeToAFlush(Suit::Club)]),
		good_draw: true,
		weak_draw: true,
		board_good_draw: true,
		board_weak_draw: false,
		
	    }
	);
    }
    
    #[test]
    fn flop_gutshot_straight_draw() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
	bot0.index = Some(0);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Jack,
            suit: Suit::Club,
        });

        let mut gamehand = GameHand::new(2, &[Some(bot0.clone())]);	
	
	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Ace,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Ten,
		suit: Suit::Spade,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Diamond,
            }
	]);

	let draw_analysis = bot0.determine_draw_analysis(&gamehand);	
	assert_eq!(
	    draw_analysis,
	    DrawAnalysis {
		my_draws: HashSet::from([DrawType::GutshotStraight]),
		board_draws: HashSet::from([]),		
		good_draw: false,
		weak_draw: true,
		board_good_draw: false,
		board_weak_draw: false,
		
	    }
	);
    }

    /// this test has A-4 but we need to make sure we don't think it is open ended
    #[test]
    fn flop_low_ace_gutshot_straight_draw() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
	bot0.index = Some(0);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::Two,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Three,
            suit: Suit::Club,
        });

        let mut gamehand = GameHand::new(2, &[Some(bot0.clone())]);
	
	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Ace,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Ten,
		suit: Suit::Spade,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Diamond,
            }
	]);

	let draw_analysis = bot0.determine_draw_analysis(&gamehand);	
	assert_eq!(
	    draw_analysis,
	    DrawAnalysis {
		my_draws: HashSet::from([DrawType::GutshotStraight]),
		board_draws: HashSet::from([]),		
		good_draw: false,
		weak_draw: true,
		board_good_draw: false,
		board_weak_draw: false,
		
	    }
	);
    }

    /// In this one we are testing that A,3-5 can be seens as a gutshot
    #[test]
    fn flop_low_ace_gutshot_straight_draw_2() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
	bot0.index = Some(0);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::Five,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Three,
            suit: Suit::Club,
        });

        let mut gamehand = GameHand::new(2, &[Some(bot0.clone())]);
	
	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Ace,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Ten,
		suit: Suit::Spade,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Diamond,
            }
	]);

	let draw_analysis = bot0.determine_draw_analysis(&gamehand);	
	assert_eq!(
	    draw_analysis,
	    DrawAnalysis {
		my_draws: HashSet::from([DrawType::GutshotStraight]),
		board_draws: HashSet::from([]),		
		good_draw: false,
		weak_draw: true,
		board_good_draw: false,
		board_weak_draw: false
		
	    }
	);
    }

    /// In this one we are testing that J-A can be seens as a gutshot
    #[test]
    fn flop_low_ace_gutshot_straight_draw_3() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
	bot0.index = Some(0);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::Jack,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Queen,
            suit: Suit::Club,
        });

        let mut gamehand = GameHand::new(2, &[Some(bot0.clone())]);	

	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Ace,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::King,
		suit: Suit::Spade,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Diamond,
            }
	]);

	let draw_analysis = bot0.determine_draw_analysis(&gamehand);	
	assert_eq!(
	    draw_analysis,
	    DrawAnalysis {
		my_draws: HashSet::from([DrawType::GutshotStraight]),
		board_draws: HashSet::from([]),		
		good_draw: false,
		weak_draw: true,
		board_good_draw: false,
		board_weak_draw: false,
		
	    }
	);
    }
    
    #[test]
    fn flop_two_overs_draw() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
	bot0.index = Some(0);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Jack,
            suit: Suit::Club,
        });

        let mut gamehand = GameHand::new(2, &[Some(bot0.clone())]);		

	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Three,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Ten,
		suit: Suit::Spade,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Diamond,
            }
	]);
	
	let draw_analysis = bot0.determine_draw_analysis(&gamehand);	
	assert_eq!(
	    draw_analysis,
	    DrawAnalysis {
		my_draws: HashSet::from([DrawType::TwoOvers]),
		board_draws: HashSet::from([]),		
		good_draw: false,
		weak_draw: true,
		board_good_draw: false,
		board_weak_draw: false,
		
	    }
	);
    }
    
    #[test]
    fn flop_open_ended_straight_draw() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
	bot0.index = Some(0);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::Jack,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Queen,
            suit: Suit::Club,
        });

        let mut gamehand = GameHand::new(2, &[Some(bot0.clone())]);
	
	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::King,
		suit: Suit::Spade,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Ten,
		suit: Suit::Diamond,
            }
	]);
	
	let draw_analysis = bot0.determine_draw_analysis(&gamehand);	
	assert_eq!(
	    draw_analysis,
	    DrawAnalysis {
		my_draws: HashSet::from([DrawType::OpenEndedStraight]),
		board_draws: HashSet::from([]),		
		good_draw: true,
		weak_draw: false,
		board_good_draw: false,
		board_weak_draw: false,
		
	    }
	);
    }

    /// we dont want three of the same suit to count unless we have some in our hole cards
    #[test]
    fn flop_three_flush_for_the_board() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
	bot0.index = Some(0);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::Jack,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Five,
            suit: Suit::Club,
        });

        let mut gamehand = GameHand::new(2, &[Some(bot0.clone())]);

	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::King,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Ten,
		suit: Suit::Diamond,
            }
	]);
	
	let draw_analysis = bot0.determine_draw_analysis(&gamehand);	
	assert_eq!(
	    draw_analysis,
	    DrawAnalysis {
		my_draws: HashSet::from([]),
		board_draws: HashSet::from([DrawType::ThreeToAFlush(Suit::Diamond)]),
		good_draw: false,
		weak_draw: false,
		board_good_draw: false,
		board_weak_draw: true,
		
	    }
	);
    }
    
    #[test]
    fn flop_three_flush_and_open_ended_draws() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
	bot0.index = Some(0);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Ten,
            suit: Suit::Club,
        });

        let mut gamehand = GameHand::new(2, &[Some(bot0.clone())]);

	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Jack,
		suit: Suit::Spade,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Queen,
		suit: Suit::Club,
            }
	]);
	
	let draw_analysis = bot0.determine_draw_analysis(&gamehand);	
	assert_eq!(
	    draw_analysis,
	    DrawAnalysis {
		my_draws: HashSet::from([DrawType::OpenEndedStraight, DrawType::ThreeToAFlush(Suit::Club)]),
		board_draws: HashSet::from([]),		
		good_draw: true,
		weak_draw: true,
		board_good_draw: false,
		board_weak_draw: false,
		
	    }
	);
    }

    #[test]
    fn flop_four_flush_and_gutshot_and_two_overs_draws() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
	bot0.index = Some(0);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Queen,
            suit: Suit::Club,
        });

        let mut gamehand = GameHand::new(2, &[Some(bot0.clone())]);

	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Ten,
		suit: Suit::Spade,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Club,
            },
	    Card {
		rank: Rank::Nine,
		suit: Suit::Club,
            }
	]);
	
	let draw_analysis = bot0.determine_draw_analysis(&gamehand);	
	assert_eq!(
	    draw_analysis,
	    DrawAnalysis {
		my_draws: HashSet::from([DrawType::FourToAFlush(Suit::Club),
					 DrawType::GutshotStraight, DrawType::TwoOvers]),
		board_draws: HashSet::from([]),
		good_draw: true,
		weak_draw: true,
		board_good_draw: false,
		board_weak_draw: false,
	    }
	);
    }
    
}
