//! `GameHub` is an actor. It keeps track of the current tables/games
//! and manages PlayerConfig structs (which include Ws Recipients)
//! When a WsMessage comes in from a WsGameSession, the GameHub routes the message to the proper Game

//! This file is adapted from the actix-web chat websocket example

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{atomic::AtomicUsize, Arc, Mutex},
};

use crate::logic::{Game, PlayerAction, PlayerConfig};
use crate::messages::{
    Connect, Create, CreateFields, CreateGameError, GameOver, Join, ListTables, MetaAction, MetaActionMessage,
    PlayerActionMessage, PlayerName, Returned, ReturnedReason, WsMessage,
};
use actix::prelude::{Actor, Context, Handler, MessageResult};
use actix::AsyncContext;
use json::object;
use rand::Rng;
use uuid::Uuid;

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

    private_tables: HashSet<String>, // which games do not show up in the loby

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
            private_tables: HashSet::new(),
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
/// Register new session with a given uuid.It could be brand new or a reconnection of an existing uuid
impl Handler<Connect> for GameHub {
    type Result = MessageResult<Connect>; // use MessageResult so that we can return a Uuid

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        let Connect { id, addr } = msg; // the message contains the uuid

        println!("Someone is connecting with uuid = {id}!");
	println!("self.main_lobby_connections = {:?}", self.main_lobby_connections);
	println!("self.players_to_table = {:?}", self.players_to_table);	

        let mut message = object! {
            msg_type: "connected".to_owned(),
            uuid: id.to_string().to_owned(),
	    name_set: false, //assume their name isn't set, unless we find out it is a re-connection with a name
        };

	let cloned_addr = addr.clone(); // since we are passing to the config, we need to keep it around for us
	if let Some(config) = self.main_lobby_connections.get_mut(&id) {
	    // the player happens to be in the lobby at this moment
	    // simply update the address in the player config
	    println!("connecting session uuid already in the lobby");
	    println!("{:?}", config);
	    if config.name.is_some() {
		// they were in the hub and already had a name!
		message["name_set"] = true.into();
	    }
	    config.player_addr = Some(addr);	    
	}
	else if let Some(table_name) = self.players_to_table.get(&id) {
	    // the player is currently at a table, so we need to tell the table
	    // that the player has a new address
	    message["name_set"] = true.into(); // if you are at a table, you must have a name
            if let Some(meta_actions) = self.tables_to_meta_actions.get_mut(table_name) {
                println!("updating player's address in an existing game");
                meta_actions
                    .lock()
                    .unwrap()
                    .push_back(MetaAction::UpdateAddress(id, addr));
            } else {
                // this should never happen. the player is allegedly at a table, but we
                // have no record of it in tables_to_meta_actions
                panic!(
                    "we can not find the meta actions for table named {:?}",
                    table_name
                );
	    }
	}
	else {
	    // we don't have a record of the uuid
	    // in either the lobby or in any existing table.
	    // This means it is a new session/player
            // create a config with name==None to start
            let player_config = PlayerConfig::new(id, None, Some(addr));
            // put them in the main lobby to wait to join a table
            self.main_lobby_connections.insert(id, player_config);
	}
	cloned_addr.do_send(WsMessage(message.dump())); // send the connection message	    	
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
            if self.private_tables.contains(key) {
                // don't return private tables
                continue;
            }
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
            let message = object! {
            msg_type: "name_changed".to_owned(),
            new_name: msg.name.clone(),
            };
            player_config
                .player_addr
                .as_ref()
                .unwrap()
                .do_send(WsMessage(message.dump()));
            player_config.name = Some(msg.name);
        } else if let Some(table_name) = self.players_to_table.get(&msg.id) {
            // otherwise, find which game they are in, and tell the game there has been a name change
            if let Some(meta_actions) = self.tables_to_meta_actions.get_mut(table_name) {
                println!("passing player name to the game");
                meta_actions
                    .lock()
                    .unwrap()
                    .push_back(MetaAction::PlayerName(msg.id, msg.name));
                println!("meta actions = {:?}", meta_actions);
            } else {
                // this should never happen. the player is allegedly at a table, but we
                // have no record of it in tables_to_meta_actions
                panic!(
                    "we can not find the meta actions for table named {:?}",
                    table_name
                );
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
        let Join {
            id,
            table_name,
            password,
        } = msg;
	
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
            let message = json::object! {
                    msg_type: "error".to_owned(),
            error: "unable_to_join".to_owned(),
                    reason: "You cannot join a game until you set your name!"
                };
            player_config
                .player_addr
                .as_ref()
                .unwrap()
                .do_send(WsMessage(message.dump()));
            // put them back in the lobby
            self.main_lobby_connections
                .insert(player_config.id, player_config);
            return;
        }

        // update the mapping to find the player at a table
        self.players_to_table.insert(id, table_name.clone());

        if let Some(meta_actions) = self.tables_to_meta_actions.get_mut(&table_name) {
            // since the meta actions already exist, this means the game already exists
            // so we can simply join it
            println!("joining existing game! {:?}", meta_actions);
            meta_actions
                .lock()
                .unwrap()
                .push_back(MetaAction::Join(player_config, password));
        } else {
            let message = json::object! {
                    msg_type: "error".to_owned(),
            error: "unable_to_join".to_owned(),
                    reason: format!("no table named {} exisits", table_name),
                };
            player_config
                .player_addr
                .as_ref()
                .unwrap()
                .do_send(WsMessage(message.dump()));
            // put them back in the lobby
            self.main_lobby_connections
                .insert(player_config.id, player_config);
        }
    }
}

/// Handler for a player that has been returned from a game officially
/// This message comes FROM a game and provides the config, which we can put back in the lobby`
impl Handler<Returned> for GameHub {
    type Result = ();

    fn handle(&mut self, msg: Returned, _: &mut Context<Self>) {
        let Returned { config, reason } = msg;
        println!("Handling player {:?} removed", config);
        if let Some(table_name) = self.players_to_table.remove(&config.id) {
            // we stil think this player is at table in our mapping, so remove it
            println!("removing player {:?} removed from {:?}", config, table_name);
        }

        // tell the player what happened (successful leave/why couldn't they join)

        if let Some(addr) = &config.player_addr {
            let mut message = object! {};
            match reason {
                ReturnedReason::Left => {
                    message["msg_type"] = "left_game".into();
                }
                ReturnedReason::FailureToJoin(err) => {
		    message["msg_type"] = "error".into();
                    message["error"] = "unable_to_join".into();		    
                    message["reason"] = err.to_string().into();
                }
            }
            addr.do_send(WsMessage(message.dump()));
        }

        // add the config back into the lobby
        self.main_lobby_connections.insert(config.id, config);
    }
}

/// create table, cannot already be at a table
impl Handler<Create> for GameHub {
    type Result = Result<String, CreateGameError>;

    /// creates a game and returns either Ok(table_name) or an Er(CreateGameError)
    /// if the player is not in the lobby or does not have their name set
    fn handle(&mut self, msg: Create, ctx: &mut Context<Self>) -> Self::Result {
        let Create { id, create_msg } = msg;

        let player_config_option = self.main_lobby_connections.remove(&id);
        if player_config_option.is_none() {
            // the player is not in the main lobby,
            // so we must be waiting for the game to remove the player still
            println!("player config not in the main lobby, so they must already be at a game");
            if let Some(table_name) = self.players_to_table.get(&id) {
                return Err(CreateGameError::AlreadyAtTable(table_name.to_string()));
            } else {
                println!("player not at lobby nor at a table");
                return Err(CreateGameError::AlreadyAtTable("unknown".to_string()));
            }
        }
        let player_config = player_config_option.unwrap();

        if player_config.name.is_none() {
            // they are not allowed to join a game without a Name set
            // put them back in the lobby
            self.main_lobby_connections
                .insert(player_config.id, player_config);
            return Err(CreateGameError::NameNotSet);
        }

	match serde_json::from_str(&create_msg) {
	    Ok(create_fields) => {
		let CreateFields {
		    max_players,
		    small_blind,
		    big_blind,
		    buy_in,
		    num_bots,
		    password,
		} = create_fields;
		println!("password in create game = {:?}", password);
		
		if num_bots >= max_players {
		    self.main_lobby_connections.insert(player_config.id, player_config);
		    return Err(CreateGameError::TooManyBots);
		}
		
		if big_blind > buy_in || small_blind > buy_in {
		    self.main_lobby_connections.insert(player_config.id, player_config);
		    return Err(CreateGameError::TooLargeBlinds);		
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
                    break genned_name;
		};
		
		let actions = Arc::new(Mutex::new(HashMap::new()));
		let meta_actions = Arc::new(Mutex::new(VecDeque::new()));
		let cloned_actions = actions.clone();
		let cloned_meta_actions = meta_actions.clone();
		
		let mut game = Game::new(
                    ctx.address(),
                    table_name.clone(),
                    None, // no deck needed to pass in
                    max_players,
                    small_blind,
                    big_blind,
                    buy_in,
                    password.clone(),
		    id, // the creator is the admin
		);
		
		for i in 0..num_bots {
                    let name = format!("Mr {}", i);
                    game.add_bot(name)
			.expect("error adding bot on freshly created game");
		}
		
		if password.is_some() {
		    // a game with a password does not show up as a public game
                    self.private_tables.insert(table_name.clone());
		}
		
		// update the mapping to find the player at a table
		self.players_to_table.insert(id, table_name.clone());

		meta_actions
                    .lock()
                    .unwrap()
                    .push_back(MetaAction::Join(player_config, password));
		
		std::thread::spawn(move || {
                    // Note: I tried having the actions and meta actions as part of the game struct,
                    // but this led to lifetime concerns.
                    // Then I changed to using scoped threads, and this sort of "solved" it,
                    // but it did not play nicely with actix async (i.e. the tests worked but the app did not)
                    // TLDR keep the actions as something passed in to play()
                    game.play(&cloned_actions, &cloned_meta_actions, None);
		});
		
		self.tables_to_actions.insert(table_name.clone(), actions);
		self.tables_to_meta_actions
                    .insert(table_name.clone(), meta_actions);
		Ok(table_name) // return the table name
            }
	    Err(e) => {
		println!("create message unable to deserialize");
		println!("{:?}", e);
		self.main_lobby_connections.insert(player_config.id, player_config);	    
		return Err(CreateGameError::UnableToParseJson(e.to_string()));
            }
	}
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
                actions_map
                    .lock()
                    .unwrap()
                    .insert(msg.id, msg.player_action);
                println!("actions map = {:?}", actions_map);
            } else {
                // TODO: this should never happen. the player is allegedly at a table, but we
                // have no record of it in tables_to_game
                println!("blah blah mp actioms queue!");
            }
        }
    }
}

/// the game tells us that it has ended (no more human players),
/// so lets remove it from our hub records
impl Handler<GameOver> for GameHub {
    type Result = ();

    fn handle(&mut self, msg: GameOver, _: &mut Context<Self>) {
        let GameOver { table_name } = msg;
        println!(
            "Handling game over in the hub for table name: {:?}",
            table_name
        );
        if self.tables_to_actions.remove(&table_name).is_some() {
            println!("removed properly from tables_to_actions");
        }
        if self.tables_to_meta_actions.remove(&table_name).is_some() {
            println!("removed properly from tables_to_meta_actions");
        }
        if self.private_tables.remove(&table_name) {
            println!("removed properly from private_tables");
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
