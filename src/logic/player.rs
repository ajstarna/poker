use super::card::Card;
use crate::messages::WsMessage;
use actix::prelude::Recipient;
use std::collections::HashMap;
use uuid::Uuid;
use std::fmt;

#[derive(Debug, Copy, Clone)]
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
}

impl PlayerConfig {
    pub fn new(id: Uuid, name: Option<String>, player_addr: Option<Recipient<WsMessage>>) -> Self {
        Self {
            id,
            name,
            player_addr,
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
}

#[derive(Debug, Clone)]
pub struct Player {
    pub id: Uuid,
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
}
