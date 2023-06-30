use std::cmp;

use super::card::Card;
use super::hand_analysis::{HandRanking, HandResult};
use super::player::{Player, PlayerAction};
use super::game_hand::{GameHand, Street};

use rand::Rng;

#[derive(Debug)]
enum BotActionError {
    NoHoleCards,
    NoIndexSet,
}


/// given a player and gamehand, this function returns a player action depending on the state of the game
pub fn get_bot_action(player: &Player, gamehand: &GameHand, players: &[Option<Player>; 9]) -> PlayerAction {
    match gamehand.street {
        Street::Preflop => {
	    let blah = get_preflop_action(player, gamehand, players); //
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
	    get_post_flop_action(player, gamehand).unwrap_or(PlayerAction::Fold)	    	    
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

	// the minimum score for a pair is 5
	if score < 5.0 {
	    score = 5.0;
	}
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

#[derive(Debug, PartialEq)]
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
    let mut on_the_board = true; // if a hand result came from board cards and not hole cards
    for used_card in &hand_result.constituent_cards {
	for hole_card in &player.hole_cards {
	    if used_card == hole_card {
		on_the_board = false;
		break;
	    }
	}
    }
    match hand_result.hand_ranking {
	HandRanking::HighCard => HandQuality::Garbage,
	HandRanking::Pair => {
	    println!("we have a pair with {:?} and top rank = {:?}",
		     hand_result.constituent_cards[0].rank,
		     top_rank);
	    if on_the_board {
		println!("this is a board pair");
		HandQuality::Garbage		
	    } else if hand_result.constituent_cards[0].rank >= top_rank {	
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

fn get_garbage_action(
    player: &Player,
    gamehand: &GameHand,    
    cannot_check: bool,
    facing_raise: bool,
    bet_size: u32,
) -> PlayerAction {
    let num = rand::thread_rng().gen_range(0..100);
    let bet_ratio = gamehand.current_bet as f32 / gamehand.total_money() as f32;
    let current_num_bets = gamehand.get_current_num_bets(); // how many bets this street
    let draw_analysis = player.determine_draw_analysis(gamehand);
    
    if current_num_bets > 2 {
	if gamehand.street == Street::Flop && draw_analysis.good_draw {
	    // it is the flop and we have a good draw, so we can call
	    println!("many  bets with this garbage hand, BUT we have a good draw {:?}", draw_analysis);	    
	    PlayerAction::Call
	} else {
	    PlayerAction::Fold
	}
    }
    else { if facing_raise
	&& ( ( bet_ratio < 0.25 && gamehand.street == Street::Flop) 
	|| ( bet_ratio <  0.20) ){
	    // don't be weak to mini bets
	    println!("tyring to mini bet me!");
	    match num {
		0..=80 => PlayerAction::Call,
		81..=90 =>
		{
		    if draw_analysis.good_draw || draw_analysis.weak_draw {
			PlayerAction::Call			
		    } else {
			PlayerAction::Fold
		    }
		},
		_ => {
		    let amount: u32 = std::cmp::min(player.money, bet_size);
		    PlayerAction::Bet(amount)
		}
	    }
	} else {
	    match num {
		0..=90 => {
		    if cannot_check {
			if draw_analysis.good_draw ||
			    (draw_analysis.weak_draw && gamehand.street == Street::Flop) {
				println!("we got a draw going so call");
				println!("{:?}", draw_analysis);
			    PlayerAction::Call			
			} else {
			    PlayerAction::Fold
			}
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
	}}
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
    let current_num_bets = gamehand.get_current_num_bets(); // how many bets this street
    let draw_analysis = player.determine_draw_analysis(gamehand);    
    if facing_raise {
	println!("facing a raise");	
	if current_num_bets > 3 {
	    // too many bets this street, time to back out
	    if draw_analysis.good_draw {
		PlayerAction::Call		
	    } else {
		println!("too many bets for my mediocre hand. time to leave");		    
		PlayerAction::Fold
	    }
	} else { match num {
            0..=50 => {
		println!("poo");
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
		    } else if bot_contribution > 0 && bet_ratio < 0.80 {
			// we already put some money in, so don't then cave so easy
			println!("lets defend our mediocre money");
			PlayerAction::Call
		    } else if draw_analysis.good_draw ||
		    (draw_analysis.weak_draw && gamehand.street == Street::Flop) {
			PlayerAction::Call
		    } else {
			PlayerAction::Fold
		    }
	    }
	    _ => {
		println!("the rare mediocre re-raise");
		let amount: u32 = std::cmp::min(player.money, bet_size);
		PlayerAction::Bet(amount)
	    }
	}}
    } else if current_num_bets > 2 {
	// too many bets this street, time to back out
	if draw_analysis.good_draw {
	    println!("we CAN call with a good draw");
	    PlayerAction::Call		
	} else {
	    println!("already been a 3-bet, so lets just get out");
	    PlayerAction::Fold
	}
    } else if gamehand.street == Street::Preflop {
	// dont limp preflop
	let amount: u32 = std::cmp::min(player.money, bet_size);
	PlayerAction::Bet(amount)		
	
    } else {
	println!("NOT facing a raise");
	match num {
            0..=80 => {
		if draw_analysis.good_draw ||
		    (draw_analysis.weak_draw && gamehand.street == Street::Flop) {
			println!("lets bet our draw with a mediocre hand!");
			let amount: u32 = std::cmp::min(player.money, bet_size);
			PlayerAction::Bet(amount)		
		    } else if cannot_check {
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
    let current_num_bets = gamehand.get_current_num_bets(); // how many bets this street
    let draw_analysis = player.determine_draw_analysis(gamehand);        
    if facing_raise {
	match num {
	    0..=80 => {
		if current_num_bets > 3 && !draw_analysis.good_draw {
		    // too many bets this street, time to back out
		    println!("too many bets for my good hand. time to leave");		    
		    PlayerAction::Fold
		} else {
		    println!("good hand just call");
		    PlayerAction::Call
		}
	    }
	    _ => {
		if current_num_bets > 3 {
		    // too many bets this street, time to back out
		    println!("many bets for my good hand. just call");		    
		    PlayerAction::Call
		} else {
		    println!("good hand rare raise");		
		    let amount: u32 = std::cmp::min(player.money, bet_size);
		    PlayerAction::Bet(amount)
		}
	    }
	}
    } else {
	if current_num_bets > 3 && !draw_analysis.good_draw {
	    // too many bets this street, time to back out
	    println!("too many bets for my good hand. time to leave");		    
	    PlayerAction::Fold
	} else {match num {
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
	}}
    }
}

fn get_big_action(
    player: &Player,
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

fn get_preflop_action(player: &Player, gamehand: &GameHand, players: &[Option<Player>; 9])
		      -> Result<PlayerAction, BotActionError> {
    if player.index.is_none(){
	return Err(BotActionError::NoIndexSet);
    }
    let score = score_preflop_hand(&player.hole_cards)?;
    println!("inside preflop. score = {:?} with hole cards = {:?}", score, &player.hole_cards);
    let bot_contribution = gamehand.get_current_contributions_for_index(player.index.unwrap());    
    let cannot_check = bot_contribution < gamehand.current_bet;
    let facing_raise = gamehand.current_bet > gamehand.big_blind;
    let bet_size = std::cmp::max(3 * gamehand.current_bet, gamehand.min_raise);
    let	(num_active, _, _) = gamehand.count_player_categories(players);
    let looser_play = {
	// in a small table or if we are
	if num_active < 5 && !facing_raise {
	    // there are at most 4 active players left in the hand, and there
	    // has not been a raise yet, i.e. we are the button or cutoff,
	    // so be a bit looser to steal
	    println!("num active = {:?}", num_active);
	    println!("looser player");
	    true
	} else if gamehand.num_starting_players < 5 {
	    // at a small table, can be a bit looser in general
	    println!("num starting = {:?}", gamehand.num_starting_players);
	    println!("looser player");	    
	    true
	} else {
	    false
	}
    };
    if score == 5.0 && looser_play {
	println!("about to get a LOOSE mediocre action");
	Ok(get_mediocre_action(player, gamehand, cannot_check, facing_raise, bet_size))		
    } else { 
	match score as i64 {
	    // TODO: need to consider number of players and position
	    -1..=5  => {
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
		Ok(get_big_action(player, cannot_check, facing_raise, bet_size))		
	    }
	}
    }
}

fn get_post_flop_action(player: &Player, gamehand: &GameHand) -> Result<PlayerAction, BotActionError> {

    if player.index.is_none(){
	return Err(BotActionError::NoIndexSet);
    }
    let best_hand = player.determine_best_hand(gamehand).unwrap();
    println!("inside flop action. best hand = {:?}", best_hand);
    println!("street contributions:\n{:?}", gamehand.street_contributions);
    let quality = qualify_hand(player, &best_hand, gamehand);
    let bot_contribution = gamehand.get_current_contributions_for_index(player.index.unwrap());    
    let cannot_check = bot_contribution < gamehand.current_bet;
    let facing_raise = gamehand.current_bet > 0;
    
    // make sure bet size at least min raise. this check needed when facing a raise
    let bet_size = std::cmp::max(gamehand.total_money() / 2, gamehand.min_raise);    
        
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
	    Ok(get_big_action(player, cannot_check, facing_raise, bet_size))
	}
	HandQuality::Exceptional => {
	    println!("about to get an exceptional action");						
	    Ok(get_big_action(player, cannot_check, facing_raise, bet_size))
	}
    }    
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::card::{Card, Rank, Suit};

    use std::convert::TryInto;

    
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


    fn set_up_game_hand(big_blind: u32, num_players: usize) -> ([Option<Player>; 9], GameHand){
	let mut players = vec![];
	for i in 0..num_players {
            let mut bot = Player::new_bot(200);
	    bot.is_active = true;
	    bot.index = Some(i);
	    players.push(Some(bot));
	}
	for _ in num_players..9 {
	    // pad with Nones to enforce 9 spots at the table
	    players.push(None);
	}

        let gamehand = GameHand::new(big_blind, &players);
	// convert the vec into an array
	(players.try_into().unwrap(), gamehand)
    }
    
    /// if a bot has a garbage hand, then they will check if possible
    #[test]
    fn check_garbage_preflop() {
	let (mut players, mut gamehand) = set_up_game_hand(2, 9);	
	let index = 0;
	let bot0 = players[index].as_mut().unwrap();	    
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
	
	// the bot contributes 2 dollars and is not all in
	gamehand.contribute(index, bot0.id, 2, false, true);
	gamehand.current_bet = 2; // the current bet of the hand is also 2 dollars

	let bot0 = players[index].as_ref().unwrap();			
	let action = get_bot_action(&bot0, &gamehand, &players);
	
        assert_eq!(action, PlayerAction::Check);
    }

    /// if a bot has a garbage hand, then they will fold preflop at the slightest aggression
    #[test]
    fn fold_garbage_preflop() {
	let (mut players, mut gamehand) = set_up_game_hand(2, 9);
	let index = 0;
	let bot0 = players[index].as_mut().unwrap();
        bot0.hole_cards.push(Card {
            rank: Rank::Two,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Seven,
            suit: Suit::Heart,
        });
	    
	// the bot contributes 2 dollars and is not all in
	gamehand.contribute(index, bot0.id, 2, false, true);
	// the current bet of the hand is a bit higher, i.e. facing a bet	
	gamehand.current_bet = 3; 
	let bot0 = players[index].as_ref().unwrap();			
	let action = get_bot_action(&bot0, &gamehand, &players);
	
        assert_eq!(action, PlayerAction::Fold);
    }

    /// if the table is very small, then more hands are considered good enough pre flop
    /// a pair of 3s has a Chen score of 5, so is right on the cusp of playable
    #[test]
    fn one_mans_small_table_garbage_preflop() {
	let table_size = 4;
	let (mut players, gamehand) = set_up_game_hand(2, table_size);
	let index = 0;
	let bot0 = players[index].as_mut().unwrap();
	
        bot0.hole_cards.push(Card {
            rank: Rank::Three,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Three,
            suit: Suit::Heart,
        });

	let score = score_preflop_hand(&bot0.hole_cards).unwrap();
	assert_eq!(score, 5.0); // make sure we are testing the score we think we are
	
	let bot0 = players[index].as_ref().unwrap();			
	let action = get_bot_action(&bot0, &gamehand, &players);

        assert!(matches!(action, PlayerAction::Bet(_)));
    }

    /// if the player is near the end of the table, then more hands are considered good enough pre flop
    /// a pair of 3s has a Chen score of 5, so is right on the cusp of playable
    /// In this test, there are few active players left, so we can be looser
    #[test]
    fn one_mans_cut_off_garbage_preflop() {
	let (mut players, gamehand) = set_up_game_hand(2, 9);
	let index = 0;

	for index in 1..7 {
	    let bot = players[index].as_mut().unwrap();
	    bot.is_active = false; // they folded
	}
	
	let bot0 = players[index].as_mut().unwrap();	
        bot0.hole_cards.push(Card {
            rank: Rank::Three,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Three,
            suit: Suit::Heart,
        });

	let score = score_preflop_hand(&bot0.hole_cards).unwrap();
	assert_eq!(score, 5.0); // make sure we are testing the score we think we are
	
	let bot0 = players[index].as_ref().unwrap();			
	let action = get_bot_action(&bot0, &gamehand, &players);

        assert!(matches!(action, PlayerAction::Bet(_)));
    }
    
    /// if a bot has a garbage hand, then they will fold at most aggression
    #[test]
    fn fold_garbage_flop() {
	let (mut players, mut gamehand) = set_up_game_hand(2, 9);
	let index = 0;
	let bot0 = players[index].as_mut().unwrap();
	
        bot0.hole_cards.push(Card {
            rank: Rank::King,
            suit: Suit::Heart
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Ten,
            suit: Suit::Heart,
        });

	// the current bet of the hand is a bit higher, i.e. facing a bet	
	gamehand.current_bet = 1;

	// flop really bad for the bot	
	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Jack,
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
	let bot0 = players[index].as_ref().unwrap();			
	let action = get_bot_action(&bot0, &gamehand, &players);
	
        assert_eq!(action, PlayerAction::Fold);
    }

    /// if the flop bet is small enough, then we wont fold garbage
    #[test]
    fn dont_fold_garbage_flop_if_bet_small() {
	let (mut players, mut gamehand) = set_up_game_hand(2, 9);
	let index = 0;
	let id = {
	    let bot0 = players[index].as_mut().unwrap();
            bot0.hole_cards.push(Card {
		rank: Rank::King,
		suit: Suit::Club,
            });
	    
            bot0.hole_cards.push(Card {
		rank: Rank::Queen,
		suit: Suit::Heart,
            });
	    bot0.id
	};
	gamehand.street = Street::Flop;
	
	// 40 bucks already in
	gamehand.contribute(index, id, 40, false, true);
	
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
	let bot0 = players[index].as_ref().unwrap();	
	let action = get_bot_action(&bot0, &gamehand, &players);
        assert!(action != PlayerAction::Fold);
    }    

    /// if a bot has a good hand, then they will call aggression
    #[test]
    fn call_good_flop() {
	let (mut players, mut gamehand) = set_up_game_hand(2, 9);
	let index = 0;
	let bot0 = players[index].as_mut().unwrap();
	
        bot0.hole_cards.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Queen,
            suit: Suit::Heart,
        });

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
	let bot0 = players[index].as_ref().unwrap();		
	let action = get_bot_action(&bot0, &gamehand, &players);
	
        assert_eq!(action, PlayerAction::Call);
    }

    /// if a bot has already decided to bet with a mediocre preflop hand (sometimes happens),
    /// then, if it gets 3 bet, it should usually defend that
    /// In this test, bot1 is our hero who gets 3-bet by bot2 and defends
    #[test]
    fn defend_mediocre_bet_preflop() {
	let (mut players, mut gamehand) = set_up_game_hand(2, 9);


	let id0 = players[0].as_ref().unwrap().id;
	let id2 = players[2].as_ref().unwrap().id;
	
	let bot1 = players[1].as_mut().unwrap();
	let id1 = bot1.id;
        bot1.hole_cards.push(Card {
	    rank: Rank::King,
	    suit: Suit::Club,
        });
	
        bot1.hole_cards.push(Card {
	    rank: Rank::Queen,
	    suit: Suit::Heart,
        });
	
    
	// bot0 contributes 2 dollars (assume the blind) and is not all in
	gamehand.contribute(0, id0, 2, false, true);
	gamehand.current_bet = 2; // the current bet of the hand is also 2 dollars

	// bot1, our hero, raises
	gamehand.contribute(1, id1, 6, false, true);
	gamehand.current_bet = 6;
	
	// bot2, 3-bets us
	gamehand.contribute(2, id2, 24, false, true);
	gamehand.current_bet = 24;

	let bot1 = players[1].as_ref().unwrap();		
	// get our hero's action now (assuming bot0 folded)	
	let action = get_bot_action(&bot1, &gamehand, &players);
	
	// the action is usually a call, with a rare re-raise
        assert!(action != PlayerAction::Fold);
    }

    /// if a bot has already decided to bet with a mediocre preflop hand (sometimes happens),
    /// then, if it gets 4-bet, it will lay down
    /// In this test, bot1 is our hero who gets 3-bet by bot2 and defends, then
    /// gets 4-bet by bot2, and lays down
    #[test]
    fn lay_down_mediocre_bet_preflop() {
	let (mut players, mut gamehand) = set_up_game_hand(2, 9);
	
	let id0 = players[0].as_ref().unwrap().id;
	let id2 = players[2].as_ref().unwrap().id;

	let bot1 = players[1].as_mut().unwrap();
	let id1 = bot1.id;
        bot1.hole_cards.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
	
        bot1.hole_cards.push(Card {
            rank: Rank::Queen,
            suit: Suit::Heart,
        });
	
	// bot0 contributes 2 dollars (assume the blind) and is not all in
	gamehand.contribute(0, id0, 2, false, true);
	gamehand.current_bet = 2; // the current bet of the hand is also 2 dollars

	// bot1, our hero, raises
	gamehand.contribute(1, id1, 6, false, true);
	gamehand.current_bet = 6;

	// bot2, 3-bets us
	gamehand.contribute(2, id2, 24, false, true);
	gamehand.current_bet = 24;

	// we defend and call the 3-bet
	let is_raise = false;
	gamehand.contribute(1, id1, 18, false, is_raise);

	// bot2, 4-bets us!!!!
	// note, this isnt that big of a raise, but it is enough for us to get out
	gamehand.contribute(2, id2, 40, false, true);
	gamehand.current_bet = 64;

	let bot1 = players[1].as_ref().unwrap();		
	// get our hero's action now
	let action = get_bot_action(&bot1, &gamehand, &players);
	
	// we folded, too rich for our blood
        assert_eq!(action, PlayerAction::Fold);
    }

    #[test]
    fn qualify_board_pair() {
	let (mut players, mut gamehand) = set_up_game_hand(2, 9);
	let index = 0;
	let bot0 = players[index].as_mut().unwrap();	
        bot0.hole_cards.push(Card {
            rank: Rank::King,
            suit: Suit::Club,
        });
	
        bot0.hole_cards.push(Card {
            rank: Rank::Queen,
            suit: Suit::Heart,
        });

	// flop a pair of 4s on the board
	gamehand.street = Street::Flop;
	gamehand.flop = Some(vec![
	    Card {
		rank: Rank::Three,
		suit: Suit::Diamond,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Heart,
            },
	    Card {
		rank: Rank::Four,
		suit: Suit::Diamond,
            }
	]);
	
	let best_hand = bot0.determine_best_hand(&gamehand).unwrap();
	assert_eq!(best_hand.hand_ranking, HandRanking::Pair);
	let quality = qualify_hand(&bot0, &best_hand, &gamehand);
	assert_eq!(quality, HandQuality::Garbage);
    }    
}
