use crate::db;

use axum::{
    extract::{FromRequestParts, Request, State},
    handler::Handler,
    http::{StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use reqwest;
use rusqlite::Connection;
use serde::Deserialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tower_sessions::Session;
use rand::RngExt;


#[derive(Clone)]
pub struct AuthState {
    pub github_client_id: String,
    pub github_client_secret: String,
    pub mast_url: String,
    pub db: Arc<Mutex<Connection>>,
}

#[derive(Deserialize)]
struct CallbackParams {
    code: String,
    state: String,
}

pub async fn login(session: Session, State(state): State<AuthState>) -> impl IntoResponse {
    let csrf_state: String = rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    session.insert("oauth_state", &csrf_state).await.unwrap();

    let redirect_uri = format!("{}/api/auth/github/callback", state.mast_url);

    let auth_url = format!(
        "https://github.com/login/oauth/authorize?\
         client_id={}&redirect_uri={}&state={}&scope=read:user+user:email",
        state.github_client_id, redirect_uri, csrf_state
    );
    Redirect::to(&auth_url)
}

pub struct CallbackHandler;

impl Handler<((),), AuthState> for CallbackHandler
{
    type Future = Pin<Box<dyn Future<Output = Response> + Send>>;

    fn call(self, req: Request, state: AuthState) -> Self::Future {
        Box::pin(async move {
            let (mut parts, _body) = req.into_parts();

            let session: Session = Session::from_request_parts(&mut parts, &state).await.unwrap();
            let uri: Uri = Uri::from_request_parts(&mut parts, &state).await.unwrap();

            let query_str: &str = uri.query().unwrap_or("");
            let params: CallbackParams = serde_urlencoded::from_str(query_str).unwrap();

            // verify CSRF state
            let saved_state: Option<String> = session.get("oauth_state").await.unwrap();
            match saved_state {
                Some(s) if s == params.state => {}
                _ => return (StatusCode::FORBIDDEN, "state mismatch").into_response(),
            }
            session.remove::<String>("oauth_state").await.unwrap();
            
            let client: reqwest::Client = reqwest::Client::new();
            
            let token_resp: reqwest::Response = client
                .post("https://github.com/login/oauth/access_token")
                .header("Accept", "application/json")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(format!(
                    "client_id={}&client_secret={}&code={}",
                    state.github_client_id, state.github_client_secret, params.code
                ))
                .send()
                .await
                .unwrap();
            
            let token_body: serde_json::Value = token_resp.json().await.unwrap();
            let access_token = match token_body["access_token"].as_str() {
                Some(t) => t.to_string(),
                None => return (StatusCode::BAD_REQUEST, "no access_token in response").into_response(),
            };
            let user_resp = client
                .get("https://api.github.com/user")
                .header("Authorization", format!("Bearer {}", access_token))
                .header("User-Agent", "mast-wiki")
                .send()
                .await
                .unwrap();
            let gh_user: serde_json::Value = user_resp.json().await.unwrap();
            let github_id = gh_user["id"].as_i64().unwrap();
            let login = gh_user["login"].as_str().unwrap_or("unknown");
            let avatar_url = gh_user["avatar_url"].as_str();
            let email = gh_user["email"].as_str();
            let user_id = {
                let conn = state.db.lock().unwrap();
                let id = db::upsert_user(&conn, github_id, login, avatar_url, email);
                drop(conn);
                id
            };
            session.insert("user_id", user_id).await.unwrap();
            Redirect::to("/wiki/home").into_response()
        })
    }
}

impl Clone for CallbackHandler {
    fn clone(&self) -> Self {
        CallbackHandler
    }
}

pub async fn me(session: Session, State(state): State<AuthState>) -> Result<Json<serde_json::Value>, StatusCode> {
    
    let user_id: Option<i64> = session.get("user_id").await.unwrap();
    let user_id = user_id.ok_or(StatusCode::UNAUTHORIZED)?;
    let conn = state.db.lock().unwrap();
    let user = db::get_user(&conn, user_id);
    drop(conn);

    let (id, github_id, login, avatar_url, email) = user.ok_or(StatusCode::UNAUTHORIZED)?;
    Ok(Json(serde_json::json!({
        "id": id,
        "github_id": github_id,
        "login": login,
        "avatar_url": avatar_url,
        "email": email,
    })))
}

pub async fn logout(session: Session) -> impl IntoResponse {
    session.clear().await;
    StatusCode::OK
}