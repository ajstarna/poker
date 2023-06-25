use std::cmp::Ordering;
use std::collections::HashMap;

use super::card::{Card, Rank, Suit};

use strum_macros::EnumIter;

#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Copy, Clone, EnumIter)]
pub enum HandRanking {
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


pub enum DrawType {
    Frontdoor,
    Backdoor,
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
    pub hand_ranking: HandRanking,
    pub constituent_cards: Vec<Card>,
    pub kickers: Vec<Card>,
    pub value: u32, // the absolute value of this hand, which can be used to compare against another hand
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
impl ToString for HandResult {
    fn to_string(&self) -> String {
        format!(
            "{:?}: {}, {}",
            self.hand_ranking,
            self.constituent_cards
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join("-"),
            self.kickers
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join("-")
        )
    }
}

impl HandResult {
    /// Given a hand ranking, along with the constituent cards and the kickers, this function returns a numerical score
    /// that determines the hand's value compared to any other hand
    /// note that the cards have been specially pre-sorted by analyze_hand,
    /// and hence this shouldn't be called otherwise
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

        let hand_ranking: HandRanking;

        let mut rank_counts: HashMap<Rank, u8> = HashMap::new();
        let mut is_flush = true;
        let first_suit = five_cards[0].suit;
        let mut is_straight = true;
        let mut is_low_ace_straight = false;
        let first_rank = five_cards[0].rank as usize;
        for (i, card) in five_cards.iter().enumerate() {
            let count = rank_counts.entry(card.rank).or_insert(0);
            *count += 1;
            if card.suit != first_suit {
                is_flush = false;
            }
            if is_straight && i == 4 && card.rank == Rank::Ace && first_rank == 2 {
                // completing the straight with an Ace on 2-->Ace
                is_low_ace_straight = true;
            } else if card.rank as usize != first_rank + i {
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

	// special case constituent sorting:
	
        if hand_ranking == HandRanking::FullHouse {
            // for a full house we actually want to make sure the sort has the 3 of a kind
            // sorted "higher" than the pair (since that is what matters more when determining
            // hand value)
            if constituent_cards[0].rank == constituent_cards[2].rank {
                constituent_cards.reverse();
            }
	}
        if is_low_ace_straight {
            // we want the constituent cards to be sorted with the Ace being "low",
            // so we need to move it to the beginning
            let ace = constituent_cards.pop().unwrap();
            constituent_cards.insert(0, ace);
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

    pub fn hand_ranking_string(&self) -> String {
        format!(
            "{:?}",
            self.hand_ranking
        )
    }

    pub fn constituent_cards_string(&self) -> String {
	self.constituent_cards
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join("-")
	    .to_string()
    }

    pub fn kickers_string(&self) -> String {
        self.kickers
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join("-")
            .to_string()
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn compare_high_card_and_pair() {
        let hand1 = vec![
            Card {
                rank: Rank::Two,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Four,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Five,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Nine,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Spade,
            },
        ];
        let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::HighCard);

        let hand2 = vec![
            Card {
                rank: Rank::Two,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Two,
                suit: Suit::Club,
            },
            Card {
                rank: Rank::Three,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Seven,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Nine,
                suit: Suit::Spade,
            },
        ];

        let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::Pair);
        assert_eq!(
            result2.constituent_cards,
            vec![
                Card {
                    rank: Rank::Two,
                    suit: Suit::Spade
                },
                Card {
                    rank: Rank::Two,
                    suit: Suit::Club
                },
            ]
        );

        assert!(result1 < result2);
    }

    #[test]
    fn compare_two_pair_and_three() {
        let hand1 = vec![
            Card {
                rank: Rank::Two,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Two,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Five,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Five,
                suit: Suit::Diamond,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Spade,
            },
        ];
        let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::TwoPair);
        assert_eq!(
            result1.constituent_cards,
            vec![
                Card {
                    rank: Rank::Two,
                    suit: Suit::Spade
                },
                Card {
                    rank: Rank::Two,
                    suit: Suit::Heart
                },
                Card {
                    rank: Rank::Five,
                    suit: Suit::Heart
                },
                Card {
                    rank: Rank::Five,
                    suit: Suit::Diamond
                },
            ]
        );

        let hand2 = vec![
            Card {
                rank: Rank::King,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Diamond,
            },
            Card {
                rank: Rank::Five,
                suit: Suit::Diamond,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Spade,
            },
        ];

        let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::ThreeOfAKind);
        assert_eq!(
            result2.constituent_cards,
            vec![
                Card {
                    rank: Rank::King,
                    suit: Suit::Spade
                },
                Card {
                    rank: Rank::King,
                    suit: Suit::Heart
                },
                Card {
                    rank: Rank::King,
                    suit: Suit::Diamond
                },
            ]
        );

        assert!(result1 < result2);
    }

    #[test]
    fn compare_high_cards() {
        let hand1 = vec![
            Card {
                rank: Rank::Two,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Seven,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Eight,
                suit: Suit::Diamond,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Spade,
            },
        ];
        let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::HighCard);

        let hand2 = vec![
            Card {
                rank: Rank::Two,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Five,
                suit: Suit::Spade,
            }, // the 5 is less than the 7
            Card {
                rank: Rank::Eight,
                suit: Suit::Diamond,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Spade,
            },
        ];

        let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::HighCard);
        assert!(result1 > result2);
    }

    #[test]
    fn compare_straights() {
        let hand1 = vec![
            Card {
                rank: Rank::Two,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Three,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Four,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Five,
                suit: Suit::Diamond,
            },
            Card {
                rank: Rank::Six,
                suit: Suit::Spade,
            },
        ];
        let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::Straight);

        let hand2 = vec![
            Card {
                rank: Rank::Nine,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Ten,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Diamond,
            },
            Card {
                rank: Rank::Queen,
                suit: Suit::Diamond,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Spade,
            },
        ];

        let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::Straight);
        assert!(result1 < result2);
    }

    /// these flushes have some cards in common (as will happen in a real game)
    #[test]
    fn compare_flushes() {
        let hand1 = vec![
            Card {
                rank: Rank::Two,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Seven,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Eight,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Spade,
            },
        ];
        let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::Flush);

        let hand2 = vec![
            Card {
                rank: Rank::Two,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Five,
                suit: Suit::Spade,
            }, // the 5 is less than the 7
            Card {
                rank: Rank::Eight,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Spade,
            },
        ];

        let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::Flush);
        assert!(result1 > result2);
    }

    #[test]
    fn compare_full_houses() {
        let hand1 = vec![
            Card {
                rank: Rank::Two,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Two,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Queen,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Queen,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Queen,
                suit: Suit::Club,
            },
        ];
        let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::FullHouse);

        let hand2 = vec![
            Card {
                rank: Rank::Two,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Two,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Two,
                suit: Suit::Diamond,
            },
            Card {
                rank: Rank::Queen,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Queen,
                suit: Suit::Club,
            },
        ];

        let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::FullHouse);
        assert!(result1 > result2);
    }

    #[test]
    fn compare_full_houses_2() {
        // the fives should beat the twos
        let hand1 = vec![
            Card {
                rank: Rank::Five,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Five,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Five,
                suit: Suit::Diamond,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Club,
            },
        ];
        let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::FullHouse);

        let hand2 = vec![
            Card {
                rank: Rank::Two,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Two,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Two,
                suit: Suit::Diamond,
            },
            Card {
                rank: Rank::Queen,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Queen,
                suit: Suit::Club,
            },
        ];

        let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::FullHouse);
        assert!(result1 > result2);
    }

    #[test]
    fn compare_pairs() {
        let hand1 = vec![
            Card {
                rank: Rank::Two,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Three,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Six,
                suit: Suit::Club,
            },
            Card {
                rank: Rank::Queen,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Queen,
                suit: Suit::Heart,
            },
        ];
        let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::Pair);

        let hand2 = vec![
            Card {
                rank: Rank::Two,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Three,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Ten,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Ten,
                suit: Suit::Club,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Diamond,
            },
        ];

        let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::Pair);
        assert!(result1 > result2);
    }

    #[test]
    fn ace_high_low() {
        let hand1 = vec![
            Card {
                rank: Rank::Ace,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Two,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Three,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Four,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Five,
                suit: Suit::Spade,
            },
        ];
        let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::Straight);

        let hand2 = vec![
            Card {
                rank: Rank::Three,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Four,
                suit: Suit::Club,
            },
            Card {
                rank: Rank::Five,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Six,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Seven,
                suit: Suit::Spade,
            },
        ];

        let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::Straight);
        assert!(result1 < result2);

        let hand3 = vec![
            Card {
                rank: Rank::Ten,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Jack,
                suit: Suit::Club,
            },
            Card {
                rank: Rank::Queen,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Heart,
            },
            Card {
                rank: Rank::Ace,
                suit: Suit::Spade,
            },
        ];

        let result3 = HandResult::analyze_hand(hand3);
        assert_eq!(result3.hand_ranking, HandRanking::Straight);
        assert!(result1 < result3);
        assert!(result2 < result3);
    }

    /// this was an actual bug, where a 2 and and A in a hand would lead to the
    /// Ace being sorted to the front EVEN if not actually a straight!
    #[test]
    fn not_ace_straight() {
	// an Ace high flush
        let hand1 = vec![
            Card {
                rank: Rank::Ace,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Two,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Three,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::Seven,
                suit: Suit::Spade,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Spade,
            },
        ];
        let result1 = HandResult::analyze_hand(hand1);
        assert_eq!(result1.hand_ranking, HandRanking::Flush);

	// a King high flush
        let hand2 = vec![
            Card {
                rank: Rank::Three,
                suit: Suit::Club,
            },
            Card {
                rank: Rank::Nine,
                suit: Suit::Club,
            },
            Card {
                rank: Rank::Ten,
                suit: Suit::Club,
            },
            Card {
                rank: Rank::Queen,
                suit: Suit::Club,
            },
            Card {
                rank: Rank::King,
                suit: Suit::Club,
            },
        ];

        let result2 = HandResult::analyze_hand(hand2);
        assert_eq!(result2.hand_ranking, HandRanking::Flush);
        assert!(result1 > result2);
    }
    
}
