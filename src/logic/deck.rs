use rand::seq::SliceRandom; // for shuffling a vec

use super::card::{Card, Rank, Suit};

///
/// This file contains structs/enums/methods for defining, using, and comparing cards and hands of cards
///
use strum::IntoEnumIterator;

/// trait to define behaviour that you would expect out of a deck of cards
/// in unit tests, we may want to provide a rigged deck, wherease in a normal game
/// we just want a standard random deck of cards
pub trait Deck: Send + std::fmt::Debug {
    /// shuffle the deck to randomize (possibly) the output of future cards
    fn shuffle(&mut self);

    /// give us a single card. Optional, because the deck may be exhausted
    fn draw_card(&mut self) -> Option<Card>;
}

#[derive(Debug)]
pub struct StandardDeck {
    cards: Vec<Card>,
    top: usize, // index that we deal the next card from
}

impl StandardDeck {
    pub fn new() -> Self {
        // returns a new unshuffled deck of 52 cards
        let mut cards = Vec::<Card>::with_capacity(52);
        for rank in Rank::iter() {
            for suit in Suit::iter() {
                cards.push(Card { rank, suit });
            }
        }
        Self { cards, top: 0 }
    }
}

impl Deck for StandardDeck {
    fn shuffle(&mut self) {
        // shuffle the deck of cards
        self.cards.shuffle(&mut rand::thread_rng());
        self.top = 0;
    }

    fn draw_card(&mut self) -> Option<Card> {
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

#[derive(Debug)]
pub struct RiggedDeck {
    cards: Vec<Card>,
    top: usize, // index that we deal the next card from
}

impl RiggedDeck {
    pub fn new() -> Self {
        let cards = Vec::<Card>::new();
        Self { cards, top: 0 }
    }

    /// push a card into the deck.
    /// we can set the order exactly how we want
    pub fn push(&mut self, card: Card) {
        self.cards.push(card);
    }
}

impl Deck for RiggedDeck {
    /// shuffle does nothing
    fn shuffle(&mut self) {}

    fn draw_card(&mut self) -> Option<Card> {
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
