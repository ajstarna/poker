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

use crate::messages::{ClientChatMessage, Connect, Disconnect, Join, ListTables, WsMessage};
use crate::{
    logic::{Game, PlayerSettings},
    messages::PlayerActionMessage,
};
use actix::prelude::{Actor, Context, Handler, MessageResult, Recipient};
use uuid::Uuid;

/// `Gamelobby` manages chat tables and responsible for coordinating chat session.
#[derive(Debug)]
pub struct GameLobby {
    // map from session id to the PlayerSettings for players that have connected but are not yet at a table
    main_lobby_connections: HashMap<Uuid, PlayerSettings>,

    // map from table name to a set of session ids
    //tables_to_session_ids: HashMap<String, HashSet<Uuid>>,

    // a map from session id to the table that it currently is in
    players_to_table: HashMap<Uuid, String>,

    tables_to_game: HashMap<String, Game>,

    visitor_count: Arc<AtomicUsize>,
}

impl GameLobby {
    pub fn new(visitor_count: Arc<AtomicUsize>) -> GameLobby {
        GameLobby {
            //sessions: HashMap::new(),
            //tables_to_session_ids: HashMap::new(),
            main_lobby_connections: HashMap::new(),
            players_to_table: HashMap::new(),
            tables_to_game: HashMap::new(),
            visitor_count,
        }
    }
}

impl GameLobby {
    /// Send message to all users in the table where the given player id is at
    fn send_message(&self, id: Uuid, message: &str) {
        /*
        if let Some(session_ids) = self.tables_to_session_ids.get(table) {
            for id in session_ids {
                if skip_id.is_none() | (*id != skip_id.unwrap()) {
                    if let Some(addr) = self.sessions.get(id) {
                        addr.do_send(WsMessage(message.to_owned()));
                    }
                }
            }
        }*/
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
        let id = uuid::Uuid::new_v4();
        // create a settings with name==None to start
        let player_settings = PlayerSettings::new(id, None, Some(msg.addr));

        self.main_lobby_connections.insert(id, player_settings); // put them in the main lobby to wait to join a table

        //self.players_to_table.insert(id, "main".to_owned());
        //let count = self.visitor_count.fetch_add(1, Ordering::SeqCst);
        //self.send_message("main", &format!("Total visitors {count}"), None);

        // send id back
        MessageResult(id)
    }
}

/*
    main_lobby_connections: HashMap<Uuid, PlayerSettings>,
    // a map from session id to the table that it currently is in
    players_to_table: HashMap<Uuid, String>,
    tables_to_game: HashMap<String, Game>,
    visitor_count: Arc<AtomicUsize>,
*/

/// Handler for Disconnect message.
impl Handler<Disconnect> for GameLobby {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        println!("Someone disconnected");

        self.main_lobby_connections.remove(&msg.id); // attempt to remove from the main lobby

        if let Some(table_name) = self.players_to_table.remove(&msg.id) {
            // the player was at a table, so tell the Game that the player left
            if let Some(game) = self.tables_to_game.get_mut(&table_name) {
                game.remove_player(msg.id);
		game.send_message("Someone disconnected");
            } else {
                // TODO: this should never happen. the player is allegedly at a table, but we
                // have no record of it in tables_to_game
            }
        }
    }
}

/// Handler for Message message.
impl Handler<ClientChatMessage> for GameLobby {
    type Result = ();

    fn handle(&mut self, msg: ClientChatMessage, _: &mut Context<Self>) {
        self.send_message(msg.id, msg.msg.as_str());
    }
}

/// Handler for `ListTables` message.
impl Handler<ListTables> for GameLobby {
    type Result = MessageResult<ListTables>;

    fn handle(&mut self, _: ListTables, _: &mut Context<Self>) -> Self::Result {
        let mut tables = Vec::new();

        for key in self.tables_to_game.keys() {
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
        let Join { id, table_name } = msg;

        if self.tables_to_game.contains_key(&table_name) {
            // for now, you cannot actually join (or create) a table that already exists
            return;
        }

        // TODO: if our player_settings.name is None, then we can't join a table!

        if let Some(old_table_name) = self.players_to_table.get(&id) {
            if *old_table_name != table_name {
                // we already exist at a table, so we must leave that table
                // we can unwrap since the mappings must always be in sync
                let game = self.tables_to_game.get_mut(old_table_name).unwrap();

                game.remove_player(id);
                game.send_message("Someone disconnected");
            }
        }

        // unwrap since how can they join a table if they were not in the lobby already?
        let player_settings = self.main_lobby_connections.remove(&id).unwrap();

        // update the mapping to find the player at a table
        self.players_to_table.insert(id, table_name.clone());

        let mut game = Game::new();
        game.add_user(player_settings);

        let num_bots = 5;
        for i in 0..num_bots {
            let name = format!("Mr {}", i);
            game.add_bot(name);
        }
	
        game.send_message("Someone connected");	
        self.tables_to_game.insert(table_name, game);

    }
}

/// Handler for Message message.
impl Handler<PlayerActionMessage> for GameLobby {
    type Result = ();

    fn handle(&mut self, msg: PlayerActionMessage, _: &mut Context<Self>) {}
}
