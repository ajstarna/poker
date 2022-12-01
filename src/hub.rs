//! `GameHub` is an actor. It keeps track of the current tables/games
//! and manages PlayerConfig structs (which include Ws Recipients)
//! When a WsMessage comes in from a WsGameSession, the GameHub routes the message to the proper Game

//! This file is adapted from the actix-web chat websocket example
 
use std::{
    thread,
    collections::{HashMap, VecDeque},
    sync::{atomic::AtomicUsize, Arc, Mutex},
};

use crate::messages::{WsMessage, MetaAction, Chat, Connect, Disconnect, Join, Leave, Removed, ListTables, PlayerName};
use crate::{
    logic::{Game, PlayerConfig, PlayerAction},
    messages::PlayerActionMessage,
};
use actix::AsyncContext;
use actix::prelude::{Actor, Context, Handler, MessageResult};
use uuid::Uuid;

/// `Gamelobby` manages chat tables and responsible for coordinating chat session.
#[derive(Debug)]
pub struct GameHub {
    // map from session id to the PlayerConfig for players that have connected but are not at a table
    main_lobby_connections: HashMap<Uuid, PlayerConfig>,

    // a map from session id to the table that it currently is in
    players_to_table: HashMap<Uuid, String>,

    // this is where the hub can add incoming player actions for a running game to grab from
    tables_to_actions: HashMap<String, Arc<Mutex<HashMap<Uuid, PlayerAction>>>>,

    tables_to_meta_actions: HashMap<String, Arc<Mutex<VecDeque<MetaAction>>>>,        

    visitor_count: Arc<AtomicUsize>,
}

impl GameHub {
    pub fn new(visitor_count: Arc<AtomicUsize>) -> GameHub {
        GameHub {
            //sessions: HashMap::new(),
            //tables_to_session_ids: HashMap::new(),
            main_lobby_connections: HashMap::new(),
            players_to_table: HashMap::new(),
	    tables_to_actions: HashMap::new(),
	    tables_to_meta_actions: HashMap::new(),	    
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

        // send id back
        MessageResult(id)
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for GameHub {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
	println!("handling Disconnect in the hub!");	
        self.main_lobby_connections.remove(&msg.id); // attempt to remove from the main lobby
        if let Some(table_name) = self.players_to_table.get(&msg.id) {
	    // tell the table that a player is gone
           if let Some(meta_actions) = self.tables_to_meta_actions.get_mut(table_name) {
	       println!("passing leave (due to disconnect) to the game!");
	       meta_actions.lock().unwrap().push_back(MetaAction::Leave(msg.id));
	       println!("meta actions = {:?}", meta_actions);	       
           } else {
 	       // this should not happen since the meta actions vec should be created at the same time as the game
	   }
	}
    }
}

/// Handler for Chat message.
impl Handler<Chat> for GameHub {
    type Result = ();

    fn handle(&mut self, msg: Chat, _: &mut Context<Self>) {
        if let Some(table_name) = self.players_to_table.get(&msg.id) {
            // the player was at a table, so tell the Game to relay the message
           if let Some(meta_actions) = self.tables_to_meta_actions.get_mut(table_name) {
	       println!("handling chat message in the hub!");
	       println!("meta actions = {:?}", meta_actions);
	       meta_actions.lock().unwrap().push_back(MetaAction::Chat(msg.id, msg.msg));
            } else {
                // TODO: this should never happen. the player is allegedly at a table, but we
                // have no record of it in tables_to_meta_actions
            }
        }
    }
}


/*
/// Handler for StartGame message.
impl Handler<StartGame> for GameHub {
    type Result = ();

    fn handle(&mut self, msg: StartGame, _: &mut Context<Self>) {
        if let Some(table_name) = self.players_to_table.get(&msg.id) {
            // the player was at a table, so tell the Game to relay the message
            if let Some(game) = self.tables_to_game.get_mut(table_name) {
		//TODO this call to play locks execution until it returns. So we can't get user input,
		//or anything hmm
            } else {
                // TODO: this should never happen. the player is allegedly at a table, but we
                // have no record of it in tables_to_game
            }
        }
    }
}
 */

/// Handler for `ListTables` message.
impl Handler<ListTables> for GameHub {
    type Result = MessageResult<ListTables>;

    fn handle(&mut self, _: ListTables, _: &mut Context<Self>) -> Self::Result {
        let mut tables = Vec::new();

        for key in self.tables_to_actions.keys() {
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
	    println!("setting player name in the main lobby");
            player_config.player_addr.as_ref().unwrap()
		.do_send(
		    WsMessage(format!("You are changing your name to {:?}", msg.name))
		);	    
            player_config.name = Some(msg.name);
        } else if let Some(table_name) = self.players_to_table.get(&msg.id) {
            // otherwise, find which game they are in, and tell the game there has been a name change
            if let Some(meta_actions) = self.tables_to_meta_actions.get_mut(table_name) {
		println!("passing player name to the game");
		meta_actions.lock().unwrap().push_back(MetaAction::PlayerName(msg.id, msg.name));
		println!("meta actions = {:?}", meta_actions);		
            } else {
                // TODO: this should never happen. the player is allegedly at a table, but we
                // have no record of it in tables_to_meta_actions
		panic!("we can not find the meta actions for table named {:?}", table_name);
            }
        } else {
            // player id not found anywhere. this should never happen
	    panic!("how can we set a name if no config exists anywhere!");	    
        }
    }
}

/// Join table, send disconnect message to old table
/// send join message to new table
impl Handler<Join> for GameHub {
    type Result = ();

    fn handle(&mut self, msg: Join, ctx: &mut Context<Self>) {
        let Join { id, table_name } = msg;
	
        let player_config_option = self.main_lobby_connections.remove(&id);
	if player_config_option.is_none() {
	    // the player is not in the main lobby, so we must be waiting for the game to remove the player still
	    println!("player config not in the main lobby, so they must already be at a game");
	    return;
	} 
	let player_config = player_config_option.unwrap();		

	if player_config.name.is_none() {
	    // they are not allowed to join a game without a Name set
            player_config.player_addr.as_ref().unwrap()
		.do_send(
		    WsMessage(format!("You cannt join a game until you set your name!"))
		);
	    // put them back in the lobby
	    self.main_lobby_connections.insert(player_config.id, player_config);
	    return
	}

        // update the mapping to find the player at a table	
        self.players_to_table.insert(id, table_name.clone());

        if let Some(meta_actions) = self.tables_to_meta_actions.get_mut(&table_name) {
	    // since the meta actions already exist, this means the game already exists
	    // so we can simply join it
	    println!("joining existing game!");
	    meta_actions.lock().unwrap().push_back(MetaAction::Join(player_config));
	    println!("meta actions = {:?}", meta_actions);
        } else {
	    // we need to create the game for the first time
            let mut game = Game::new(Some(ctx.address()), None);
	    
            let num_bots = 2;
            for i in 0..num_bots {
		let name = format!("Mr {}", i);
		game.add_bot(name);
            }

            if game.add_user(player_config).is_none() {
		panic!("how were we unable to join a fresh game?");
	    } else {
		println!("in the hub. we just joined fine?");
	    }
	    
	    let actions = Arc::new(Mutex::new(HashMap::new()));	
	    let cloned_actions = actions.clone();
	    
	    let meta_actions = Arc::new(Mutex::new(VecDeque::new()));
	    let cloned_meta_actions = meta_actions.clone();
	    //let b: bool = cloned_queue;
	    thread::spawn(move || {
		// start a game with no hand limit
		game.play(&cloned_actions, &cloned_meta_actions, None);
	    });
	    
            self.tables_to_actions.insert(table_name.clone(), actions);
            self.tables_to_meta_actions.insert(table_name, meta_actions);	
	}
    }    
}

/// Handler for leaving a table (if we are even at one)
impl Handler<Leave> for GameHub {
    type Result = ();

    fn handle(&mut self, msg: Leave, _: &mut Context<Self>) {
	println!("handling leave message in the hub!");	
        if let Some(table_name) = self.players_to_table.get(&msg.id) {
	    // tell the table that we want to leave
           if let Some(meta_actions) = self.tables_to_meta_actions.get_mut(table_name) {
	       println!("passing leave to the game!");
	       meta_actions.lock().unwrap().push_back(MetaAction::Leave(msg.id));
	       println!("meta actions = {:?}", meta_actions);	       
           } else {
 	       // this should not happen since the meta actions vec should be created at the same time as the game
	   }
	}
    }
}

/// Handler for a player that has been removed from a game officially
/// This message comes FROM a game and provides the config, which we can put back in the lobby`
impl Handler<Removed> for GameHub {
    type Result = ();

    fn handle(&mut self, msg: Removed, _: &mut Context<Self>) {
	println!("Handling player {:?} removed", msg.config);	
        if let Some(table_name) = self.players_to_table.remove(&msg.config.id) {
	    // we stil think this player is at table in our mapping, so remove it
	    println!("removing player {:?} removed from {:?}", msg.config, table_name);
	}
	// add the config back into the lobby
        if let Some(addr) = &msg.config.player_addr {
            addr.do_send(WsMessage("You have left the game and are back in the lobby".to_owned()));
        }
	
	self.main_lobby_connections.insert(msg.config.id, msg.config);
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
            if let Some(actions_map) = self.tables_to_actions.get_mut(table_name) {
		println!("handling player action in the hub!");
		actions_map.lock().unwrap().insert(msg.id, msg.player_action);
		println!("actions map = {:?}", actions_map);		
            } else {
                // TODO: this should never happen. the player is allegedly at a table, but we
                // have no record of it in tables_to_game
		println!("blah blah mp actioms queue!");
            }

        }
    }
	
}
