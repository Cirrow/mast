use crate::config::CFG;
use axum::{Json, http::StatusCode};
use git2;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tower_sessions::Session;

#[derive(Deserialize)]
pub struct SaveRequest {
    pub path: String,
    pub content: String,
    pub sha: String,
}

#[derive(Serialize)]
pub struct SaveResponse {
    pub sha: String,
}

pub async fn save(
    session: Session,
    Json(body): Json<SaveRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let (login, author_email) = if CFG.auth.edit_requires_auth {
        let username: String = session.get("username").await.unwrap().ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "unauthorized"})),
            )
        })?;
        (username.clone(), format!("{}@localhost", username))
    } else {
        ("guest".to_string(), "guest@localhost".to_string())
    };

    let base = CFG.base_dir.join(&CFG.storage.location);
    let canonical = crate::pages::safe_path(&base, &body.path)
        .map_err(|s| (s, Json(serde_json::json!({"error": "forbidden path"}))))?;

    let current_mtime = std::fs::metadata(&canonical)
        .and_then(|m| m.modified())
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "can't read file"})),
            )
        })?;

    let current_sha = chrono::DateTime::<chrono::Utc>::from(current_mtime)
        .timestamp()
        .to_string();

    if !body.sha.is_empty() && body.sha != current_sha {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "conflict — page was edited elsewhere"
            })),
        ));
    }

    std::fs::write(&canonical, &body.content).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "write failed"})),
        )
    })?;

    let repo_path = &CFG.base_dir;
    let repo: git2::Repository = git2::Repository::open(&repo_path).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "git repo not found"})),
        )
    })?;

    let mut index: git2::Index = repo.index().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "can't open git index"})),
        )
    })?;

    let rel_path: String = format!("{}/{}.txt", &CFG.storage.location, body.path);
    index
        .add_path(std::path::Path::new(&rel_path))
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "can't stage file"})),
            )
        })?;

    let tree_id: git2::Oid = index.write_tree().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "can't write tree"})),
        )
    })?;

    let tree = repo.find_tree(tree_id).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "can't find tree"})),
        )
    })?;

    let signature = git2::Signature::now(&login, &author_email).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "can't create signature"})),
        )
    })?;

    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();

    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        &format!("Edit {} via Mast web editor", body.path),
        &tree,
        &parents,
    )
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "commit failed"})),
        )
    })?;

    let new_mtime = std::fs::metadata(&canonical)
        .and_then(|m| m.modified())
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "can't read mtime"})),
            )
        })?;

    let new_sha = chrono::DateTime::<chrono::Utc>::from(new_mtime)
        .timestamp()
        .to_string();

    Ok(Json(SaveResponse { sha: new_sha }))
}
