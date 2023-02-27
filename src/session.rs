//! This file is adapted from the actix-web chat websocket example

use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web_actors::ws;

use serde_json::Value;
use uuid::Uuid;

use crate::hub;
use crate::logic::player::PlayerAction;
use crate::messages;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub fn get_help_message() -> Vec<String> {
    vec!["/small_blind X".to_string(),
	 "/big_blind X".to_string(),
	 "/buy_in X".to_string(),
	 "/password X".to_string(),
	 "/add_bot".to_string(),
	 "/remove_bot".to_string()]
}

#[derive(Debug)]
pub struct WsGameSession {
    /// unique session id
    pub id: Uuid,

    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    pub hb: Instant,

    /// Game hub address
    pub hub_addr: Addr<hub::GameHub>,
}

impl WsGameSession {
    pub fn new(hub_addr: Addr<hub::GameHub>) -> Self {
        let id = Uuid::new_v4();
	println!("brand new uuid = {id}");
        Self {
            id: id,
            hb: Instant::now(),
            hub_addr,
        }
    }

    /// if the client wants to reconnect with an existing uuid
    pub fn from_existing(uuid: Uuid, hub_addr: Addr<hub::GameHub>) -> Self {
        Self {
            id: uuid,
            hb: Instant::now(),
            hub_addr,
        }
    }
    
    /// helper method that sends ping to client every 5 seconds (HEARTBEAT_INTERVAL).
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // notify game server. A Leave is the same thing for the game
                act.hub_addr.do_send(messages::MetaActionMessage {
                    id: act.id,
                    meta_action: messages::MetaAction::Leave(act.id),
                });

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Actor for WsGameSession {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start.
    /// We register ws session with GameServer
    fn started(&mut self, ctx: &mut Self::Context) {
        // we'll start heartbeat process on session start.
        self.hb(ctx);

        // register self in game server. `AsyncContext::wait` register
        // future within context, but context waits until this future resolves
        // before processing any other events.
        // HttpContext::state() is instance of WsGameSessionState, state is shared
        // across all routes within application
        let addr = ctx.address();
        self.hub_addr
            .send(messages::Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    // something is wrong with game server
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // notify game server. A Leave is the same thing for the game

	TODO i think this is where we should not send a leave message
	    
        self.hub_addr.do_send(messages::MetaActionMessage {
            id: self.id,
            meta_action: messages::MetaAction::Leave(self.id),
        });
        Running::Stop
    }
}

/// Handle messages from game server, we simply send it to peer websocket
impl Handler<messages::WsMessage> for WsGameSession {
    type Result = ();

    fn handle(&mut self, msg: messages::WsMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

/// WebSocket message handler
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsGameSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        log::debug!("WEBSOCKET MESSAGE: {msg:?}");
        //println!("WEBSOCKET MESSAGE: {:?}", msg);
        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(text) => {
                let m = text.trim();

                if let Ok(object) = serde_json::from_str(m) {
                    println!("parsed: {}", object);
                    self.handle_client_command(object, m, ctx);
                } else {
                    println!("message unable to parse as json: {}", m);
                };
            }
            ws::Message::Binary(_) => println!("Unexpected binary"),
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}

impl WsGameSession {
    fn handle_client_command(
        &mut self,
        object: Value,
	m: &str, // the original string in case we want to use it to parse
        ctx: &mut <WsGameSession as Actor>::Context,
    ) {
        println!("Entered handle_client_command {:?}", object);
        let msg_type_opt = object.get("msg_type");
        if msg_type_opt.is_none() {
            println!("missing message type!");
            return;
        }
        let msg_type = msg_type_opt.unwrap();
        match msg_type {
            Value::String(type_str) => match type_str.as_str() {
                "player_action" => {
                    self.handle_player_action(object, ctx);
                }
                "list" => {
                    self.handle_list_tables(ctx);
                }
                "join" => {
                    self.handle_join_table(object, ctx);
                }
                "create" => {
                    self.handle_create_table(m, ctx);
                }
                "admin_command" => {
                    self.handle_admin_command(object, ctx);
                }
                "leave" => {
                    self.hub_addr.do_send(messages::MetaActionMessage {
                        id: self.id,
                        meta_action: messages::MetaAction::Leave(self.id),
                    });
                }
                "sitout" => {
                    self.hub_addr.do_send(messages::MetaActionMessage {
                        id: self.id,
                        meta_action: messages::MetaAction::SitOut(self.id),
                    });
                }
                "imback" => {
                    self.hub_addr.do_send(messages::MetaActionMessage {
                        id: self.id,
                        meta_action: messages::MetaAction::ImBack(self.id),
                    });
                }
                "name" => {
                    self.handle_player_name(object, ctx);
                }
                "chat" => {
                    self.handle_chat(object, ctx);
                }
		"help" => {
                    let message = json::object! {
			msg_type: "help_message".to_owned(),
			commands: get_help_message(),
                    };
                    ctx.text(message.dump());
		}
                _ => ctx.text(format!("!!! unknown command: {:?}", object)),
            },
            _ => ctx.text(format!("!!! improper msg_type in: {:?}", object)),
        }
    }

    fn handle_create_table(&self, msg: &str, ctx: &mut <WsGameSession as Actor>::Context) {
        self.hub_addr
            .send(messages::Create {
                id: self.id,
                create_msg: msg.into(),
            })
            .into_actor(self)
            .then(|res, _, ctx| {
                match res {
                    Ok(create_game_result) => match create_game_result {
                        Ok(table_name) => {
                            println!("created game = {}", table_name);
                            let message = json::object! {
                                msg_type: "created_game".to_owned(),
                                table_name: table_name,
                            };
                            ctx.text(message.dump());
                        }
                        Err(e) => {
                            println!("{}", e);
                            let message = json::object! {
                                            msg_type: "error".to_owned(),
                            error: "unable_to_create".to_owned(),
                                            reason: e.to_string(),
                                        };
                            ctx.text(message.dump());
                        }
                    },
                    _ => println!("MailBox error"),
                }
                fut::ready(())
            })
            .wait(ctx)
        // .wait(ctx) pauses all events in context,
        // so actor wont receive any new messages until it get list
        // of tables back
    }
    
    fn handle_list_tables(&self, ctx: &mut <WsGameSession as Actor>::Context) {
        // Send ListTables message to game server and wait for
        // response
        println!("List tables");
        self.hub_addr
            .send(messages::ListTables)
            .into_actor(self)
            .then(|res, _, ctx| {
                match res {
                    Ok(tables) => {
                        let message = json::object! {
                            msg_type: "tables_list".to_owned(),
                            tables: tables,
                        };
                        ctx.text(message.dump());
                    }
                    _ => println!("Something is wrong"),
                }
                fut::ready(())
            })
            .wait(ctx)
        // .wait(ctx) pauses all events in context,
        // so actor wont receive any new messages until it get list
        // of tables back
    }

    fn handle_join_table(&self, object: Value, ctx: &mut <WsGameSession as Actor>::Context) {
        if let (Some(Value::String(table_name)), Some(password)) =
            (object.get("table_name"), object.get("password"))
        {
            let table_name = table_name.to_string();
            let password = if password.is_string() {
                Some(password.to_string())
            } else {
                None
            };

            self.hub_addr.do_send(messages::Join {
                id: self.id,
                table_name,
                password,
            });
        } else {
            println!("missing table name or password!");
            ctx.text("!!! table_name and password (possibly null) are required");
        }
    }

    fn handle_player_action(&self, object: Value, ctx: &mut <WsGameSession as Actor>::Context) {
        if let Some(Value::String(player_action)) = object.get("action") {
            let player_action = player_action.to_string();
            match player_action.as_str() {
                "check" => {
                    self.hub_addr.do_send(messages::PlayerActionMessage {
                        id: self.id,
                        player_action: PlayerAction::Check,
                    });
                }
                "fold" => {
                    self.hub_addr.do_send(messages::PlayerActionMessage {
                        id: self.id,
                        player_action: PlayerAction::Fold,
                    });
                }
                "call" => {
                    self.hub_addr.do_send(messages::PlayerActionMessage {
                        id: self.id,
                        player_action: PlayerAction::Call,
                    });
                }
                "bet" => {
                    if let Some(Value::String(amount)) = object.get("amount") {
                        let amount = amount.to_string();
                        self.hub_addr.do_send(messages::PlayerActionMessage {
                            id: self.id,
                            player_action: PlayerAction::Bet(amount.parse::<u32>().unwrap()),
                        });
                    //ctx.text(format!("placing bet of: {:?}", v[1]));
                    } else {
                        ctx.text("!!!You much specify how much to bet!");
                    }
                }
                other => {
                    ctx.text(format!(
                        "invalid action set for type:player_action: {:?}",
                        other
                    ));
                }
            }
        } else {
            ctx.text("!!! action is required");
        }
    }

    fn handle_player_name(&self, object: Value, ctx: &mut <WsGameSession as Actor>::Context) {
        if let Some(Value::String(name)) = object.get("player_name") {
            println!("{}", name);
            self.hub_addr.do_send(messages::PlayerName {
                id: self.id,
                name: name.to_string(),
            });
        } else {
            ctx.text("!!! player_name is required");
        }
    }

    fn handle_chat(&self, object: Value, ctx: &mut <WsGameSession as Actor>::Context) {
        if let Some(Value::String(text)) = object.get("text") {
            let text = text.to_string();
            self.hub_addr.do_send(messages::MetaActionMessage {
                id: self.id,
                meta_action: messages::MetaAction::Chat(self.id, text),
            })
        } else {
            println!("missing chat_message!");
            ctx.text("!!! chat_message is required");
        }
    }

    // e.g. {"msg_type": "admin_command", "admin_command": "big_blind", "big_blind": 24}
    fn handle_admin_command(&self, object: Value, ctx: &mut <WsGameSession as Actor>::Context) {
        if let Some(Value::String(admin_command)) = object.get("admin_command") {
            let invalid_json =  match admin_command.as_str() {
                "small_blind" => {
		    if let Some(Value::String(amount)) = object.get("small_blind") {
			if let Ok(amount) = amount.to_string().parse::<u32>() {
			    self.hub_addr.do_send(messages::MetaActionMessage {
				id: self.id,
				meta_action: messages::MetaAction::Admin(
				    self.id,				
				    messages::AdminCommand::SmallBlind(amount),
				)
			    });
			    false
			} else {
			    true
			}
		    } else {
			// invalid_json
			true
		    }
                }
                "big_blind" => {
		    if let Some(Value::String(amount)) = object.get("big_blind") {
			if let Ok(amount) = amount.to_string().parse::<u32>() {			
			    self.hub_addr.do_send(messages::MetaActionMessage {
				id: self.id,
				meta_action: messages::MetaAction::Admin(
				    self.id,				
				    messages::AdminCommand::BigBlind(amount),
				)
			    });
			    false			    
			} else {
			    true
			}
		    } else {
			// invalid_json			
			true
		    }
                }
                "buy_in" => {
		    if let Some(Value::String(amount)) = object.get("buy_in") {
			if let Ok(amount) = amount.to_string().parse::<u32>() {	
			    self.hub_addr.do_send(messages::MetaActionMessage {
				id: self.id,
				meta_action: messages::MetaAction::Admin(
				    self.id,				
				    messages::AdminCommand::BuyIn(amount),
				)
			    });
			    false
			} else {
			    true
			}
		    } else {
			// invalid json
			true
		    }		    
                }		
                "password" => {
		    if let Some(Value::String(amount)) = object.get("password") {
			let amount = amount.to_string();
			self.hub_addr.do_send(messages::MetaActionMessage {
			    id: self.id,
			    meta_action: messages::MetaAction::Admin(
				self.id,				
				messages::AdminCommand::Password(amount),
			    )
			});
			false
		    } else {
			// invalid json
			true
		    }
                }

                "add_bot" => {
		    self.hub_addr.do_send(messages::MetaActionMessage {
			id: self.id,
			meta_action: messages::MetaAction::Admin(
			    self.id,				
			    messages::AdminCommand::AddBot),
                    });
		    false
		}
                "remove_bot" => {
		    self.hub_addr.do_send(messages::MetaActionMessage {
			id: self.id,
			meta_action: messages::MetaAction::Admin(
			    self.id,				
			    messages::AdminCommand::RemoveBot),
                    });
		    false
                }
                "restart" => {
		    self.hub_addr.do_send(messages::MetaActionMessage {
			id: self.id,
			meta_action: messages::MetaAction::Admin(
			    self.id,				
			    messages::AdminCommand::Restart),
                    });
		    false
                }
                _ => {
		    // invalid command
		    true 
                }
            };
	    if invalid_json {
                let message = json::object! {
                    msg_type: "error".to_owned(),
		    error: "invalid_admin_command".to_owned(),
                    reason: "this admin_command was invalid.".to_owned(),
                };
                ctx.text(message.dump());
		
	    }
	}
    }
    

    
}
