use std::cmp;

use super::card::{Card};
use super::player::{Player, PlayerAction};
use super::game_hand::{GameHand, Street};

use rand::Rng;

#[derive(Debug)]
enum BotActionError {
    NoHoleCards,
    NoIndexSet,
}

/*
https://www.thepokerbank.com/strategy/basic/starting-hand-selection/chen-formula/
The Chen Formula
*/

/// given two hole cards, return the Chen Formula score
fn score_preflop_hand(hole_cards: &Vec<Card>) -> f32 {
    if hole_cards.len() != 2 {
	// something is weird
	return 0f32;
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
    score.ceil()
}

/// given a player and gamehand, this function returns a player action depending on the state of the game
pub fn get_bot_action(player: &Player, gamehand: &GameHand) -> PlayerAction {

    match gamehand.street {
        Street::Preflop => {
	    let blah = get_preflop_action(player, gamehand); //.unwrap_or(PlayerAction::Fold);
	    println!("blah = {:?}", blah);
	    blah.unwrap()
        }
        Street::Flop => {
	    get_random_action(player)
        }
        Street::Turn => {
	    get_random_action(player)	    
        }
        Street::River => {
	    get_random_action(player)	    
        }
        Street::ShowDown => {
	    get_random_action(player)	    
	}
    }
}

fn get_random_action(player: &Player) -> PlayerAction {
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


fn get_mediocre_action(player: &Player, cannot_check: bool, facing_raise: bool) -> PlayerAction {
    let num = rand::thread_rng().gen_range(0..100);
    match num {
        0..=5 => PlayerAction::Check,
        6..=15 => PlayerAction::Check,
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

fn get_preflop_action(player: &Player, gamehand: &GameHand) -> Result<PlayerAction, BotActionError> {
    if player.index.is_none(){
	return Err(BotActionError::NoIndexSet);
    }
    let score = score_preflop_hand(&player.hole_cards);
    let bot_contribution = gamehand.get_current_contributions_for_index(player.index.unwrap());    
    let cannot_check = bot_contribution < gamehand.current_bet;
    let facing_raise = gamehand.current_bet > gamehand.big_blind;    
    println!("sup: {score}");
    match score as i64 {
	// TODO: need to consider number of players and position
	-1..=6 => {
	    if cannot_check {
		Ok(PlayerAction::Fold)	    
	    } else {
		Ok(PlayerAction::Check)
	    }
	}
	7..=9 => {
	    Ok(get_mediocre_action(player, cannot_check, facing_raise))		
	}
	_ => {
	    // TODO get_big_bet
	    Ok(PlayerAction::Check)
	}
    }
	    

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
        assert_eq!(score, 20.0);	
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
        assert_eq!(score, 12.0);	
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
        assert_eq!(score, 12.0);	
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
        assert_eq!(score, 6.0);	
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
        assert_eq!(score, -1.0);	
    }
    
    /// if a bot has a garbage hand, then they will check if possible
    #[test]
    fn check_garbage_preflop() {
        let mut bot0 = Player::new_bot(200);
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
    
}
