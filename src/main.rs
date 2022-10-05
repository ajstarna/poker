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
use uuid::Uuid;

mod hub;
mod messages;
mod session;

async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await.unwrap()
}

/// Entry point for our websocket route
async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    hub_addr: web::Data<Addr<hub::GameHub>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        session::WsGameSession::new(hub_addr.get_ref().clone()),
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

    // start main hub actor
    let hub = hub::GameHub::new(app_state.clone()).start();

    log::info!("starting HTTP server at http://localhost:8080");

    play(); // TODO: remove this. for now just getting rid of dead code warnings

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::from(app_state.clone()))
            .app_data(web::Data::new(hub.clone()))
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
    let player_settings =
        logic::PlayerSettings::new(Uuid::new_v4(), Some("Adam".to_string()), None);
    game.add_user(player_settings);
    game.play();
}
