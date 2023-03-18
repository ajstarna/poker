use clap::Parser;

mod logic;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use actix::*;
use actix_files::{Files, NamedFile};
use actix_web::{
    get, web, middleware::Logger, App, Error, HttpRequest, HttpResponse, HttpServer, Responder, Result,
};
use actix_web_actors::ws;
use uuid::Uuid;

mod hub;
mod messages;
mod session;

const LOCAL_HOST: &str = "localhost";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// ip address
    #[arg(short, long, default_value_t = LOCAL_HOST.to_string())]
    ip: String,

    /// port
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}

async fn index() -> impl Responder {
    NamedFile::open_async("./site/index.html").await.unwrap()
}

/// Entry point for our websocket route
#[get("/join")]
async fn new_connection(
    req: HttpRequest,
    stream: web::Payload,
    hub_addr: web::Data<Addr<hub::GameHub>>,
) -> Result<HttpResponse, Error> {
    log::info!("inside new_connection()");
    ws::start(
        session::WsPlayerSession::new(hub_addr.get_ref().clone()),
        &req,
        stream,
    )
}

/// It is possible to try reconnecting with an existing Uuid.
/// useful for client-side caching
/// extract path info from "/ws/{uuid}" url
/// {uuid} - deserializes into a Uuid
#[get("/rejoin/{uuid}")]
async fn reconnect(
    path: web::Path<Uuid>,
    req: HttpRequest,
    stream: web::Payload,
    hub_addr: web::Data<Addr<hub::GameHub>>,
) -> Result<HttpResponse, Error> {
    let uuid = path.into_inner();
    ws::start(
        session::WsPlayerSession::from_existing(uuid, hub_addr.get_ref().clone()),
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
    let hub = hub::GameHub::new().start();

    log::info!("starting HTTP server at http://{}:{}", args.ip, args.port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::from(app_state.clone()))
            .app_data(web::Data::new(hub.clone()))
            .service(web::resource("/").to(index))
            .service(reconnect)
            .service(new_connection)	    
            .route("/count", web::get().to(get_count))
            .service(Files::new("/", "./site/"))
            .wrap(Logger::default())
    })
    .workers(2)
    .bind((args.ip, args.port))?
    .run()
    .await
}
