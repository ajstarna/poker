use actix::prelude::{Message, Recipient};
/// Game server sends this messages to session
use uuid::Uuid;
use crate::logic::player::PlayerAction;

#[derive(Message)]
#[rtype(result = "()")]
pub struct WsMessage(pub String);

/// Message for ws communications

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

/// Send message to specific table
#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientChatMessage {
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

#[derive(Message)]
#[rtype(result = "()")]
pub struct PlayerActionMessage {
    // Client ID
    pub id: Uuid,
    
    pub player_action: PlayerAction,
}