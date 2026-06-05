use std::sync::{Arc, Mutex};
use axum::{
    extract::{Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Redirect},
    Json,
};
use rand::{Rng, RngExt};
use reqwest;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;


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

#[derive(Serialize)]
struct UserResponse {
    id: i64,
    github_id: i64,
    login: String,
    email: Option<String>,
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

pub async fn callback(
    session: Session,
    State(state): State<AuthState>,
    Query(params): Query<CallbackParams>,
) -> impl IntoResponse {
    // verify CSRF state
    let saved_state: Option<String> = session.get("oauth_state").await.unwrap();
    match saved_state {
        Some(ref s) if s == ¶ms.state => {}
        _ => return (StatusCode::FORBIDDEN, "state mismatch").into_response(),
    }
    session.remove::<String>("oauth_state").await.unwrap();
    // exchange code for access token
    let client = reqwest::Client::new();
    let token_resp = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", &state.github_client_id),
            ("client_secret", &state.github_client_secret),
            ("code", ¶ms.code),
        ])
        .send()
        .await
        .unwrap();
    let token_body: serde_json::Value = token_resp.json().await.unwrap();
    let access_token = match token_body["access_token"].as_str() {
        Some(t) => t.to_string(),
        None => return (StatusCode::BAD_REQUEST, "no access_token in response").into_response(),
    };
    
    // fetch GitHub user
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
    let email = gh_user["email"].as_str();

    // upsert into local db
    let conn = state.db.lock().unwrap();
    let user_id = db::upsert_user(&conn, github_id, login, email);
    drop(conn);

    session.insert("user_id", user_id).await.unwrap();
    Redirect::to("/wiki/home").into_response()
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
        "email": email,
    })))
}