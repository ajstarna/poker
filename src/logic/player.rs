use super::card::Card;
use crate::messages::WsMessage;
use actix::prelude::Recipient;
use uuid::Uuid;

#[derive(Debug, Copy, Clone)]
pub enum PlayerAction {
    PostSmallBlind(f64),
    PostBigBlind(f64),
    Fold,
    Check,
    Bet(f64),
    Call,
    //Raise(u32), // i guess a raise is just a bet really?
}
/// this struct holds the player name and recipient address
#[derive(Debug)]
pub struct PlayerSettings {
    pub id: Uuid,
    pub name: Option<String>,
    pub player_addr: Option<Recipient<WsMessage>>,
}

impl PlayerSettings {
    pub fn new(id: Uuid, name: Option<String>, player_addr: Option<Recipient<WsMessage>>) -> Self {
        Self {
            id,
            name,
            player_addr,
        }
    }
}

#[derive(Debug)]
pub struct Player {
    pub player_settings: PlayerSettings,
    pub hole_cards: Vec<Card>,
    pub is_active: bool,      // is still playing the current hand
    pub is_sitting_out: bool, // if sitting out, then they are not active for any future hand
    pub money: f64,
    pub human_controlled: bool, // do we need user input or let the computer control it
    pub current_action: Option<PlayerAction>, // this action can be set from the websocket, so should be there when we need it
}

impl Player {
    pub fn new(player_settings: PlayerSettings, human_controlled: bool) -> Self {
        Player {
            player_settings,
            hole_cards: Vec::<Card>::with_capacity(2),
            is_active: true,
            is_sitting_out: false,
            money: 1000.0, // let them start with 1000 for now,
            human_controlled,
            current_action: None,
        }
    }

    pub fn new_bot(name: String) -> Self {
        let bot_id = Uuid::new_v4(); // can just gen a new arbitrary id for the bot
        let player_settings = PlayerSettings::new(bot_id, Some(name), None); // recipient add == None
        Self::new(player_settings, false)
    }

    pub fn pay(&mut self, payment: f64) {
        self.money += payment;
    }

    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// If the player has put all their money in, but has not folded (is_active),
    /// then they are all-in
    pub fn is_all_in(&self) -> bool {
        self.is_active && self.money == 0.0
    }
}
