use rand::seq::SliceRandom; // for shuffling a vec
use std::cmp::Ordering;
use std::collections::HashMap;
///
/// This file contains structs/enums/methods for defining, using, and comparing cards and hands of cards
///
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Copy, Clone, EnumIter, Hash)]
enum Rank {
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
    Ten = 10,
    Jack = 11,
    Queen = 12,
    King = 13,
    Ace = 14,
}

#[derive(Eq, PartialEq, Debug, Copy, Clone, EnumIter)]
enum Suit {
    Club,
    Diamond,
    Heart,
    Spade,
}

#[derive(Eq, Debug, Copy, Clone)]
pub struct Card {
    rank: Rank,
    suit: Suit,
}

/// We simply compare Cards based on their rank field.
impl Ord for Card {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank.cmp(&other.rank)
    }
}

impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Card {
    fn eq(&self, other: &Self) -> bool {
        self.rank == other.rank
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Copy, Clone, EnumIter)]
enum HandRanking {
    HighCard = 1,
    Pair = 2,
    TwoPair = 3,
    ThreeOfAKind = 4,
    Straight = 5,
    Flush = 6,
    FullHouse = 7,
    FourOfAKind = 8,
    StraightFlush = 9,
    RoyalFlush = 10,
}

/// The hand result has the HandRanking, for quick comparisons, then the cads that make
/// up that HandRanking, along with the remaining kicker cards for tie breaking (sorted)
/// There is also a field "value", which gives a value of the hand that can be used to quickly
/// compare it against other hands. The HandRanking, then each of the constituent cards, then the kickers,
/// are each represented by 4 bits, so a better hand will have a higher value.
/// e.g. a hand of Q, Q, Q, 9, 4 would look like
/// {
/// hand_ranking: HandRanking::ThreeOfAKind,
/// contsituent_cards: [Q, Q, Q],
/// kickers: [9, 4]
/// value = [3] | [12] | [12] | [12] | [9] | [4]
///        == [0000] | [0000] | [0011] | [1100] | [1100] | [1100] | [1001] | [0100]
/// Note: value has eight leading 0s since we only need 24 bits to represent it.
/// }
#[derive(Debug, Eq)]
pub struct HandResult {
    hand_ranking: HandRanking,
    constituent_cards: Vec<Card>,
    kickers: Vec<Card>,
    value: u32, // the absolute value of this hand, which can be used to compare against another hand
}

/// We simply compare HandResults based on their value field.
impl Ord for HandResult {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl PartialOrd for HandResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for HandResult {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl HandResult {
    /// Given a hand ranking, along with the constituent cards and the kickers, this function returns a numerical score
    /// that determines the hand's value compared to any other hand
    /// note that the cards have been pre-sorted by analyze_hand, and hence this shouldn't be called otherwise
    fn score_hand(
        hand_ranking: HandRanking,
        constituent_cards: &Vec<Card>,
        kickers: &[Card],
    ) -> u32 {
        let mut value = hand_ranking as u32;
        value <<= 20; // shift it into the most significant area we need

	// add the values of the constituent cards and then kickers
	// note that this is only a tie breaker when the handranking is the same for both hands
	// e.g. a pair of Kings will lead to higher "extra" bit value than a two-pair of 5s and 6s,
	// but the original significant bits will be higher for the two-pair
	
	// The constituent cards have more significant value than the kickers,
	// e.g. a pair of Queens with a highest card of 6 in its kickers will get a higher score
	// than a pair of 10s with a highest card of King in its kickers (because the King doesn't get added
	// until we are dealing with the kicker bits, and the Queens get a higher score than the 10s)
        let mut shift_amount = 16;
        for card in constituent_cards.iter().rev() {
	    // the highest cards are shifted all the way to the left
            let mut extra = card.rank as u32;
            extra <<= shift_amount;
            value += extra;
            shift_amount -= 4;
        }	
        for kicker in kickers.iter().rev() {
            let mut extra = kicker.rank as u32;
            extra <<= shift_amount;
            value += extra;
            shift_amount -= 4;
        }

        value
    }

    /// Given a hand of 5 cards, we return a HandResult, which tells
    /// us the hand ranking, the constituent cards, kickers, and hand score    
    pub fn analyze_hand(mut five_cards: Vec<Card>) -> Self {
        assert!(five_cards.len() == 5);
        five_cards.sort(); // first sort by Rank
                           //println!("five cards = {:?}", five_cards);

        let hand_ranking: HandRanking;

        let mut rank_counts: HashMap<Rank, u8> = HashMap::new();
        let mut is_flush = true;
        let first_suit = five_cards[0].suit;
        let mut is_straight = true;
        let first_rank = five_cards[0].rank as usize;
        for (i, card) in five_cards.iter().enumerate() {
            let count = rank_counts.entry(card.rank).or_insert(0);
            *count += 1;
            if card.suit != first_suit {
                is_flush = false;
            }
            if card.rank as usize != first_rank + i {
                // TODO: we need to handle Ace being high or low
                is_straight = false;
            }
        }
        let mut constituent_cards = Vec::new();

        let mut kickers = Vec::new();

        if is_flush && is_straight {
            if let Rank::Ace = five_cards[4].rank {
                hand_ranking = HandRanking::RoyalFlush;
            } else {
                hand_ranking = HandRanking::StraightFlush;
            }
            constituent_cards.extend(five_cards);
        } else {
            let mut num_fours = 0;
            let mut num_threes = 0;
            let mut num_twos = 0;
            for count in rank_counts.values() {
                //println!("rank = {:?}, count = {}", rank, count);
                match count {
                    4 => num_fours += 1,
                    3 => num_threes += 1,
                    2 => num_twos += 1,
                    _ => (),
                }
            }

            if num_fours == 1 {
                hand_ranking = HandRanking::FourOfAKind;
                for card in five_cards {
                    match *rank_counts.get(&card.rank).unwrap() {
                        4 => constituent_cards.push(card),
                        _ => kickers.push(card),
                    }
                }
            } else if num_threes == 1 && num_twos == 1 {
                hand_ranking = HandRanking::FullHouse;
                for card in five_cards {
                    match *rank_counts.get(&card.rank).unwrap() {
                        2 | 3 => constituent_cards.push(card),
                        _ => kickers.push(card),
                    }
                }
            } else if is_flush {
                hand_ranking = HandRanking::Flush;
                constituent_cards.extend(five_cards);
            } else if is_straight {
                hand_ranking = HandRanking::Straight;
                constituent_cards.extend(five_cards);
            } else if num_threes == 1 {
                hand_ranking = HandRanking::ThreeOfAKind;
                for card in five_cards {
                    match *rank_counts.get(&card.rank).unwrap() {
                        3 => constituent_cards.push(card),
                        _ => kickers.push(card),
                    }
                }
            } else if num_twos == 2 {
                hand_ranking = HandRanking::TwoPair;
                for card in five_cards {
                    match *rank_counts.get(&card.rank).unwrap() {
                        2 => constituent_cards.push(card),
                        _ => kickers.push(card),
                    }
                }
            } else if num_twos == 1 {
                hand_ranking = HandRanking::Pair;
                for card in five_cards {
                    match *rank_counts.get(&card.rank).unwrap() {
                        2 => constituent_cards.push(card),
                        _ => kickers.push(card),
                    }
                }
            } else {
                hand_ranking = HandRanking::HighCard;
                constituent_cards.push(five_cards[4]);
                for &card in five_cards.iter().take(4) {
                    kickers.push(card);
                }
            }
        }
        constituent_cards.sort();

	if hand_ranking == HandRanking::FullHouse {
	    // for a full house we actually want to make sure the sort has the 3 of a kind
	    // sorted "higher" than the pair (since that is what matters more when determining
	    // hand value)
	    if constituent_cards[0].rank == constituent_cards[2].rank {
		constituent_cards.reverse();
	    }
	}
	
        kickers.sort();
        let value = HandResult::score_hand(hand_ranking, &constituent_cards, &kickers);
        Self {
            hand_ranking,
            constituent_cards,
            kickers,
            value,
        }
    }
}

#[derive(Debug)]
pub struct Deck {
    cards: Vec<Card>,
    top: usize, // index that we deal the next card from
}

impl Deck {
    pub fn new() -> Self {
        // returns a new unshuffled deck of 52 cards
        let mut cards = Vec::<Card>::with_capacity(52);
        for rank in Rank::iter() {
            for suit in Suit::iter() {
                cards.push(Card { rank, suit });
            }
        }
        Deck { cards, top: 0 }
    }

    pub fn shuffle(&mut self) {
        // shuffle the deck of cards
        self.cards.shuffle(&mut rand::thread_rng());
        self.top = 0;
    }

    pub fn draw_card(&mut self) -> Option<Card> {
        // take the top card from the deck and move the index of the top of the deck
        if self.top == self.cards.len() {
            // the deck is exhausted, no card to give
            None
        } else {
            let card = self.cards[self.top];
            self.top += 1;
            Some(card)
        }
    }
}

#[cfg(test)]
mod tests {
     use super::*;

    #[test]
    fn compare_high_card_and_pair() {
        let hand1 = vec![
	    Card{rank: Rank::Two, suit: Suit::Spade},
	    Card{rank: Rank::Four, suit: Suit::Spade},
	    Card{rank: Rank::Five, suit: Suit::Heart},
	    Card{rank: Rank::Nine, suit: Suit::Heart},
	    Card{rank: Rank::Jack, suit: Suit::Spade},	    
	];
	let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::HighCard);

        let hand2 = vec![
	    Card{rank: Rank::Two, suit: Suit::Spade},
	    Card{rank: Rank::Two, suit: Suit::Club},
	    Card{rank: Rank::Three, suit: Suit::Spade},
	    Card{rank: Rank::Seven, suit: Suit::Heart},
	    Card{rank: Rank::Nine, suit: Suit::Spade},	    
	];
	
	let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::Pair);
        assert_eq!(
	    result2.constituent_cards,
	    vec![
	    	Card{rank: Rank::Two, suit: Suit::Spade},
		Card{rank: Rank::Two, suit: Suit::Club},
	    ]
	);

	assert!(result1 < result2);
    }

/*
    HighCard = 1,
    Pair = 2,
    TwoPair = 3,
    ThreeOfAKind = 4,
    Straight = 5,
    Flush = 6,
    FullHouse = 7,
    FourOfAKind = 8,
    StraightFlush = 9,
    RoyalFlush = 10,
*/
    
    #[test]
    fn compare_two_pair_and_three() {
        let hand1 = vec![
	    Card{rank: Rank::Two, suit: Suit::Spade},
	    Card{rank: Rank::Two, suit: Suit::Heart},
	    Card{rank: Rank::Five, suit: Suit::Heart},
	    Card{rank: Rank::Five, suit: Suit::Diamond},
	    Card{rank: Rank::Jack, suit: Suit::Spade},	    
	];
	let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::TwoPair);
        assert_eq!(
	    result1.constituent_cards,
	    vec![
	    Card{rank: Rank::Two, suit: Suit::Spade},
	    Card{rank: Rank::Two, suit: Suit::Heart},
	    Card{rank: Rank::Five, suit: Suit::Heart},
	    Card{rank: Rank::Five, suit: Suit::Diamond},		
	    ]
	);

        let hand2 = vec![
	    Card{rank: Rank::King, suit: Suit::Spade},
	    Card{rank: Rank::King, suit: Suit::Heart},
	    Card{rank: Rank::King, suit: Suit::Diamond},
	    Card{rank: Rank::Five, suit: Suit::Diamond},
	    Card{rank: Rank::Jack, suit: Suit::Spade},	    
	    
	];
	
	let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::ThreeOfAKind);
        assert_eq!(
	    result2.constituent_cards,
	    vec![
	    Card{rank: Rank::King, suit: Suit::Spade},
	    Card{rank: Rank::King, suit: Suit::Heart},
	    Card{rank: Rank::King, suit: Suit::Diamond},		
	    ]
	);

	assert!(result1 < result2);
    }

    #[test]
    fn compare_high_cards() {
        let hand1 = vec![
	    Card{rank: Rank::Two, suit: Suit::Spade},
	    Card{rank: Rank::Seven, suit: Suit::Spade},
	    Card{rank: Rank::Eight, suit: Suit::Diamond},
	    Card{rank: Rank::Jack, suit: Suit::Spade},
	    Card{rank: Rank::King, suit: Suit::Spade},	    
	];
	let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::HighCard);

        let hand2 = vec![
	    Card{rank: Rank::Two, suit: Suit::Spade},
	    Card{rank: Rank::Five, suit: Suit::Spade}, // the 5 is less than the 7
	    Card{rank: Rank::Eight, suit: Suit::Diamond},
	    Card{rank: Rank::Jack, suit: Suit::Spade},
	    Card{rank: Rank::King, suit: Suit::Spade},	    	    
	];
	
	let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::HighCard);
	assert!(result1 > result2);
    }
    
    #[test]
    fn compare_straights() {
        let hand1 = vec![
	    Card{rank: Rank::Two, suit: Suit::Spade},
	    Card{rank: Rank::Three, suit: Suit::Heart},
	    Card{rank: Rank::Four, suit: Suit::Heart},
	    Card{rank: Rank::Five, suit: Suit::Diamond},
	    Card{rank: Rank::Six, suit: Suit::Spade},	    
	];
	let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::Straight);

        let hand2 = vec![
	    Card{rank: Rank::Nine, suit: Suit::Spade},
	    Card{rank: Rank::Ten, suit: Suit::Heart},
	    Card{rank: Rank::Jack, suit: Suit::Diamond},
	    Card{rank: Rank::Queen, suit: Suit::Diamond},
	    Card{rank: Rank::King, suit: Suit::Spade},	    
	    
	];
	
	let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::Straight);
	assert!(result1 < result2);
    }

    /// these flushes have some cards in common (as will happen in a real game)
    #[test]
    fn compare_flushes() {
        let hand1 = vec![
	    Card{rank: Rank::Two, suit: Suit::Spade},
	    Card{rank: Rank::Seven, suit: Suit::Spade},
	    Card{rank: Rank::Eight, suit: Suit::Spade},
	    Card{rank: Rank::Jack, suit: Suit::Spade},
	    Card{rank: Rank::King, suit: Suit::Spade},	    
	];
	let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::Flush);

        let hand2 = vec![
	    Card{rank: Rank::Two, suit: Suit::Spade},
	    Card{rank: Rank::Five, suit: Suit::Spade}, // the 5 is less than the 7
	    Card{rank: Rank::Eight, suit: Suit::Spade},
	    Card{rank: Rank::Jack, suit: Suit::Spade},
	    Card{rank: Rank::King, suit: Suit::Spade},	    	    
	];
	
	let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::Flush);
	assert!(result1 > result2);
    }    

    #[test]
    fn compare_full_houses() {
        let hand1 = vec![
	    Card{rank: Rank::Two, suit: Suit::Spade},
	    Card{rank: Rank::Two, suit: Suit::Heart},
	    Card{rank: Rank::Queen, suit: Suit::Spade},
	    Card{rank: Rank::Queen, suit: Suit::Heart},
	    Card{rank: Rank::Queen, suit: Suit::Club},	    
	];
	let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::FullHouse);

        let hand2 = vec![
	    Card{rank: Rank::Two, suit: Suit::Spade},
	    Card{rank: Rank::Two, suit: Suit::Heart},
	    Card{rank: Rank::Two, suit: Suit::Diamond},
	    Card{rank: Rank::Queen, suit: Suit::Heart},
	    Card{rank: Rank::Queen, suit: Suit::Club},	    
	];
	
	let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::FullHouse);
	assert!(result1 > result2);
    }    

    #[test]
    fn compare_full_houses_2() {
	// the fives should beat the twos
        let hand1 = vec![
	    Card{rank: Rank::Five, suit: Suit::Spade},
	    Card{rank: Rank::Five, suit: Suit::Heart},
	    Card{rank: Rank::Five, suit: Suit::Diamond},
	    Card{rank: Rank::Jack, suit: Suit::Heart},
	    Card{rank: Rank::Jack, suit: Suit::Club},	    
	];
	let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::FullHouse);

        let hand2 = vec![
	    Card{rank: Rank::Two, suit: Suit::Spade},
	    Card{rank: Rank::Two, suit: Suit::Heart},
	    Card{rank: Rank::Two, suit: Suit::Diamond},
	    Card{rank: Rank::Queen, suit: Suit::Heart},
	    Card{rank: Rank::Queen, suit: Suit::Club},	    
	];
	
	let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::FullHouse);
	assert!(result1 > result2);
    }    
    
    #[test]
    fn compare_pairs() {
        let hand1 = vec![
	    Card{rank: Rank::Two, suit: Suit::Spade},
	    Card{rank: Rank::Three, suit: Suit::Heart},
	    Card{rank: Rank::Six, suit: Suit::Club},	    	    
	    Card{rank: Rank::Queen, suit: Suit::Spade},
	    Card{rank: Rank::Queen, suit: Suit::Heart},
	];
	let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::Pair);

        let hand2 = vec![
	    Card{rank: Rank::Two, suit: Suit::Spade},
	    Card{rank: Rank::Three, suit: Suit::Heart},
	    Card{rank: Rank::Ten, suit: Suit::Heart},
	    Card{rank: Rank::Ten, suit: Suit::Club},
	    Card{rank: Rank::King, suit: Suit::Diamond},	    
	];
	
	let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::Pair);
	assert!(result1 > result2);
    }    
    
}
