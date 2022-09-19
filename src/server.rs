//! `GameServer` is an actor. It maintains list of connection client session.
//! And manages available tables. Peers send messages to other peers in same
//! table through `GameServer`.

//! This file is adapted from the actix-web chat websocket example



use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use actix::prelude::*;
use rand::{self, rngs::ThreadRng, Rng};

/// Game server sends this messages to session
#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

/// Message for chat server communications

/// New chat session is created
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Message>,
}

/// Session is disconnected
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

/// Send message to specific table
#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    /// Id of the client session
    pub id: usize,
    /// Peer message
    pub msg: String,
    /// Table name
    pub table: String,
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
    pub id: usize,

    /// Table name
    pub name: String,
}

/// `GameServer` manages chat tables and responsible for coordinating chat session.
///
/// Implementation is very naïve.
#[derive(Debug)]
pub struct GameServer {
    sessions: HashMap<usize, Recipient<Message>>,
    tables: HashMap<String, HashSet<usize>>,
    rng: ThreadRng,
    visitor_count: Arc<AtomicUsize>,
}

impl GameServer {
    pub fn new(visitor_count: Arc<AtomicUsize>) -> GameServer {
        // default table
        let mut tables = HashMap::new();
        tables.insert("main".to_owned(), HashSet::new());

        GameServer {
            sessions: HashMap::new(),
            tables,
            rng: rand::thread_rng(),
            visitor_count,
        }
    }
}

impl GameServer {
    /// Send message to all users in the table
    fn send_message(&self, table: &str, message: &str, skip_id: usize) {
        if let Some(sessions) = self.tables.get(table) {
            for id in sessions {
                if *id != skip_id {
                    if let Some(addr) = self.sessions.get(id) {
                        addr.do_send(Message(message.to_owned()));
                    }
                }
            }
        }
    }
}

/// Make actor from `GameServer`
impl Actor for GameServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for GameServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("Someone joined");

        // notify all users in same table
        self.send_message("main", "Someone joined", 0);

        // register session with random id
        let id = self.rng.gen::<usize>();
        self.sessions.insert(id, msg.addr);

        // auto join session to main table
        self.tables
            .entry("main".to_owned())
            .or_insert_with(HashSet::new)
            .insert(id);

        let count = self.visitor_count.fetch_add(1, Ordering::SeqCst);
        self.send_message("main", &format!("Total visitors {count}"), 0);

        // send id back
        id
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for GameServer {
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
            self.send_message(&table, "Someone disconnected", 0);
        }
    }
}

/// Handler for Message message.
impl Handler<ClientMessage> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        self.send_message(&msg.table, msg.msg.as_str(), msg.id);
    }
}

/// Handler for `ListTables` message.
impl Handler<ListTables> for GameServer {
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
impl Handler<Join> for GameServer {
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
            self.send_message(&table, "Someone disconnected", 0);
        }

        self.tables
            .entry(name.clone())
            .or_insert_with(HashSet::new)
            .insert(id);

        self.send_message(&name, "Someone connected", id);
    }
}
