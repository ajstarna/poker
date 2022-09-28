use super::card::Card;
use uuid::Uuid;

#[derive(Debug)]
pub enum PlayerAction {
    PostSmallBlind(f64),
    PostBigBlind(f64),
    Fold,
    Check,
    Bet(f64),
    Call,
    //Raise(u32), // i guess a raise is just a bet really?
}

#[derive(Debug)]
pub struct Player {
    name: String,
    id: Uuid, // the session id to uniquely identify the player    
    hole_cards: Vec<Card>,
    is_active: bool,      // is still playing the current hand
    is_sitting_out: bool, // if sitting out, then they are not active for any future hand
    money: f64,
    human_controlled: bool, // do we need user input or let the computer control it
    current_action: Option<PlayerAction>, // this action can be set from the websocket, so should be there when we need it
}

impl Player {
    pub fn new(name: String, id: Uuid, human_controlled: bool) -> Self {
        Player {
            name,
	    id,	    
            hole_cards: Vec::<Card>::with_capacity(2),
            is_active: true,
            is_sitting_out: false,
            money: 1000.0, // let them start with 1000 for now,
            human_controlled,
	    current_action: None,
        }
    }

    fn pay(&mut self, payment: f64) {
        self.money += payment;
    }

    fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// If the player has put all their money in, but has not folded (is_active),
    /// then they are all-in
    fn is_all_in(&self) -> bool {
        self.is_active && self.money == 0.0
    }
}
