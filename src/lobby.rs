//! `Gamelobby` is an actor. It maintains list of connection client session.
//! And manages available tables. Peers send messages to other peers in same
//! table through `Gamelobby`.

//! This file is adapted from the actix-web chat websocket example

use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use crate::{logic::Game, messages::PlayerActionMessage};
use crate::messages::{ClientChatMessage, Connect, Disconnect, Join, ListTables, WsMessage};
use actix::prelude::{Actor, Context, Handler, MessageResult, Recipient};
use uuid::Uuid;

/// `Gamelobby` manages chat tables and responsible for coordinating chat session.
#[derive(Debug)]
pub struct GameLobby {
    // map from session id to the connection
    sessions: HashMap<Uuid, Recipient<WsMessage>>,

    // map from table name to a set of session ids
    tables_to_session_ids: HashMap<String, HashSet<Uuid>>,

    // a map from session id to the table that it currently is in
    players_to_table: HashMap<Uuid, String>,


    TODO: i am pretty sure we wanna move the sessions into the Game itself,
    // that way i guess the game can write the messages
    And we can get rid of almost all the redundant HashMaps that need to stay in sync?
    tables_to_game: HashMap<String, Game>,

    visitor_count: Arc<AtomicUsize>,
}

/*

TODO: is this all it would be???
the game would need to know its own name
each player would have its own connection, so the lobby would just relay commands to the proper game based on the sesssion id and thats it?
struct GameLobby {
players_to_game: HashMap<Uuid, Game>
}
*/

impl GameLobby {
    pub fn new(visitor_count: Arc<AtomicUsize>) -> GameLobby {
        GameLobby {
            sessions: HashMap::new(),
            tables_to_session_ids: HashMap::new(),
            players_to_table: HashMap::new(),
            tables_to_game: HashMap::new(),
            visitor_count,
        }
    }
}

impl GameLobby {
    /// Send message to all users in the table
    TODO i think this method should be in the game too?  After it owns the connections
	So each player shouldhave their id and their connection?
	So the lobby would simply have the mapping from ids to their games basically?
    fn send_message(&self, table: &str, message: &str, skip_id: Option<Uuid>) {
        if let Some(session_ids) = self.tables_to_session_ids.get(table) {
            for id in session_ids {
                if skip_id.is_none() | (*id != skip_id.unwrap()) {
                    if let Some(addr) = self.sessions.get(id) {
                        addr.do_send(WsMessage(message.to_owned()));
                    }
                }
            }
        }
    }

    fn set_player_action(&self, ) {
	tell the game that this player is at that the player has entered a move. the game sets player.current_action
    }
}

/// Make actor from `GameLobby`
impl Actor for GameLobby {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for GameLobby {
    type Result = MessageResult<Connect>; // use MessageResult so that we can return a Uuid

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("Someone joined");

        // register session with random id
        let id = uuid::Uuid::new_v4(); // .rng.gen::<usize>();
        self.sessions.insert(id, msg.addr);

        // auto join session to main table
        self.tables_to_session_ids
            .entry("main".to_owned())
            .or_insert_with(HashSet::new)
            .insert(id);
        self.players_to_table.insert(id, "main".to_owned());
        let count = self.visitor_count.fetch_add(1, Ordering::SeqCst);
        self.send_message("main", &format!("Total visitors {count}"), None);

        // send id back
        MessageResult(id)
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for GameLobby {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        println!("Someone disconnected");

        let mut tables: Vec<String> = Vec::new();
        // remove address
        if self.sessions.remove(&msg.id).is_some() {
            // remove session from all tables
            for (name, sessions) in &mut self.tables_to_session_ids {
                if sessions.remove(&msg.id) {
                    tables.push(name.to_owned());
                }
            }
        }
        // send message to other users
        for table in tables {
            self.send_message(&table, "Someone disconnected", None);
        }
    }
}

/// Handler for Message message.
impl Handler<ClientChatMessage> for GameLobby {
    type Result = ();

    fn handle(&mut self, msg: ClientChatMessage, _: &mut Context<Self>) {
        self.send_message(&msg.table, msg.msg.as_str(), Some(msg.id));
    }
}

/// Handler for `ListTables` message.
impl Handler<ListTables> for GameLobby {
    type Result = MessageResult<ListTables>;

    fn handle(&mut self, _: ListTables, _: &mut Context<Self>) -> Self::Result {
        let mut tables = Vec::new();

        for key in self.tables_to_session_ids.keys() {
            tables.push(key.to_owned())
        }

        MessageResult(tables)
    }
}

/// Join table, send disconnect message to old table
/// send join message to new table
impl Handler<Join> for GameLobby {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) {
        let Join {
            id,
            table_name,
            player_name,
        } = msg;

        if self.tables_to_session_ids.contains_key(&table_name) {
            // for now, you cannot actually join (or create) a table that already exists
            return;
        }

        if let Some(table_name) = self.players_to_table.get(&id) {
            // we already exist at a table, so we must leave that table
            // we can unwrap since the mappings must always be in sync
            let sessions = self.tables_to_session_ids.get_mut(table_name).unwrap();
            sessions.remove(&id);
            self.send_message(table_name, "Someone disconnected", None);
        }

        // update the mapping to find the player at a table
        self.players_to_table.insert(id, table_name.clone());

        self.tables_to_session_ids
            .entry(table_name.clone())
            .or_insert_with(HashSet::new)
            .insert(id);

        self.send_message(&table_name, "Someone connected", Some(id));

        let mut game = Game::new();
        if let Some(player_name) = player_name {
            game.add_user(player_name);
        } else {
            game.add_user("player".to_string());
        }

        let num_bots = 5;
        for i in 0..num_bots {
            let name = format!("Mr {}", i);
            game.add_bot(name);
        }

        self.tables_to_game.insert(table_name, game);
    }
}


/// Handler for Message message.
impl Handler<PlayerActionMessage> for GameLobby {
    type Result = ();

    fn handle(&mut self, msg: PlayerActionMessage, _: &mut Context<Self>) {
        self.set_player_action(&msg.table, msg.player_action., Some(msg.id));
    }
}
