use axum::{extract::Path, http::StatusCode, Json};
use serde::Serialize;
use std::path::PathBuf;
use chrono;

#[derive(Serialize)]
pub struct PageResponse {
    pub content: String,
    pub html: String,
    pub sha: String,
    pub last: Last,
}

#[derive(Serialize)]
pub struct Last {
    pub updated: String,
    pub committer: String,
    pub commit_sha: String,
}

pub async fn get_page( Path(path): Path<String>, )
    -> Result<Json<PageResponse>, StatusCode> {

        let base: PathBuf = PathBuf::from("../.wiki/wiki");
        let file_path: PathBuf = base.join(format!("{}.txt", path));

        let canonical:PathBuf= file_path.canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
        if !canonical.starts_with(&base.canonicalize().map_err(|_| StatusCode::NOT_FOUND)?) {
           return Err(StatusCode::FORBIDDEN);
        }

        let content: String = std::fs::read_to_string(&canonical)
            .map_err(|_| StatusCode::NOT_FOUND)?;
        let metadata: std::fs::Metadata = std::fs::metadata(&canonical)
            .map_err(|_| StatusCode::NOT_FOUND)?;
        let mtime: chrono::DateTime<chrono::Utc> = chrono::DateTime::from(
            metadata.modified().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        );

        let html = converter::render_page(&content).html;

    Ok(Json(PageResponse {
        content,
        html,
        sha: mtime.timestamp().to_string(),
        last: Last {
            updated: mtime.to_rfc3339(),
            committer: String::new(),
            commit_sha: String::new(),
        },
    }))
    
    
}