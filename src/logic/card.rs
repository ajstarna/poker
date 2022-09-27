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
    TWO = 2,
    THREE = 3,
    FOUR = 4,
    FIVE = 5,
    SIX = 6,
    SEVEN = 7,
    EIGHT = 8,
    NINE = 9,
    TEN = 10,
    JACK = 11,
    QUEEN = 12,
    KING = 13,
    ACE = 14,
}

#[derive(Eq, PartialEq, Debug, Copy, Clone, EnumIter)]
enum Suit {
    CLUB,
    DIAMOND,
    HEART,
    SPADE,
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
    HIGHCARD = 1,
    PAIR = 2,
    TWOPAIR = 3,
    THREEOFAKIND = 4,
    STRAIGHT = 5,
    FLUSH = 6,
    FULLHOUSE = 7,
    FOUROFAKIND = 8,
    STRAIGHTFLUSH = 9,
    ROYALFLUSH = 10,
}

/// The hand result has the HandRanking, for quick comparisons, then the cads that make
/// up that HandRanking, along with the remaining kicker cards for tie breaking (sorted)
/// There is also a field "value", which gives a value of the hand that can be used to quickly
/// compare it against other hands. The HandRanking, then each of the constituent cards, then the kickers,
/// are each represented by 4 bits, so a better hand will have a higher value.
/// e.g. a hand of Q, Q, Q, 9, 4 would look like
/// {
/// hand_ranking: HandRanking::THREEOFAKIND,
/// contsituent_cards: [Q, Q, Q],
/// kickers: [9, 4]
/// value = [3] | [12] | [12] | [12] | [9] | [4] == [0000] | [0000] | [0011] | [1100] | [1100] | [1100] | [1001] | [0100]
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
    fn score_hand(
        hand_ranking: HandRanking,
        constituent_cards: &Vec<Card>,
        kickers: &Vec<Card>,
    ) -> u32 {
        let mut value = hand_ranking as u32;
        value = value << 20; // shift it into the most sifnificant area we need
        match hand_ranking {
            HandRanking::HIGHCARD
            | HandRanking::PAIR
            | HandRanking::THREEOFAKIND
            | HandRanking::STRAIGHT
            | HandRanking::FLUSH
            | HandRanking::FOUROFAKIND
            | HandRanking::STRAIGHTFLUSH
            | HandRanking::ROYALFLUSH => {
                // These handrankings are all uniquely identified by a single constituent card
                // first add the rank of the constituent
                let mut extra = constituent_cards.last().unwrap().rank as u32;
                extra = extra << 16;
                value += extra;
            }
            HandRanking::TWOPAIR => {
                // a two pair is valued by its higher pair, then lower pair
                let mut extra = constituent_cards.last().unwrap().rank as u32;
                extra = extra << 16;
                value += extra;

                // the lower pair is sorted to the front
                extra = constituent_cards[0].rank as u32;
                extra = extra << 12;
                value += extra;
            }
            HandRanking::FULLHOUSE => {
                // a full house is valued first by the three of a kind, then the pair
                // the three of a kind will always exist as the middle element, regardless of the sort order
                let mut extra = constituent_cards[2].rank as u32;
                extra = extra << 16;
                value += extra;

                // the pair will be either at the beginning or the end of the constituent_cards, we need to check.
                // this depends on the sort.
                // e.g. could be [ 2, 2, 2, 6, 6 ], OR [ 2, 2, 6, 6, 6 ]
                let mut second_extra = constituent_cards[0].rank as u32;
                if second_extra == extra {
                    // the first card was the same as the middle card, i.e. we grabbed another card in the three of a kind.
                    // So grab the last card in the list, which will necessarily be part of the pair
                    second_extra = constituent_cards.last().unwrap().rank as u32;
                }
                second_extra = second_extra << 12;
                value += second_extra;
            }
        }

        // next add the value of the kicker(s), in order
        // Note: for rankings without kickers, this loop simply won't happen
        let mut shift_amount = 0;
        // TODO: double check this logic. originally shift_amount started at 12. but that wont work for 2 pair in particular, since
        // the second pair is being shifted 12. so if we start at 0 and go UP, i think we are good right?
        for i in 0..(kickers.len()) {
            let mut extra = kickers[i].rank as u32;
            extra = extra << shift_amount;
            value += extra;
            shift_amount += 4;
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
                // TODO: we need to handle ACE being high or low
                is_straight = false;
            }
        }

        /*
        if is_flush {
            println!("is_flush = {}", is_flush);
        }
        if is_straight {
            println!("is_straight = {}", is_straight);
        }
         */
        //println!("rank counts = {:?}", rank_counts);
        let mut constituent_cards = Vec::new();

        let mut kickers = Vec::new();

        if is_flush && is_straight {
            if let Rank::ACE = five_cards[4].rank {
                hand_ranking = HandRanking::ROYALFLUSH;
            } else {
                hand_ranking = HandRanking::STRAIGHTFLUSH;
            }
            constituent_cards.extend(five_cards);
        } else {
            let mut num_fours = 0;
            let mut num_threes = 0;
            let mut num_twos = 0;
            for (_, count) in &rank_counts {
                //println!("rank = {:?}, count = {}", rank, count);
                match count {
                    4 => num_fours += 1,
                    3 => num_threes += 1,
                    2 => num_twos += 1,
                    _ => (),
                }
            }

            if num_fours == 1 {
                hand_ranking = HandRanking::FOUROFAKIND;
                for card in five_cards {
                    match *rank_counts.get(&card.rank).unwrap() {
                        4 => constituent_cards.push(card),
                        _ => kickers.push(card),
                    }
                }
            } else if num_threes == 1 && num_twos == 1 {
                hand_ranking = HandRanking::FULLHOUSE;
                for card in five_cards {
                    match *rank_counts.get(&card.rank).unwrap() {
                        2 | 3 => constituent_cards.push(card),
                        _ => kickers.push(card),
                    }
                }
            } else if is_flush {
                hand_ranking = HandRanking::FLUSH;
                constituent_cards.extend(five_cards);
            } else if is_straight {
                hand_ranking = HandRanking::STRAIGHT;
                constituent_cards.extend(five_cards);
            } else if num_threes == 1 {
                hand_ranking = HandRanking::THREEOFAKIND;
                for card in five_cards {
                    match *rank_counts.get(&card.rank).unwrap() {
                        3 => constituent_cards.push(card),
                        _ => kickers.push(card),
                    }
                }
            } else if num_twos == 2 {
                hand_ranking = HandRanking::TWOPAIR;
                for card in five_cards {
                    match *rank_counts.get(&card.rank).unwrap() {
                        2 => constituent_cards.push(card),
                        _ => kickers.push(card),
                    }
                }
            } else if num_twos == 1 {
                hand_ranking = HandRanking::PAIR;
                for card in five_cards {
                    match *rank_counts.get(&card.rank).unwrap() {
                        2 => constituent_cards.push(card),
                        _ => kickers.push(card),
                    }
                }
            } else {
                hand_ranking = HandRanking::HIGHCARD;
                constituent_cards.push(five_cards[4]);
                for &card in five_cards.iter().take(4) {
                    kickers.push(card);
                }
            }
        }
        constituent_cards.sort();
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
