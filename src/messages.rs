use crate::logic::{player::PlayerAction, PlayerConfig};
use actix::prelude::{Message, Recipient};
use uuid::Uuid;
use std::fmt;
use serde_json::Value;

/// Game server sends this messages to session

/// this enum represents higher level commands that the hub will relay
/// to the running games Player name change. player join/leave
#[derive(Debug)]
pub enum MetaAction {
    Join(PlayerConfig),
    Leave(Uuid),
    ImBack(Uuid),    
    SitOut(Uuid),
    PlayerName(Uuid, String),
    Chat(Uuid, String), 
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct WsMessage(pub String);

/// New ws session is created
#[derive(Message)]
#[rtype(result = "Uuid")]
pub struct Connect {
    pub addr: Recipient<WsMessage>,
}

/// Session is disconnected
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: Uuid,
}

/// List of available tables
pub struct ListTables;

impl actix::Message for ListTables {
    type Result = Vec<String>;
}

/// Join table, if table does not exists create new one.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    /// Client ID
    pub id: Uuid,

    /// Table name
    pub table_name: String,
}

/// the game sends this message to confirm that a player has been removed
/// It provides the playerconfig so it can be added back to the lobby by the hub
#[derive(Message)]
#[rtype(result = "()")]
pub struct Removed {
    pub config: PlayerConfig,
}

/// Session wants to the set the player's name
#[derive(Message)]
#[rtype(result = "()")]
pub struct PlayerName {
    pub id: Uuid,
    pub name: String,
}


pub enum CreateGameError {
    NameNotSet,
    MissingField,
    InvalidFieldValue(String), // contains the invalid field
    AlreadyAtTable(String), // contains the table name
}

impl fmt::Display for CreateGameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
	match self {
	    CreateGameError::NameNotSet => {
		write!(f, "Unable to create a game since you have not set your name")
	    },
	    CreateGameError::MissingField => {
		write!(f, "Unable to create a game since missing field(s)")
		/*
		write!(f, "Unable to create a game since command is missing fields:")?;
		for field in missing_fields {
		    write!(f, format!("{:?}", field))?;
		}
		Ok(())
		 */
	    },
	    CreateGameError::AlreadyAtTable(table_name) => {
		write!(
		    f,
		    "Unable to create a game since already at a table: {}",
		    table_name
		)
	    },
	    CreateGameError::InvalidFieldValue(invalid_field) => {
		write!(
		    f,
		    "Unable to create a game since invalid field value: {}",
		    invalid_field
		)
	    }
	}
    }
}


/// Session wants to create a game
#[derive(Message)]
#[rtype(result = "Result<String, CreateGameError>")]
pub struct Create {
    /// Client ID
    pub id: Uuid,
    pub create_msg: Value,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct PlayerActionMessage {
    // Client ID
    pub id: Uuid,

    pub player_action: PlayerAction,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct MetaActionMessage {
    pub id: Uuid,
    pub meta_action: MetaAction,
}
