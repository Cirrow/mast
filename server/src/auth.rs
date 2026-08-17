use crate::config::CFG;
use crate::rate_limit::RateLimiter;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::{Json, extract::ConnectInfo, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;
use tower_sessions::Session;

static SIGNUP_LIMIT: LazyLock<RateLimiter> =
    LazyLock::new(|| RateLimiter::new(Duration::from_secs(3600), 10));
static LOGIN_IP_LIMIT: LazyLock<RateLimiter> =
    LazyLock::new(|| RateLimiter::new(Duration::from_secs(300), 5));
static LOGIN_USER_LIMIT: LazyLock<RateLimiter> =
    LazyLock::new(|| RateLimiter::new(Duration::from_secs(900), 10));

static USERS_LOCK: Mutex<()> = Mutex::new(());

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

#[derive(Deserialize)]
pub struct SignupRequest {
    pub username: String,
    pub email: Option<String>,
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
        "signup_enabled": CFG.auth.signup_enabled,
        "password_min_length": CFG.auth.signup_pwd_minimum_length,
        "home": format!(
            "{}{}",
            CFG.basic.wikipage_directory_prefix, CFG.basic.default_wikipage
        ),
    }))
}

pub(crate) fn load_users() -> HashMap<String, User> {
    let path = CFG.base_dir.join(&CFG.auth.users_file);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| toml::from_str(&c).ok())
        .unwrap_or_default()
}

pub async fn signup(
    session: Session,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<SignupRequest>,
) -> Result<Json<UserInfo>, (StatusCode, Json<serde_json::Value>)> {
    if !CFG.auth.signup_enabled {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "signup is disabled"})),
        ));
    }

    if !SIGNUP_LIMIT.hit(&addr.ip().to_string()) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({"error": "too many signup attempts"})),
        ));
    }

    if !CFG
        .auth
        .auth_methods
        .clone()
        .unwrap_or_default()
        .iter()
        .any(|m| m == "username")
    {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "username signup is not enabled"})),
        ));
    }

    let username = body.username.trim().to_string();
    let valid_username = username.len() >= 3
        && username.len() <= 32
        && username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid_username {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({"error": "username must be 3-32 chars: letters, numbers, '-' or '_'"
                }),
            ),
        ));
    }

    if body.password.len() < CFG.auth.signup_pwd_minimum_length as usize {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!(
                "password must be at least {} characters",
                CFG.auth.signup_pwd_minimum_length
            )})),
        ));
    }

    let email = body.email.as_ref().map(|e| e.trim().to_lowercase());
    if CFG.auth.username_signup_requires_email
        && email.as_deref().map(str::is_empty).unwrap_or(true)
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "email is required"})),
        ));
    }
    if let Some(e) = &email {
        if !valid_email(e) {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid email"})),
            ));
        }
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(body.password.as_bytes(), &salt)
        .map_err(|_| server_error())?
        .to_string();

    let _guard = USERS_LOCK.lock().unwrap();
    let path = CFG.base_dir.join(&CFG.auth.users_file);
    let mut content = std::fs::read_to_string(&path).unwrap_or_default();

    let users: HashMap<String, User> = toml::from_str(&content).unwrap_or_default();
    if users.keys().any(|k| k.eq_ignore_ascii_case(&username)) {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "username is already taken"})),
        ));
    }
    if let Some(e) = &email {
        if users.values().any(|u| {
            u.email
                .as_deref()
                .map(|ue| ue.eq_ignore_ascii_case(e))
                .unwrap_or(false)
        }) {
            return Err((
                StatusCode::CONFLICT,
                Json(serde_json::json!({"error": "email is already in use"})),
            ));
        }
    }

    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    let section = match email.as_ref() {
        Some(e) => format!(
            "[\"{username}\"]\npwd_hash = \"{hash}\"\nemail = {}\n",
            toml_valid_str(e)
        ),
        None => format!("[\"{username}\"]\npwd_hash = \"{hash}\"\n"),
    };
    content.push_str(&section);

    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &content).map_err(|_| server_error())?;
    std::fs::rename(&tmp, &path).map_err(|_| server_error())?;

    drop(_guard);

    session.insert("username", &username).await.unwrap();

    let user = User {
        pwd_hash: Some(hash),
        email,
        groups: None,
        userpv: HashMap::new(),
    };
    Ok(Json(user_to_info(username, &user)))
}

pub async fn login(
    session: Session,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<LocalLoginRequest>,
) -> Result<Json<UserInfo>, (StatusCode, Json<serde_json::Value>)> {
    let ip = addr.ip().to_string();
    if !LOGIN_IP_LIMIT.is_allowed(&ip) {
        return Err(too_many_requests());
    }

    // Resolve email-or-username to the canonical username (case-insensitive)
    let users = load_users();
    let username = if body.username.contains('@') {
        users
            .iter()
            .find(|(_, u)| {
                u.email
                    .as_deref()
                    .map(|e| e.eq_ignore_ascii_case(&body.username))
                    .unwrap_or(false)
            })
            .map(|(n, _)| n.clone())
            .ok_or_else(unauthorized)?
    } else {
        users
            .keys()
            .find(|k| k.eq_ignore_ascii_case(&body.username))
            .cloned()
            .ok_or_else(unauthorized)?
    };

    if !LOGIN_USER_LIMIT.is_allowed(&username) {
        return Err(too_many_requests());
    }

    let user = users.get(&username).ok_or_else(unauthorized)?;

    let hash = user.pwd_hash.as_ref().ok_or_else(unauthorized)?;
    let parsed = PasswordHash::new(hash).map_err(|_| server_error())?;

    if Argon2::default()
        .verify_password(body.password.as_bytes(), &parsed)
        .is_err()
    {
        LOGIN_IP_LIMIT.hit(&ip);
        LOGIN_USER_LIMIT.hit(&username);
        return Err(unauthorized());
    }

    LOGIN_IP_LIMIT.reset(&ip);
    LOGIN_USER_LIMIT.reset(&username);
    session.insert("username", &username).await.unwrap();
    Ok(Json(user_to_info(username, user)))
}

pub async fn me(session: Session) -> Result<Json<UserInfo>, StatusCode> {
    let username: Option<String> = session.get("username").await.unwrap_or(None);
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

pub(crate) fn toml_valid_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn valid_email(e: &str) -> bool {
    !e.is_empty()
        && e.len() <= 254
        && !e
            .chars()
            .any(|c| c.is_whitespace() || c == '"' || c == '\\')
        && e.split('@').count() == 2
        && {
            let (local, domain) = e.split_once('@').unwrap();
            !local.is_empty() && domain.contains('.')
        }
}

fn too_many_requests() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(serde_json::json!({"error": "too many attempts, try again later"})),
    )
}
