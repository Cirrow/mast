mod auth;
mod config;
mod iac;
mod install;
mod pages;
mod routes;
mod save;
use pages::serve_raw_page;

use axum::{
    Router,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    serve,
};

use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_sessions::{MemoryStore, SessionManagerLayer, cookie::SameSite};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenvy::from_filename("../.env.local").ok();
    config::init();

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
        // auth api calls
        .route("/api/auth/config", get(auth::config))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/signup", post(auth::signup))
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/save", post(save::save))
        //assets
        .route("/assets/{*path}", get(pages::serve_asset))
        //
        .route("/", get(root))
        .route(
            "/install",
            get(install::serve_install_page).post(install::handle_install),
        )
        //routings
        .route("/wiki/{*slug}", get(pages::serve_wiki_page))
        .route("/user/{username}", get(pages::serve_user_page))
        .route("/{slug}", get(pages::serve_static_page))
        //layers
        .layer(session_layer)
        .layer(axum::middleware::from_fn(install_guard));

    let addr: SocketAddr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener: TcpListener = TcpListener::bind(addr).await.unwrap();

    println!("Mast is running on http://{}", addr);

    serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();

    async fn install_guard(
        uri: axum::http::Uri,
        req: axum::extract::Request,
        next: axum::middleware::Next,
    ) -> impl IntoResponse {
        let path = uri.path();
        if path.starts_with("/install") || path.starts_with("/assets") {
            return next.run(req).await.into_response();
        }
        if !config::initcheck() {
            return Redirect::to("/install").into_response();
        }
        next.run(req).await.into_response()
    }
}
