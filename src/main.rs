mod logic;

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

use actix::*;
use actix_files::{Files, NamedFile};
use actix_web::{
    middleware::Logger, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
};
use actix_web_actors::ws;

mod lobby;
mod messages;
mod session;

async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await.unwrap()
}

/// Entry point for our websocket route
async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    lob: web::Data<Addr<lobby::GameLobby>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        session::WsGameSession::new(lob.get_ref().clone()),
        &req,
        stream,
    )
}

/// Displays state
async fn get_count(count: web::Data<AtomicUsize>) -> impl Responder {
    let current_count = count.load(Ordering::SeqCst);
    format!("Visitors: {current_count}")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // set up applications state
    // keep a count of the number of visitors
    let app_state = Arc::new(AtomicUsize::new(0));

    // start chat server actor
    let lobby = lobby::GameLobby::new(app_state.clone()).start();

    log::info!("starting HTTP server at http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::from(app_state.clone()))
            .app_data(web::Data::new(lobby.clone()))
            .service(web::resource("/").to(index))
            .route("/count", web::get().to(get_count))
            .route("/ws", web::get().to(ws_route))
            .service(Files::new("/static", "./static"))
            .wrap(Logger::default())
    })
    .workers(2)
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

fn play() {
    println!("Hello, world!");
    let mut game = logic::Game::new();
    let num_bots = 5;
    for i in 0..num_bots {
        let name = format!("Mr {}", i);
        game.add_bot(name);
    }
    let user_name = "Adam".to_string();
    game.add_user(user_name);
    game.play();
}
