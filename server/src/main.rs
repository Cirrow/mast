mod auth;
mod config;
mod pages;
mod routes;
mod save;

use pages::{get_config, serve_raw_page};

use axum::{
    Extension, Router,
    response::Redirect,
    routing::{get, post},
    serve,
};

use tower_http::services::ServeDir;
use tower_sessions::{MemoryStore, SessionManagerLayer, cookie::SameSite};

use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]

async fn main() {
    tracing_subscriber::fmt::init();
    dotenvy::from_filename("../.env.local").ok();

    let cfg: config::Config = config::Config::load();

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) // no HTTPS in dev
        .with_name("mast-session")
        .with_same_site(SameSite::Lax);

    async fn root() -> Redirect {
        Redirect::to("/wiki/home")
    }

    let app: Router = Router::new()
        //api calls
        .route("/api/raw/{*slug}", get(serve_raw_page))
        .route("/api/routes", get(routes::get_routes))
        .route("/api/config", get(get_config))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/save", post(save::save))
        //assets
        .route("/assets/{*path}", get(pages::serve_asset))
        //
        .route("/", get(root))
        //routings
        .route("/wiki/{*slug}", get(pages::serve_wiki_page))
        .route("/{slug}", get(pages::serve_static_page))
        //layers
        .layer(session_layer)
        .layer(Extension(cfg));

    let addr: SocketAddr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener: TcpListener = TcpListener::bind(addr).await.unwrap();

    println!("Mast is running on http://{}", addr);

    serve(listener, app).await.unwrap();
}
