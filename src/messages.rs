use crate::logic::{player::PlayerAction, PlayerConfig};
use actix::prelude::{Message, Recipient};
use uuid::Uuid;

/// Game server sends this messages to session

/// this enum represents higher level commands that the hub will relay
/// to the running games Player name change. player join/leave
#[derive(Debug)]
pub enum MetaAction {
    Join(PlayerConfig),
    Leave(Uuid),  // disconnect can also just use leave
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


/*
/// Game should start
#[derive(Message)]
#[rtype(result = "()")]
pub struct StartGame {
    pub id: Uuid, // player session id
}
*/

/// Send message to specific table
#[derive(Message)]
#[rtype(result = "()")]
pub struct Chat {
    /// Id of the client session
    pub id: Uuid,
    /// Peer message
    pub msg: String,
    // Table name
    //pub table: String,
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

/// If you are at a table, leave it.
/// Any money that you ahve already committed to the pot is lost, and you will fold out
#[derive(Message)]
#[rtype(result = "()")]
pub struct Leave {
    /// Client ID
    pub id: Uuid,
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

#[derive(Message)]
#[rtype(result = "()")]
pub struct PlayerActionMessage {
    // Client ID
    pub id: Uuid,

    pub player_action: PlayerAction,
}
