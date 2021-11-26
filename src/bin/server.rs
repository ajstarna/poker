use std::{net::TcpListener, thread::spawn};
use tungstenite::{
    accept_hdr,
    handshake::server::{Request, Response},
    Message,
};

use poker::{Player, Game};

fn main() {
    //env_logger::init();
    let server = TcpListener::bind("127.0.0.1:3012").unwrap();
    for stream in server.incoming() {
        spawn(move || {
            let callback = |req: &Request, mut response: Response| {
                println!("Received a new ws handshake");
                println!("The request's path is: {}", req.uri().path());
                println!("The request's headers are:");
                for (ref header, _value) in req.headers() {
                    println!("* {}", header);
                }

                // Let's add an additional header to our response to the client.
                let headers = response.headers_mut();
                headers.append("MyCustomHeader", ":)".parse().unwrap());
                headers.append("SOME_TUNGSTENITE_HEADER", "header_value".parse().unwrap());

                Ok(response)
            };
            let mut websocket = accept_hdr(stream.unwrap(), callback).unwrap();

            loop {
                let msg = websocket.read_message().unwrap();
		if msg == Message::Text("start game".to_string()) {
		    let new_msg = Message::Text("about to play a game".into());
                    websocket.write_message(new_msg).unwrap();
		    /*
		    let mut game = Game::new(websocket);
		    let num_bots = 2;
		    for i in 0..num_bots {
			let name = format!("Mr {}", i);
			game.add_player(Player::new(name));
		    }
		    let name = "Adam".to_string();
		    let user_player = Player::new_human(name);
		    game.add_player(user_player);
		    game.play();
		    let after_msg = Message::Text(format!("{:?}", &game.players));
		     */
		    let after_msg = Message::Text("end of game area".to_string());		    
                    websocket.write_message(after_msg).unwrap();
		    
		} else if msg.is_binary() || msg.is_text() {
		    let new_msg = Message::Text(format!("{} plus some stuff hah", msg));
                    websocket.write_message(new_msg).unwrap();
                }
            }
        });
    }
}
