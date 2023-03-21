use crate::logic::{player::PlayerAction, PlayerConfig};
use actix::prelude::{Message, Recipient};
use std::fmt;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

/// this enum represents higher level commands that the hub will relay
/// to the running games Player name change. player join/leave
#[derive(Debug)]
pub enum MetaAction {
    Join(PlayerConfig, Option<String>), // player config and optional password
    UpdateAddress(Uuid, Recipient<WsMessage>), // update a player with an existing uuid and new message address
    Leave(Uuid),
    SitOut(Uuid),    
    ImBack(Uuid),
    SetPlayerName(Uuid, String),
    SendPlayerName(Uuid),    
    Chat(Uuid, String),
    Admin(Uuid, AdminCommand),
    TableInfo(Recipient<WsMessage>), // send the table info to the given address
}

/// these admin commands can be taken by the owner of a PRIVATE game.
/// commands to change the blinds, buy in, password, and to add or remove bots
/// The Uuid of the player attemping an admin command must actually be the game.admin to work
#[derive(Debug)]
pub enum AdminCommand {
    SmallBlind(u32),
    BigBlind(u32),
    BuyIn(u32),
    SetPassword(String),
    ShowPassword,    
    AddBot,
    RemoveBot,
    Restart,
    // NewAdmin(Uuid), // todo? would they give the name of the player or what?
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct WsMessage(pub String);

/// New ws session is created
#[derive(Message)]
#[rtype(result = "Uuid")]
pub struct Connect {
    pub id: Uuid,    
    pub addr: Recipient<WsMessage>,
}

/// Session is disconnected
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: Uuid,
}

/// List available tables and send info to the provided address
pub struct ListTables(pub Recipient<WsMessage>);

impl actix::Message for ListTables {
    type Result = Vec<String>;
}

#[derive(Debug)]
pub enum JoinTableError {
    GameIsFull,
    InvalidPassword,
    MissingPassword
}

impl fmt::Display for JoinTableError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            JoinTableError::GameIsFull => {
                write!(f, "Game is full.",)
            }
            JoinTableError::InvalidPassword => {
                write!(f, "Invalid password.")
            }
            JoinTableError::MissingPassword => {
                write!(f, "Password is required.")
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
    HeartBeatFailed,
    FailureToJoin(JoinTableError),
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

pub enum CreateTableError {
    NameNotSet,
    UnableToParseJson(String),
    PlayerDoesNotExist, // cannot be found in the lobby or at a table
    AlreadyAtTable(String),    // contains the table name
    TooManyBots,
    TooLargeBlinds,
}

impl fmt::Display for CreateTableError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CreateTableError::NameNotSet => {
                write!(f, "You have not set your name")
            }
            CreateTableError::UnableToParseJson(error_msg) => {
                write!(f, "Unable to parse json: {:?}", error_msg)
	    }
            CreateTableError::AlreadyAtTable(table_name) => {
                write!(f, "You are already at the table {}", table_name)
            }
            CreateTableError::PlayerDoesNotExist => {
                write!(f, "The game is unaware of you. Please try refreshing your browser.")
            }
            CreateTableError::TooManyBots => {
                write!(f, "Too many bots selected")
            }
            CreateTableError::TooLargeBlinds => {
                write!(f, "Blinds must be smaller than the starting stacks.")
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct CreateFields {
    pub max_players: u8,
    pub small_blind: u32,
    pub big_blind: u32,
    pub buy_in: u32,
    pub num_bots: u8,
    pub password: Option<String>,
}

/// Session wants to create a game
#[derive(Message)]
#[rtype(result = "Result<String, CreateTableError>")]
pub struct Create {
    /// Client ID
    pub id: Uuid,
    pub create_msg: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct PlayerActionMessage {
    // Client ID
    pub id: Uuid,

    pub player_action: PlayerAction,
}

/// the hub learns that a game has ended
#[derive(Message)]
#[rtype(result = "()")]
pub struct GameOver {
    pub table_name: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct MetaActionMessage {
    pub id: Uuid,
    pub meta_action: MetaAction,
}
