use crate::config::{self, CFG};
use crate::iac::{self, PV};
use axum::{Json, extract::Path, http::StatusCode, response::Html};
use chrono;
use http::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::Serialize;
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::OnceLock;
use tower_sessions::Session;

static SHELL: OnceLock<String> = OnceLock::new();

#[derive(Debug, Clone, Default)]
pub struct PageContext {
    pub username: Option<String>,
}

#[derive(Serialize)]
pub struct RawPageResponse {
    pub content: String,
    pub sha: String,
    pub status: u16,
}

pub fn safe_path(base: &std::path::Path, user_path: &str) -> Result<PathBuf, StatusCode> {
    let base_canonical = base
        .canonicalize()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let joined = base.join(user_path);
    let leaf = joined.file_name().ok_or(StatusCode::FORBIDDEN)?;
    if leaf == std::ffi::OsStr::new("..") || leaf == std::ffi::OsStr::new(".") {
        return Err(StatusCode::FORBIDDEN);
    }

    let parent = joined.parent().unwrap_or(&base_canonical);
    let parent_canonical = parent.canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    parent_canonical
        .strip_prefix(&base_canonical)
        .map_err(|_| StatusCode::FORBIDDEN)?;

    let final_path = parent_canonical.join(leaf);
    match final_path.canonicalize() {
        Ok(c) => {
            c.strip_prefix(&base_canonical)
                .map_err(|_| StatusCode::FORBIDDEN)?;
            Ok(c)
        }
        Err(_) => Ok(final_path), // new file: parent is safe, leaf is clean
    }
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

fn extract_first_heading(html: &str) -> (String, String) {
    for level in 1..=6u8 {
        let open = format!("<h{level}");
        if let Some(start) = html.find(&open) {
            let close = format!("</h{level}>");
            if let Some(close_pos) = html[start..].find(&close) {
                let heading_end = start + close_pos + close.len();
                let heading = html[start..heading_end].to_string();
                let body = format!("{}{}", &html[..start], &html[heading_end..]);
                return (heading, body);
            }
        }
    }
    (String::new(), html.to_string())
}

pub fn get_shell() -> Cow<'static, str> {
    let base = crate::config::base_dir();
    let shell = if config::initcheck() {
        CFG.shell.shell.clone()
    } else {
        "default".to_string()
    };
    let path = base.join("src/shells").join(&shell).join("shell.html");

    if std::env::var("MAST_DEV").is_ok() {
        Cow::Owned(std::fs::read_to_string(&path).expect("Failed to load shell"))
    } else {
        Cow::Borrowed(
            SHELL.get_or_init(|| std::fs::read_to_string(&path).expect("Failed to load shell")),
        )
    }
}

pub fn inject(content: &str, toc: &str, ctx: &PageContext) -> String {
    let (clean, custom_toc) = extract_custom_toc(content);
    let final_toc = if custom_toc.is_empty() {
        toc
    } else {
        &custom_toc
    };
    let (heading, body) = extract_first_heading(&clean);
    let mut html = get_shell().into_owned();
    html = resolve_includes(&html);
    for (key, val) in template_vars(ctx) {
        html = html.replace(&format!("<!--MAST_{key}-->"), &val);
    }
    html.replace("<!--MAST-HEADING-->", &heading)
        .replace("<!--MAST-CONTENT-->", &body)
        .replace("<!--MAST-TOC-->", final_toc)
}

fn template_vars(ctx: &PageContext) -> Vec<(&'static str, String)> {
    if !config::initcheck() {
        return vec![
            ("WIKINAME", "Mast".to_string()),
            ("AUTH", auth_markup(None)),
        ];
    }
    vec![
        ("WIKINAME", CFG.basic.name.clone()),
        (
            "HOME",
            format!(
                "{}{}",
                CFG.basic.wikipage_directory_prefix, CFG.basic.default_wikipage
            ),
        ),
        ("MAST_FOOTER", "Powered by Mast".to_string()),
        ("AUTH", auth_markup(ctx.username.as_deref())),
    ]
}

fn auth_markup(username: Option<&str>) -> String {
    match username {
        Some(_) => r#"<li><a href="/account">Account</a></li>"#.to_string(),
        None => r#"<li><a href="/signin">Log in</a></li>"#.to_string(),
    }
}

fn resolve_includes(html: &str) -> String {
    let base = crate::config::base_dir();
    let shell = if config::initcheck() {
        CFG.shell.shell.clone()
    } else {
        "default".to_string()
    };
    let shell_dir = base.join("src/shells").join(&shell);

    let mut result = html.to_string();
    let marker_start = "<!--MAST_INCLUDE:";
    let marker_end = "-->";

    while let Some(start) = result.find(marker_start) {
        let content_start = start + marker_start.len();
        if let Some(end) = result[content_start..].find(marker_end) {
            let name = &result[content_start..content_start + end];
            // strip any path traversal chars, force .html
            let safe_name = name
                .replace('/', "")
                .replace('\\', "")
                .replace('.', "")
                .replace("..", "");
            let file_path = shell_dir.join(format!("components/{safe_name}.html"));
            let included = std::fs::read_to_string(&file_path)
                .unwrap_or_else(|e| format!("<!-- include error: {safe_name}: {e} -->"));
            let full_end = content_start + end + marker_end.len();
            result = format!("{}{}{}", &result[..start], included, &result[full_end..]);
        } else {
            break;
        }
    }
    result
}

pub async fn serve_static_page(
    session: Session,
    Path(slug): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let username: Option<String> = session.get("username").await.unwrap();
    let base = CFG.base_dir.join("src/shells").join(&CFG.shell.shell);
    let file_path = safe_path(&base, &format!("{slug}.html"))?;

    let content = std::fs::read_to_string(&file_path).unwrap_or_else(|_| {
        std::fs::read_to_string(base.join("404.html"))
            .unwrap_or_else(|_| "<h1>404</h1><p>Page not found</p>".to_string())
    });

    Ok(Html(inject(&content, "", &PageContext { username })))
}

fn extract_custom_toc(content: &str) -> (String, String) {
    let start = "<!--MAST-TOC-CONTENT-->";
    let end = "<!--/MAST-TOC-CONTENT-->";
    if let (Some(s), Some(e)) = (content.find(start), content.find(end)) {
        let toc = content[s + start.len()..e].trim().to_string();
        let clean = format!("{}{}", &content[..s], &content[e + end.len()..]);
        (clean, toc)
    } else {
        (content.to_string(), String::new())
    }
}

pub async fn serve_wiki_page(
    session: Session,
    Path(slug): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let requester = iac::requester_from_session(&session).await;
    let acl = iac::load_acl();
    if !iac::can_access(&acl, &requester, &slug, PV::R) {
        return Err(StatusCode::NOT_FOUND);
    }

    let base = CFG.base_dir.join(&CFG.storage.location);
    let file_path = safe_path(&base, &format!("{}.txt", slug))?;

    let content = std::fs::read_to_string(&file_path).unwrap_or_default();
    let page = converter::render_page(&content);
    let ctx = PageContext {
        username: requester.username.clone(),
    };
    Ok(Html(inject(&page.html, &page.toc, &ctx)))
}

pub async fn serve_raw_page(
    session: Session,
    Path(slug): Path<String>,
) -> Result<Json<RawPageResponse>, StatusCode> {
    let requester = iac::requester_from_session(&session).await;
    let acl = iac::load_acl();
    if !iac::can_access(&acl, &requester, &slug, PV::R) {
        return Ok(Json(RawPageResponse {
            content: String::new(),
            sha: String::new(),
            status: StatusCode::NOT_FOUND.as_u16(),
        }));
    }
    let base = CFG.base_dir.join(&CFG.storage.location);
    let file_path = safe_path(&base, &format!("{}.txt", slug))?;

    match std::fs::read_to_string(&file_path) {
        Ok(content) => {
            let (sha, _) = mtime_info(&file_path);
            Ok(Json(RawPageResponse {
                content,
                sha,
                status: StatusCode::OK.as_u16(),
            }))
        }
        Err(_) => Ok(Json(RawPageResponse {
            content: String::new(),
            sha: String::new(),
            status: StatusCode::NOT_FOUND.as_u16(),
        })),
    }
}

pub async fn serve_asset(Path(path): Path<String>) -> Result<(HeaderMap, Vec<u8>), StatusCode> {
    let base = config::base_dir();
    let canonical = safe_path(&base.join("dist/client"), &path)
        .or_else(|_| safe_path(&base.join("public"), &path))?;

    let mime = match path.rsplit('.').next() {
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("svg") => "image/svg+xml",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    };

    let bytes = std::fs::read(&canonical).map_err(|_| StatusCode::NOT_FOUND)?;
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_str(mime).unwrap());
    Ok((headers, bytes))
}
