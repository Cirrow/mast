use axum::{Extension, Json, extract::Path, http::StatusCode, response::Html};
use chrono;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::OnceLock;

static CFG: LazyLock<crate::config::Config> = LazyLock::new(|| crate::config::Config::load());
static SHELL: OnceLock<String> = OnceLock::new();

#[derive(Serialize)]
pub struct PageResponse {
    pub html: String,
    pub sha: String,
    pub last: Last,
    pub status: u16,
}

#[derive(Serialize)]
pub struct RawPageResponse {
    pub content: String,
    pub sha: String,
    pub status: u16,
}

#[derive(Serialize)]
pub struct Last {
    pub updated: String,
    pub committer: String,
    pub commit_sha: String,
}

fn mtime_info(path: &std::path::Path) -> (String, String) {
    match std::fs::metadata(path).and_then(|m| m.modified()) {
        Ok(mtime) => {
            let dt: chrono::DateTime<chrono::Utc> = chrono::DateTime::from(mtime);
            (dt.timestamp().to_string(), dt.to_rfc3339())
        }
        Err(_) => (String::new(), String::new()),
    }
}

fn get_shell() -> &'static String {
    SHELL.get_or_init(|| {
        let path = CFG.base_dir.join(&CFG.shell.shell);
        std::fs::read_to_string(&path).expect("Failed to load shell")
    })
}

fn inject(content: &str) -> String {
    get_shell().replace("<!--MAST-CONTENT-->", content)
}

pub async fn serve_edit_page() -> Html<String> {
    let edit_path = CFG.base_dir.join("src/edit.html");
    let edit_content = std::fs::read_to_string(&edit_path).expect("Failed to load edit.html");
    let page = inject(&edit_content);
    Html(page)
}

pub async fn serve_wiki_page(Path(slug): Path<String>) -> Html<String> {
    let file_path = CFG
        .base_dir
        .join(&CFG.storage.location)
        .join(format!("{}.txt", slug));
    let content = std::fs::read_to_string(&file_path).unwrap_or_default();
    let html = converter::render_page(&content).html;
    Html(inject(&html))
}

pub async fn serve_raw_page(Path(slug): Path<String>) -> Json<RawPageResponse> {
    let file_path = PathBuf::from(&CFG.storage.location).join(format!("{}.txt", slug));
    match std::fs::read_to_string(&file_path) {
        Ok(content) => {
            let (sha, _) = mtime_info(&file_path);
            Json(RawPageResponse {
                content,
                sha,
                status: StatusCode::OK.as_u16(),
            })
        }
        Err(_) => Json(RawPageResponse {
            content: String::new(),
            sha: String::new(),
            status: StatusCode::NOT_FOUND.as_u16(),
        }),
    }
}

pub async fn serve_html_page(
    Extension(cfg): Extension<crate::config::Config>,
    Path(slug): Path<String>,
) -> Json<PageResponse> {
    let file_path = PathBuf::from(&cfg.storage.location).join(format!("{}.txt", slug));
    match std::fs::read_to_string(&file_path) {
        Ok(content) => {
            let html = converter::render_page(&content).html;
            let (sha, updated) = mtime_info(&file_path);
            Json(PageResponse {
                html,
                sha,
                last: Last {
                    updated,
                    committer: String::new(),
                    commit_sha: String::new(),
                },
                status: StatusCode::OK.as_u16(),
            })
        }
        Err(_) => Json(PageResponse {
            html: String::new(),
            sha: String::new(),
            last: Last {
                updated: String::new(),
                committer: String::new(),
                commit_sha: String::new(),
            },
            status: StatusCode::NOT_FOUND.as_u16(),
        }),
    }
}

pub async fn get_config(
    Extension(cfg): Extension<crate::config::Config>,
) -> Json<crate::config::Config> {
    Json(cfg)
}
