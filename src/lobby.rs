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

use actix::prelude::{Actor, Context, Handler, Recipient, MessageResult};
use rand::{self, rngs::ThreadRng, Rng};
use crate::messages::{WsMessage, Connect, Disconnect, ClientMessage, ListTables, Join};

use uuid::Uuid;

/// `Gamelobby` manages chat tables and responsible for coordinating chat session.
#[derive(Debug)]
pub struct GameLobby {
    // map from session id to the connection
    sessions: HashMap<Uuid, Recipient<WsMessage>>,

    // map from table name to a set of session ids
    tables: HashMap<String, HashSet<Uuid>>,
    
    visitor_count: Arc<AtomicUsize>,
}

impl GameLobby {
    pub fn new(visitor_count: Arc<AtomicUsize>) -> GameLobby {
 
        GameLobby {
            sessions: HashMap::new(),
            tables: HashMap::new(),
            visitor_count,
        }
    }
}

impl GameLobby {
    /// Send message to all users in the table
    fn send_message(&self, table: &str, message: &str, skip_id: Option<Uuid>) {
        if let Some(session_ids) = self.tables.get(table) {
            for id in session_ids {
                if skip_id.is_none() | (*id != skip_id.unwrap()) {
                    if let Some(addr) = self.sessions.get(id) {
                        addr.do_send(WsMessage(message.to_owned()));
                    }
                }
            }
        }
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
        self.tables
            .entry("main".to_owned())
            .or_insert_with(HashSet::new)
            .insert(id);

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
            for (name, sessions) in &mut self.tables {
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
impl Handler<ClientMessage> for GameLobby {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        self.send_message(&msg.table, msg.msg.as_str(), Some(msg.id));
    }
}

/// Handler for `ListTables` message.
impl Handler<ListTables> for GameLobby {
    type Result = MessageResult<ListTables>;

    fn handle(&mut self, _: ListTables, _: &mut Context<Self>) -> Self::Result {
        let mut tables = Vec::new();

        for key in self.tables.keys() {
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
        let Join { id, name } = msg;
        let mut tables = Vec::new();

        // remove session from all tables
        for (n, sessions) in &mut self.tables {
            if sessions.remove(&id) {
                tables.push(n.to_owned());
            }
        }
        // send message to other users
        for table in tables {
            self.send_message(&table, "Someone disconnected", None);
        }

        self.tables
            .entry(name.clone())
            .or_insert_with(HashSet::new)
            .insert(id);

        self.send_message(&name, "Someone connected", Some(id));
    }
}
