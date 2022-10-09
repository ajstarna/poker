//! `GameHub` is an actor. It keeps track of the current tables/games
//! and manages PlayerConfig structs (which include Ws Recipients)
//! When a WsMessage comes in from a WsGameSession, the GameHub routes the message to the proper Game

//! This file is adapted from the actix-web chat websocket example

use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
};

use crate::messages::{ClientChatMessage, Connect, Disconnect, Join, ListTables, PlayerName};
use crate::{
    logic::{Game, PlayerConfig},
    messages::PlayerActionMessage,
};
use actix::prelude::{Actor, Context, Handler, MessageResult};
use uuid::Uuid;

/// `Gamelobby` manages chat tables and responsible for coordinating chat session.
#[derive(Debug)]
pub struct GameHub {
    // map from session id to the PlayerConfig for players that have connected but are not yet at a table
    main_lobby_connections: HashMap<Uuid, PlayerConfig>,

    // map from table name to a set of session ids
    //tables_to_session_ids: HashMap<String, HashSet<Uuid>>,

    // a map from session id to the table that it currently is in
    players_to_table: HashMap<Uuid, String>,

    tables_to_game: HashMap<String, Game>,

    visitor_count: Arc<AtomicUsize>,
}

impl GameHub {
    pub fn new(visitor_count: Arc<AtomicUsize>) -> GameHub {
        GameHub {
            //sessions: HashMap::new(),
            //tables_to_session_ids: HashMap::new(),
            main_lobby_connections: HashMap::new(),
            players_to_table: HashMap::new(),
            tables_to_game: HashMap::new(),
            visitor_count,
        }
    }
}

/// Make actor from `GameHub`
impl Actor for GameHub {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for GameHub {
    type Result = MessageResult<Connect>; // use MessageResult so that we can return a Uuid

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("Someone joined");

        // register session with random id
        let id = uuid::Uuid::new_v4();
        // create a config with name==None to start
        let player_config = PlayerConfig::new(id, None, Some(msg.addr));

        self.main_lobby_connections.insert(id, player_config); // put them in the main lobby to wait to join a table

        //self.players_to_table.insert(id, "main".to_owned());
        //let count = self.visitor_count.fetch_add(1, Ordering::SeqCst);
        //self.send_message("main", &format!("Total visitors {count}"), None);

        // send id back
        MessageResult(id)
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for GameHub {
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
impl Handler<ClientChatMessage> for GameHub {
    type Result = ();

    fn handle(&mut self, msg: ClientChatMessage, _: &mut Context<Self>) {
        if let Some(table_name) = self.players_to_table.get(&msg.id) {
            // the player was at a table, so tell the Game to relay the message
            if let Some(game) = self.tables_to_game.get_mut(table_name) {
                game.send_message(msg.msg.as_str());
            } else {
                // TODO: this should never happen. the player is allegedly at a table, but we
                // have no record of it in tables_to_game
            }
        }
    }
}

/// Handler for `ListTables` message.
impl Handler<ListTables> for GameHub {
    type Result = MessageResult<ListTables>;

    fn handle(&mut self, _: ListTables, _: &mut Context<Self>) -> Self::Result {
        let mut tables = Vec::new();

        for key in self.tables_to_game.keys() {
            tables.push(key.to_owned())
        }

        MessageResult(tables)
    }
}

/// Handler for Message message.
impl Handler<PlayerName> for GameHub {
    type Result = ();

    fn handle(&mut self, msg: PlayerName, _: &mut Context<Self>) {
        // if the player is the main lobby, find them and set their name
        if let Some(player_config) = self.main_lobby_connections.get_mut(&msg.id) {
            player_config.name = Some(msg.name);
        } else if let Some(table_name) = self.players_to_table.remove(&msg.id) {
            // otherwise, find which game they are in, and tell the game there has been a name change
            // the player was at a table, so tell the Game that the player left
            if let Some(game) = self.tables_to_game.get_mut(&table_name) {
                game.set_player_name(msg.id, &msg.name);
            } else {
                // TODO: this should never happen. the player is allegedly at a table, but we
                // have no record of it in tables_to_game
            }
        } else {
            // player id not found anywhere. this should never happen
        }
    }
}

/// Join table, send disconnect message to old table
/// send join message to new table
impl Handler<Join> for GameHub {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) {
        let Join { id, table_name } = msg;

        if self.tables_to_game.contains_key(&table_name) {
            // for now, you cannot actually join (or create) a table that already exists
            return;
        }

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
        let player_config = self.main_lobby_connections.remove(&id).unwrap();

        // update the mapping to find the player at a table
        self.players_to_table.insert(id, table_name.clone());

        let mut game = Game::new();
        game.add_user(player_config);

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
impl Handler<PlayerActionMessage> for GameHub {
    type Result = ();

    /// the player has sent a message of what their next action in the game should be,
    /// so we need to relay that to the game
    fn handle(&mut self, msg: PlayerActionMessage, _: &mut Context<Self>) {
        if let Some(table_name) = self.players_to_table.get(&msg.id) {
            // the player was at a table, so tell the Game this player's message
            if let Some(game) = self.tables_to_game.get_mut(table_name) {
                game.set_player_action(msg.id, msg.player_action);
            } else {
                // TODO: this should never happen. the player is allegedly at a table, but we
                // have no record of it in tables_to_game
            }
        }
    }
	
}
