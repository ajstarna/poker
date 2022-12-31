use crate::logic::{player::PlayerAction, PlayerConfig};
use actix::prelude::{Message, Recipient};
use serde_json::Value;
use std::fmt;
use uuid::Uuid;

/// Game server sends this messages to session

/// this enum represents higher level commands that the hub will relay
/// to the running games Player name change. player join/leave
#[derive(Debug)]
pub enum MetaAction {
    Join(PlayerConfig, Option<String>),
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

#[derive(Debug)]
pub enum JoinGameError {
    GameIsFull,
    InvalidPassword,
}

impl fmt::Display for JoinGameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            JoinGameError::GameIsFull => {
                write!(f, "Game is full",)
            }
            JoinGameError::InvalidPassword => {
                write!(f, "Invalid Password")
            }
        }
    }
}

/// Join table, if table does not exists create new one.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    /// Client ID
    pub id: Uuid,

    /// Table name
    pub table_name: String,

    pub password: Option<String>,
}

pub enum ReturnedReason {
    Left, // the player left
    FailureToJoin(JoinGameError),
}

/// the game sends this message when a player config has been returned to the hub
/// The playerconfig can be added back to the lobby by the hub
#[derive(Message)]
#[rtype(result = "()")]
pub struct Returned {
    pub config: PlayerConfig,
    pub reason: ReturnedReason,
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
    AlreadyAtTable(String),    // contains the table name
    TooManyBots,
    TooLargeBlinds,
}

impl fmt::Display for CreateGameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CreateGameError::NameNotSet => {
                write!(
                    f,
                    "You have not set your name"
                )
            }
            CreateGameError::MissingField => {
                write!(f, "Missing field(s)")
                /*
                write!(f, "Unable to create a game since command is missing fields:")?;
                for field in missing_fields {
                    write!(f, format!("{:?}", field))?;
                }
                Ok(())
                 */
            }
            CreateGameError::AlreadyAtTable(table_name) => {
                write!(f, "You are already at the table {}", table_name)
            }
            CreateGameError::InvalidFieldValue(invalid_field) => {
                write!(
                    f,
                    "Invalid field value: {}",
                    invalid_field
                )
            }
            CreateGameError::TooManyBots => {
                write!(f, "Too many bots selected")
            }
            CreateGameError::TooLargeBlinds => {
                write!(f, "Blinds must be smaller than the starting stacks.")
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
