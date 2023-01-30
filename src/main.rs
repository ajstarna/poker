use clap::Parser;

mod logic;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use actix::*;
use actix_files::{Files, NamedFile};
use actix_web::{
    middleware::Logger, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
};
use actix_web_actors::ws;

mod hub;
mod messages;
mod session;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// ip address
    #[arg(short, long, default_value_t = ("localhost".to_string()))]
    ip: String,

    /// port
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}

async fn index() -> impl Responder {
    NamedFile::open_async("./static/index.html").await.unwrap()
}

/// Entry point for our websocket route
async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    hub_addr: web::Data<Addr<hub::GameHub>>,
) -> Result<HttpResponse, Error> {
    log::info!("inside ws_route()");
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
    let args = Args::parse();

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // set up applications state
    // keep a count of the number of visitors
    let app_state = Arc::new(AtomicUsize::new(0));

    // start main hub actor
    let hub = hub::GameHub::new(app_state.clone()).start();

    log::info!("starting HTTP server at http://{}:{}", args.ip, args.port);

    let paths = std::fs::read_dir("./").unwrap();
    for path in paths {
        log::info!("Name: {}", path.unwrap().path().display());
    }
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
    .bind((args.ip, args.port))?
    .run()
    .await
}
