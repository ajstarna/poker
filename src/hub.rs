//! `GameHub` is an actor. It keeps track of the current tables/games
//! and manages PlayerConfig structs (which include Ws Recipients)
//! When a WsMessage comes in from a WsGameSession, the GameHub routes the message to the proper Game

//! This file is adapted from the actix-web chat websocket example
 
use std::{
    thread,
    collections::{HashMap, VecDeque},
    sync::{atomic::AtomicUsize, Arc, Mutex},
};

use crate::messages::{WsMessage, MetaAction, MetaActionMessage, Connect, Create, CreateGameError,
		      Join, Removed, ListTables, PlayerName, PlayerActionMessage};
use crate::logic::{Game, PlayerConfig, PlayerAction};
use actix::AsyncContext;
use actix::prelude::{Actor, Context, Handler, MessageResult};
use uuid::Uuid;
use rand::Rng;
use json::object;
use serde_json::Value;

// for generator random game names
const CHAR_SET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const GAME_NAME_LEN: usize = 4;

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
	
	// put them in the main lobby to wait to join a table
        self.main_lobby_connections.insert(id, player_config); 

        // send id back
        MessageResult(id)
    }
}

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

/// Handler for PlayerName message.
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

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) {
        let Join { id, table_name } = msg;
	
        let player_config_option = self.main_lobby_connections.remove(&id);
	if player_config_option.is_none() {
	    // the player is not in the main lobby,
	    // so we must be waiting for the game to remove the player still
	    println!("player config not in the main lobby, so they must already be at a game");
	    return;
	} 
	let player_config = player_config_option.unwrap();		

	if player_config.name.is_none() {
	    // they are not allowed to join a game without a Name set
            player_config.player_addr.as_ref().unwrap()
		.do_send(
		    WsMessage(format!("You cannot join a game until you set your name!"))
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
        } else {
            player_config.player_addr.as_ref().unwrap()
		.do_send(
		    WsMessage(format!("No table with that name exists to join!"))
		);
	    // put them back in the lobby
	    self.main_lobby_connections.insert(player_config.id, player_config);
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


/// create table, cannot already be at a table
impl Handler<Create> for GameHub {
    type Result = Result<String, CreateGameError>;

    /// creates a game and returns either Ok(table_name) or an Er(CreateGameError)
    /// if the player is not in the lobby or does not have their name set
    fn handle(&mut self, msg: Create, ctx: &mut Context<Self>) -> Self::Result {
        let Create { id, create_msg } = msg;

	if let (Some(max_players),
		Some(small_blind),
		Some(big_blind),
		Some(buy_in),
		Some(is_private),
		Some(password)) = (create_msg.get("max_players"),
				   create_msg.get("small_blind"),
				   create_msg.get("big_blind"),
				   create_msg.get("buy_in"),
				   create_msg.get("is_private"),
				   create_msg.get("password")) 	{
	    let max_players = max_players.to_string().parse::<u32>().map_err(|_| CreateGameError)?;
		
	} else {
	    println!("create message missing one or more required fields!");
	    return Err(CreateGameError);
	}
	    
	
        let player_config_option = self.main_lobby_connections.remove(&id);
	if player_config_option.is_none() {
	    // the player is not in the main lobby,
	    // so we must be waiting for the game to remove the player still
	    println!("player config not in the main lobby, so they must already be at a game");
	    return Err(CreateGameError);
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
	    return Err(CreateGameError);	    	    
	}

	let mut rng = rand::thread_rng();	
	let table_name = loop {
	    // create a new 4-char unique name for the table
	    let genned_name: String = (0..GAME_NAME_LEN)
		.map(|_| {
		    let idx = rng.gen_range(0..CHAR_SET.len());
		    CHAR_SET[idx] as char
		})
		.collect();
            if self.tables_to_actions.contains_key(&genned_name) {
		// unlikely, but we already have a table with this exact name
		continue;
	    }
	    // we genned a name that is new
	    break genned_name
	};
	
        // update the mapping to find the player at a table	
        self.players_to_table.insert(id, table_name.clone());
	
	// TODO get all this info in the Create message to pass in
        let mut game = Game::new(
	    Some(ctx.address()),
	    table_name.clone(),
	    None,
	    9,
	    4,
	    8,
	    1000,
	    false,
	    None,
	);

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
        self.tables_to_meta_actions.insert(table_name.clone(), meta_actions);
        Ok(table_name)
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

/// Handler for MetaAction messages.
/// The types of meta actions inside a MetaAction message should simply be
/// passed on to the game (if one exists)
impl Handler<MetaActionMessage> for GameHub {
    type Result = ();

    fn handle(&mut self, msg: MetaActionMessage, _: &mut Context<Self>) {
	let MetaActionMessage { id, meta_action } = msg;
	println!("handling MetaActionMessage in the hub! {:?}", meta_action);	
        if let Some(table_name) = self.players_to_table.get(&id) {
	    // tell the table that a player is gone
           if let Some(meta_actions) = self.tables_to_meta_actions.get_mut(table_name) {
	       meta_actions.lock().unwrap().push_back(meta_action);
           } else {
 	       // this should not happen since the meta actions vec should be created at the same time as the game
	   }
	}
    }
}

