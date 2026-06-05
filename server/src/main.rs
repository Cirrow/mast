mod config;
mod pages;
mod auth;
mod db;

use pages::get_page;

use std::sync::{Arc, Mutex};
use rusqlite::Connection;

use axum::{Router, routing::{get, post}, serve};

use tower_http::services::ServeDir;
use tower_sessions::{MemoryStore, SessionManagerLayer};

use std::net::SocketAddr;
use tokio::net::TcpListener;


#[tokio::main]

async fn main() {
    tracing_subscriber::fmt::init();

     let github_client_id: String = std::env::var("OAUTH_GITHUB_CLIENT_ID")
        .expect("OAUTH_GITHUB_CLIENT_ID not set");
    let github_client_secret: String = std::env::var("OAUTH_GITHUB_CLIENT_SECRET")
        .expect("OAUTH_GITHUB_CLIENT_SECRET not set");
    let mast_url: String = std::env::var("MAST_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());


    let conn: Connection = db::init_db("../.wiki/auth.db");
    let db: Arc<Mutex<Connection>> = Arc::new(Mutex::new(conn));


    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)       // no HTTPS in dev
        .with_name("mast-session");

    let auth_state: auth::AuthState = auth::AuthState {
        github_client_id,
        github_client_secret,
        mast_url,
        db,
    };

    let app: Router = Router::new()
        .route("/api/pages/{*path}", get(get_page))
        .route("/api/auth/github/login", get(auth::login))
        .route("/api/auth/github/callback", get(auth::callback))
        .route("/api/auth/me", get(auth::me))
        //.route("/api/auth/logout", post(auth::logout))
        .layer(session_layer)
        .with_state(auth_state)
        .fallback_service(ServeDir::new("../dist/client"));

    let addr: SocketAddr = SocketAddr::from(([0,0,0,0], 3000));
    let listener: TcpListener = TcpListener::bind(addr).await.unwrap();
    
    println!("Mast is running on http://{}", addr);

    serve(listener, app).await.unwrap();
}