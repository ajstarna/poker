use std::cmp;

use super::card::Card;
use super::hand_analysis::{HandRanking, HandResult};
use super::player::{Player, PlayerAction};
use super::game_hand::{GameHand, Street};

use super::random::{RngGenerator};

use rand::Rng;

#[derive(Debug)]
enum BotActionError {
    NoHoleCards,
    NoIndexSet,
}



/// given a player and gamehand, this function returns a player action depending on the state of the game
pub fn get_bot_action<T>(
    player: &Player,
    gamehand: &GameHand,
    rng_generator: Box<dyn RngGenerator<T>>
) -> PlayerAction
where
    T: rand::distributions::uniform::SampleUniform + std::cmp::PartialOrd
{
    match gamehand.street {
        Street::Preflop => {
	    let blah = get_preflop_action(player, gamehand); //
	    blah.unwrap_or(PlayerAction::Fold)
        }
        Street::Flop => {
	    get_post_flop_action(player, gamehand).unwrap_or(PlayerAction::Fold)
        }
        Street::Turn => {
	    get_post_flop_action(player, gamehand).unwrap_or(PlayerAction::Fold)	    
        }
        Street::River => {
	    get_post_flop_action(player, gamehand).unwrap_or(PlayerAction::Fold)	    
        }
        Street::ShowDown => {
	    // this should never happen
	    get_random_action(player, gamehand)	    
	}
    }
}

/*
https://www.thepokerbank.com/strategy/basic/starting-hand-selection/chen-formula/
The Chen Formula
*/
/// given two hole cards, return the Chen Formula score
fn score_preflop_hand(hole_cards: &Vec<Card>) -> Result<f32, BotActionError>  {
    if hole_cards.len() != 2 {
	// something is weird
	return Err(BotActionError::NoHoleCards);
    }
    let rank1 = hole_cards[0].rank as u8;
    let rank2 = hole_cards[1].rank as u8;
    let higher = cmp::max(rank1, rank2);
    let lower = cmp::min(rank1, rank2);

    let mut score: f32 = match higher {
	14 => 10.0, // ace
	13 => 8.0, // king
	12 => 7.0, // queen
	11 => 6.0, // jack
	_ => higher as f32 / 2.0
    };
    if higher == lower {
	// a pair
	score *= 2.0;
    }

    if hole_cards[0].suit == hole_cards[1].suit {
	// suited
	score += 2.0;
    }
    
    let diff = higher - lower;
    let punishment = match diff {
	0..=1 => 0.0, // no gap
	2 => 1.0, // 1 card gap
	3 => 2.0,
	4 => 4.0,
	_ => 5.0, // 4 card gap or more (also including A2, A3, etc)
    };
    score -= punishment;

    if (diff == 1 || diff == 2) && higher < 12 {
	// 1 or 0 gapper both less than Queen
	score += 1.0;
    }
    
    // round up
    Ok(score.ceil())
}

#[derive(Debug)]
enum HandQuality {
    Garbage,
    Mediocre,
    Good,
    Great,
    Exceptional
}

// returns a hand quality enum (for use by bots)
fn qualify_hand(player: &Player, hand_result: &HandResult, gamehand: &GameHand) -> HandQuality {
    let top_rank = gamehand.highest_rank().unwrap();
    match hand_result.hand_ranking {
	HandRanking::HighCard => HandQuality::Garbage,
	HandRanking::Pair => {
	    println!("we have a pair with {:?} and top rank = {:?}",
		     hand_result.constituent_cards[0].rank,
		     top_rank);
	    if hand_result.constituent_cards[0].rank >= top_rank {	
		// top pair or better	
		HandQuality::Good
	    } else {
		// not a great pair
		HandQuality::Mediocre		
	    }
	}
	HandRanking::TwoPair => HandQuality::Good,
	HandRanking::ThreeOfAKind => {
	    if player.hole_cards[0].rank == player.hole_cards[1].rank {
		// we have a set
		HandQuality::Great
	    } else {
		// just trips
		HandQuality::Good
	    }
	},
	HandRanking::Straight | HandRanking::Flush => HandQuality::Great,
	HandRanking::FullHouse | HandRanking::FourOfAKind
	    | HandRanking::StraightFlush | HandRanking::RoyalFlush => HandQuality::Exceptional,	    
	}
	
    }


fn get_random_action(player: &Player, _gamehand: &GameHand) -> PlayerAction {
    let num = rand::thread_rng().gen_range(0..100);
    match num {
        0..=20 => PlayerAction::Fold,
        21..=55 => PlayerAction::Check,
        56..=70 => {
            let amount: u32 = if player.money <= 100 {
                // just go all in if we are at 10% starting

                player.money
            } else {
                rand::thread_rng().gen_range(1..player.money / 2_u32)
            };
            PlayerAction::Bet(amount)
        }
        _ => PlayerAction::Call
    }
}

fn get_garbage_action(
    player: &Player,
    gamehand: &GameHand,    
    cannot_check: bool,
    facing_raise: bool,
    bet_size: u32,
) -> PlayerAction {
    let num = rand::thread_rng().gen_range(0..100);
    let bet_ratio = gamehand.current_bet as f32 / gamehand.total_money() as f32;
    
    if facing_raise
	&& ( ( bet_ratio < 0.25 && gamehand.street == Street::Flop) 
	|| ( bet_ratio <  0.20) ){
	    // don't be weak to mini bets
	    println!("tyring to mini bet me!");
	    match num {
		0..=80 => PlayerAction::Call,
		81..=90 => PlayerAction::Fold,
		_ => {
		    let amount: u32 = std::cmp::min(player.money, bet_size);
		    PlayerAction::Bet(amount)
		}
	    }
	} else {
	    match num {
		0..=90 => {
		    if cannot_check {		    
			PlayerAction::Fold
		    } else {
			PlayerAction::Check			
		    }
		},
		_ => {
		    if gamehand.street == Street::Preflop {
			// if preflop, just fold garbage
			PlayerAction::Fold			
		    } else {
			// post flop, throw in a rare garbage bluff
			let amount: u32 = std::cmp::min(player.money, bet_size);
			PlayerAction::Bet(amount)
		    }
		}
	    }	   
	}    
}

fn get_mediocre_action(
    player: &Player,
    gamehand: &GameHand,    
    cannot_check: bool,
    facing_raise: bool,
    bet_size: u32,
) -> PlayerAction {
    let num = rand::thread_rng().gen_range(0..100);
    let bot_contribution = gamehand.get_current_contributions_for_index(player.index.unwrap());        
    if facing_raise {
	println!("facing a raise");
	match num {
            0..=50 => {
		PlayerAction::Call
	    },
            51..=90 => {
		let bet_ratio = gamehand.current_bet as f32 / gamehand.total_money() as f32;
		println!("bet_ratio = {:?}", bet_ratio);
		if ( bet_ratio < 0.30
		     && gamehand.street == Street::Flop )
		    || ( bet_ratio < 0.25 ) {
			println!("tyring to mini bet me!");			
			PlayerAction::Call
		    } else if bot_contribution > 0 && bet_ratio < 0.75 {
			// we already put some money in, so don't then cave so easy
			println!("lets defend our mediocre money");
			PlayerAction::Call
		    } else {
			PlayerAction::Fold
		    }
	    }
	    _ => {
		let amount: u32 = std::cmp::min(player.money, bet_size);
		PlayerAction::Bet(amount)
	    }
	}
    } else if gamehand.street == Street::Preflop {
	// dont limp preflop
	let amount: u32 = std::cmp::min(player.money, bet_size);
	PlayerAction::Bet(amount)		
	
    } else {
	println!("NOT facing a raise");	
	match num {
            0..=80 => {
		if cannot_check {
		    PlayerAction::Call
		} else {
		    PlayerAction::Check		    
		}
	    },
	    _ =>  {
		let amount: u32 = std::cmp::min(player.money, bet_size);
		PlayerAction::Bet(amount)		
	    }
	}	
    }
}

fn get_good_action(
    player: &Player,
    gamehand: &GameHand,    
    cannot_check: bool,
    facing_raise: bool,
    bet_size: u32,    
) -> PlayerAction {
    let num = rand::thread_rng().gen_range(0..100);
    if facing_raise {
	match num {
	    0..=75 => {
		println!("good hand just call");
		PlayerAction::Call		
	    }
	    _ => {
		println!("good hand rare raise");		
		let amount: u32 = std::cmp::min(player.money, bet_size);
		PlayerAction::Bet(amount)
	    }
	}
    } else {
	match num {
	    0..=80 => {
		println!("good hand bet");
		let amount: u32 = std::cmp::min(player.money, bet_size);
		PlayerAction::Bet(amount)
	    }
	    _ => {
		if cannot_check {
		    // not sure this can really happen after preflop
		    let amount: u32 = std::cmp::min(player.money, bet_size);
		    PlayerAction::Bet(amount)
		} else {
		    println!("good hand rare check");		
		    PlayerAction::Check
		}
	    }
	}
    }
}

fn get_big_action(
    player: &Player,
    gamehand: &GameHand,    
    cannot_check: bool,
    facing_raise: bool,
    bet_size: u32,    
) -> PlayerAction {
    let num = rand::thread_rng().gen_range(0..100);
    
    match num {
	0..=95 => {
	    println!("big hand and we rolled a {num}");
	    let amount: u32 = std::cmp::min(player.money, bet_size);
	    PlayerAction::Bet(amount)
	}
	_ => {
	    if facing_raise || cannot_check {
		println!("big hand the rare call");		
		PlayerAction::Call
	    } else {
		println!("big hand the rare check");		
		PlayerAction::Check		
	    }
	}
    }
}

fn get_draw_action(
    player: &Player,
    gamehand: &GameHand,    
    cannot_check: bool,
    facing_raise: bool,
    bet_size: u32,
) -> PlayerAction {
    let num = rand::thread_rng().gen_range(0..100);
    if facing_raise {
	match num {
            0..=50 => PlayerAction::Call,
            51..=85 => PlayerAction::Fold,
	    _ => {
		let amount: u32 = std::cmp::min(player.money, bet_size);
		PlayerAction::Bet(amount)
            }
	}
    } else {
	println!("NOT facing a raise");	
	match num {
            0..=30 => {
		if cannot_check {
		    // note sure this makes any sense post flop?
		    PlayerAction::Call
		} else {
		    PlayerAction::Check		    
		}
	    },
	    _ =>  {
		let amount: u32 = std::cmp::min(player.money, bet_size);
		PlayerAction::Bet(amount)		
	    }
	}	
    }
}

fn get_preflop_action(player: &Player, gamehand: &GameHand) -> Result<PlayerAction, BotActionError> {
    if player.index.is_none(){
	return Err(BotActionError::NoIndexSet);
    }
    let score = score_preflop_hand(&player.hole_cards)?;
    println!("inside preflop. score = {:?} with hole cards = {:?}", score, &player.hole_cards);
    let bot_contribution = gamehand.get_current_contributions_for_index(player.index.unwrap());    
    let cannot_check = bot_contribution < gamehand.current_bet;
    let facing_raise = gamehand.current_bet > gamehand.big_blind;
    let bet_size = 3 * gamehand.current_bet;
    match score as i64 {
	// TODO: need to consider number of players and position
	-1..=5 => {
	    // setting the number at 5 to be more fun to play against. maybe 6 or 7 is better poker
	    if cannot_check {
		Ok(PlayerAction::Fold)	    
	    } else {
		Ok(PlayerAction::Check)
	    }
	}
	6..=8 => {
	    println!("about to get a mediocre action");
	    Ok(get_mediocre_action(player, gamehand, cannot_check, facing_raise, bet_size))		
	}
	9 => {
	    println!("about to get a good action");
	    Ok(get_good_action(player, gamehand, cannot_check, facing_raise, bet_size))		
	}
	_ => {
	    println!("about to get a big action");
	    Ok(get_big_action(player, gamehand, cannot_check, facing_raise, bet_size))		
	}
    }	   
}

fn get_post_flop_action(player: &Player, gamehand: &GameHand) -> Result<PlayerAction, BotActionError> {

    // TODO the bots need to know that a board pair doesn't contribute to their hand quality
    
    if player.index.is_none(){
	return Err(BotActionError::NoIndexSet);
    }
    let best_hand = player.determine_best_hand(gamehand).unwrap();
    println!("inside flop action. best hand = {:?}", best_hand);
    let quality = qualify_hand(player, &best_hand, gamehand);
    let draw_type = player.determine_draw_type(gamehand);
    let bot_contribution = gamehand.get_current_contributions_for_index(player.index.unwrap());    
    let cannot_check = bot_contribution < gamehand.current_bet;
    let facing_raise = gamehand.current_bet > 0;
    let bet_size = gamehand.total_money() / 2;

    if draw_type.is_some() {
	println!("about to get a draw action");
	Ok(get_draw_action(player, gamehand, cannot_check, facing_raise, bet_size))
    }   
    else {
	match quality {
	    // TODO: need to consider number of players and position
	    HandQuality::Garbage => {
		println!("about to get garbage action");				
		Ok(get_garbage_action(player, gamehand, cannot_check, facing_raise, bet_size))		
	    }
	    HandQuality::Mediocre => {
		println!("about to get a mediocre action");		
		Ok(get_mediocre_action(player, gamehand, cannot_check, facing_raise, bet_size))
	    }
	    HandQuality::Good => {
		println!("about to get a good action");		
		Ok(get_good_action(player, gamehand, cannot_check, facing_raise, bet_size))
	    }
	    HandQuality::Great => {
		println!("about to get a great action");				
		Ok(get_big_action(player, gamehand, cannot_check, facing_raise, bet_size))
	    }
	    HandQuality::Exceptional => {
		println!("about to get an exceptional action");						
		Ok(get_big_action(player, gamehand, cannot_check, facing_raise, bet_size))
	    }
	    
	}}	    
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::card::{Card, Rank, Suit};

    #[test]
    fn score_pair_aces() {
	let mut hole_cards = Vec::<Card>::new();
        hole_cards.push(Card {
            rank: Rank::Ace,
            suit: Suit::Club,
        });
	
        hole_cards.push(Card {
            rank: Rank::Ace,
            suit: Suit::Heart,
        });
	let score = score_preflop_hand(&hole_cards);
        assert_eq!(score.unwrap(), 20.0);	
    }

    #[test]
    fn score_ace_king_suited() {
	let mut hole_cards = Vec::<Card>::new();
        hole_cards.push(Card {
            rank: Rank::Ace,
            suit: Suit::Club,
        });
	
        hole_cards.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
	let score = score_preflop_hand(&hole_cards);
        assert_eq!(score.unwrap(), 12.0);	
    }
    
    #[test]
    fn score_pair_jacks() {
	let mut hole_cards = Vec::<Card>::new();
        hole_cards.push(Card {
            rank: Rank::Jack,
            suit: Suit::Club,
        });
	
        hole_cards.push(Card {
            rank: Rank::Jack,
            suit: Suit::Heart,
        });
	let score = score_preflop_hand(&hole_cards);
        assert_eq!(score.unwrap(), 12.0);	
    }

    /// 3.5 for the 7
    /// suited +2 points
    /// 1 card gap -1
    /// 0-1 card gap both below Queen +1
    /// round up
    #[test]
    fn score_suited_gapper() {
	let mut hole_cards = Vec::<Card>::new();
        hole_cards.push(Card {
            rank: Rank::Five,
            suit: Suit::Heart,
        });
	
        hole_cards.push(Card {
            rank: Rank::Seven,
            suit: Suit::Heart,
        });
	let score = score_preflop_hand(&hole_cards);
        assert_eq!(score.unwrap(), 6.0);	
    }

    #[test]
    fn score_seven_two_off() {
	let mut hole_cards = Vec::<Card>::new();
        hole_cards.push(Card {
            rank: Rank::Two,
            suit: Suit::Heart,
        });
	
        hole_cards.push(Card {
            rank: Rank::Seven,
            suit: Suit::Club,
        });
	let score = score_preflop_hand(&hole_cards);
        assert_eq!(score.unwrap(), -1.0);	
    }
    
    /// if a bot has a garbage hand, then they will check if possible
    #[test]
    fn check_garbage_preflop() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;	
        bot0.hole_cards.push(Card {
            rank: Rank::Two,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Seven,
            suit: Suit::Heart,
        });

	let index = 0;
	bot0.index = Some(index);
	
        let mut gamehand = GameHand::new(2);
	// the bot contributes 2 dollars and is not all in
	gamehand.contribute(index, bot0.id, 2, false);
	gamehand.current_bet = 2; // the current bet of the hand is also 2 dollars
	
	let action = get_bot_action(&bot0, &gamehand);
	
        assert_eq!(action, PlayerAction::Check);
    }
    /// if a bot has a garbage hand, then they will fold preflop at the slightest aggression
    #[test]
    fn fold_garbage_preflop() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::Two,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Seven,
            suit: Suit::Heart,
        });

	let index = 0;
	bot0.index = Some(index);
	    
        let mut gamehand = GameHand::new(2);
	// the bot contributes 2 dollars and is not all in
	gamehand.contribute(index, bot0.id, 2, false);

	// the current bet of the hand is a bit higher, i.e. facing a bet	
	gamehand.current_bet = 3; 
	
	let action = get_bot_action(&bot0, &gamehand);
	
        assert_eq!(action, PlayerAction::Fold);
    }

    /// if a bot has a garbage hand, then they will check if possible
    #[test]
    fn check_garbage_flop() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Queen,
            suit: Suit::Heart,
        });

	let index = 0;
	bot0.index = Some(index);
	
        let mut gamehand = GameHand::new(2);

	// flop really bad for the bot
	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Three,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Five,
		suit: Suit::Diamond,
            }
	]);
	
	let action = get_bot_action(&bot0, &gamehand);
	
        assert_eq!(action, PlayerAction::Check);
    }
    
    /// if a bot has a garbage hand, then they will fold at the slightest aggression
    #[test]
    fn fold_garbage_flop() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Queen,
            suit: Suit::Heart,
        });

	let index = 0;
	bot0.index = Some(index);
	    
        let mut gamehand = GameHand::new(2);
	
	// the current bet of the hand is a bit higher, i.e. facing a bet	
	gamehand.current_bet = 1;

	// flop really bad for the bot	
	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Three,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Five,
		suit: Suit::Diamond,
            }
	]);
	let action = get_bot_action(&bot0, &gamehand);
	
        assert_eq!(action, PlayerAction::Fold);
    }

    /// if the flop bet is small enough, then we wont fold garbage
    #[test]
    fn dont_fold_garbage_flop_if_bet_small() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Queen,
            suit: Suit::Heart,
        });

	let index = 0;
	bot0.index = Some(index);
	    
        let mut gamehand = GameHand::new(2);
	gamehand.street = Street::Flop;
	
	// 40 bucks already in
	gamehand.contribute(index, bot0.id, 40, false);
	
	// the current bet of the hand is a bit higher, i.e. facing a bet
	// but this bet is too small to fold to
	gamehand.current_bet = 6;
	
	// flop really bad for the bot	
	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Three,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Five,
		suit: Suit::Diamond,
            }
	]);
	let action = get_bot_action(&bot0, &gamehand);
        assert!(action != PlayerAction::Fold);
    }
    

    /// if a bot has a mediocre hand, then they will check the flop
    #[test]
    fn check_mediocre_flop() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Queen,
            suit: Suit::Heart,
        });

	let index = 0;
	bot0.index = Some(index);
	    
        let mut gamehand = GameHand::new(2);
	
	// the current bet of the hand is a bit higher, i.e. facing a bet	
	gamehand.current_bet = 1;

	// flop gives us pair of Kings, but not top pair (Ace)
	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Three,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::King,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Ace,
		suit: Suit::Diamond,
            }
	]);
	let action = get_bot_action(&bot0, &gamehand);
	
        assert_eq!(action, PlayerAction::Fold);
    }

    /// if a bot has a good hand, then they will call aggression
    #[test]
    fn call_good_flop() {
        let mut bot0 = Player::new_bot(200);
	bot0.is_active = true;
        bot0.hole_cards.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Queen,
            suit: Suit::Heart,
        });

	let index = 0;
	bot0.index = Some(index);
	    
        let mut gamehand = GameHand::new(2);

	gamehand.current_bet = 1;	

	// flop gives us pair of Kings, which is top pair, so we can call
	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Three,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::King,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Diamond,
            }
	]);
	let action = get_bot_action(&bot0, &gamehand);
	
        assert_eq!(action, PlayerAction::Call);
    }
    
}
