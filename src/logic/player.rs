use super::card::Card;
use crate::messages::WsMessage;
use actix::prelude::Recipient;
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Debug, Copy, Clone)]
pub enum PlayerAction {
    PostSmallBlind(u32),
    PostBigBlind(u32),
    Fold,
    Check,
    Bet(u32),
    Call,
    //Raise(u32), // i guess a raise is just a bet really?
}
/// this struct holds the player name and recipient address
#[derive(Debug)]
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
    pub fn send_specific_message(message: &str, id: Uuid, ids_to_configs: &HashMap<Uuid, PlayerConfig>) {
	if let Some(player_config) = ids_to_configs.get(&id) {
            if let Some(addr) = &player_config.player_addr {
		addr.do_send(WsMessage(message.to_owned()));
	    }
	}
    }

    /// find a player with the given id, and set their name to be the given name
    pub fn set_player_name(id: Uuid, name: &str, ids_to_configs: &mut HashMap<Uuid, PlayerConfig>) {
	if let Some(player_config) = ids_to_configs.get_mut(&id) {
            player_config.name = Some(name.to_string());
            player_config.player_addr.as_ref().unwrap()
		.do_send(
		    WsMessage(format!("You are changing your name to {:?}", name))
		);	    	    
	}
    }    
}

#[derive(Debug, Clone)]
pub struct Player {
    pub id: Uuid,    
    pub hole_cards: Vec<Card>,
    pub is_active: bool,      // is still playing the current hand
    pub is_sitting_out: bool, // if sitting out, then they are not active for any future hand
    pub money: u32,
    pub human_controlled: bool, // do we need user input or let the computer control it
}

impl Player {
    pub fn new(id: Uuid, human_controlled: bool) -> Self {
        Player {
            id,
            hole_cards: Vec::<Card>::with_capacity(2),
            is_active: false, // a branch new player is not active in a hand
            is_sitting_out: false,
            money: 1000, // let them start with 1000 for now,
            human_controlled,
        }
    }

    /// create a new bot from scratch
    pub fn new_bot() -> Self {
        let bot_id = Uuid::new_v4(); // can just gen a new arbitrary id for the bot
        Self::new(bot_id, false)
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
