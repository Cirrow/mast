use crate::config::CFG;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tower_sessions::Session;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct User {
    pub pwd_hash: Option<String>,
    pub email: Option<String>,
    pub groups: Option<Vec<String>>,
    #[serde(default)]
    pub userpv: HashMap<u8, Vec<String>>,
}

#[derive(Deserialize)]
pub struct LocalLoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct UserInfo {
    pub username: String,
    pub email: Option<String>,
    pub groups: Vec<String>,
    pub userpv: HashMap<u8, Vec<String>>,
}

pub async fn config() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "auth_methods": CFG.auth.auth_methods.clone().unwrap_or_default(),
        "username_signup_requires_email": CFG.auth.username_signup_requires_email,
    }))
}

pub(crate) fn load_users() -> HashMap<String, User> {
    let path = CFG.base_dir.join(&CFG.auth.users_file);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| toml::from_str(&c).ok())
        .unwrap_or_default()
}

pub async fn login(
    session: Session,
    Json(body): Json<LocalLoginRequest>,
) -> Result<Json<UserInfo>, (StatusCode, Json<serde_json::Value>)> {
    // Resolve email input to a username
    let username = if body.username.contains('@') {
        load_users()
            .iter()
            .find(|(_, u)| u.email.as_deref() == Some(&body.username))
            .map(|(n, _)| n.clone())
            .ok_or_else(|| unauthorized())?
    } else {
        body.username.clone()
    };

    let users = load_users();
    let user = users.get(&username).ok_or_else(unauthorized)?;

    let hash = user.pwd_hash.as_ref().ok_or_else(unauthorized)?;
    let parsed = PasswordHash::new(hash).map_err(|_| server_error())?;

    if Argon2::default()
        .verify_password(body.password.as_bytes(), &parsed)
        .is_err()
    {
        return Err(unauthorized());
    }

    session.insert("username", &username).await.unwrap();
    Ok(Json(user_to_info(username, user)))
}

pub async fn me(session: Session) -> Result<Json<UserInfo>, StatusCode> {
    let username: Option<String> = session.get("username").await.unwrap();
    let username = username.ok_or(StatusCode::UNAUTHORIZED)?;
    let users = load_users();
    let user = users.get(&username).ok_or(StatusCode::UNAUTHORIZED)?;
    Ok(Json(user_to_info(username, user)))
}

pub async fn logout(session: Session) -> impl IntoResponse {
    session.clear().await;
    StatusCode::OK
}

fn unauthorized() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": "invalid credentials"})),
    )
}

fn server_error() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": "server error"})),
    )
}

fn user_to_info(username: String, user: &User) -> UserInfo {
    UserInfo {
        username,
        email: user.email.clone(),
        groups: user.groups.clone().unwrap_or_default(),
        userpv: user.userpv.clone(),
    }
}
