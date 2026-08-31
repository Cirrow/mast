use crate::config::{self, CFG};
use crate::iac::{self, PV};
use axum::{Form, Json, extract::Path, http::StatusCode, response::Html};
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
        Some(name) => format!(r#"<li><a href="/user/{name}">Account</a></li>"#),
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

pub async fn serve_admin_panel(session: Session) -> Result<Html<String>, StatusCode> {
    let requester = iac::requester_from_session(&session).await;
    if !iac::is_sudo(&requester) {
        return Err(StatusCode::NOT_FOUND);
    }
    let base = CFG.base_dir.join("src/shells").join(&CFG.shell.shell);
    let content =
        std::fs::read_to_string(base.join("admin.html")).map_err(|_| StatusCode::NOT_FOUND)?;
    let content = content.replace(
        "<!--MAST_ADMIN_STATUS-->",
        &status_markup("operational", 0, "Wiki is operational"),
    );
    let ctx = PageContext {
        username: requester.username.clone(),
    };
    Ok(Html(inject(&content, "", &ctx)))
}

pub async fn get_config(
    session: Session,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let requester = iac::requester_from_session(&session).await;
    let is_admin = iac::is_sudo(&requester);

    let requested: Vec<&str> = params
        .get("sections")
        .map(|s| s.split(',').collect())
        .unwrap_or_default();

    let config_json = serde_json::to_value(&*CFG).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let sections: Vec<serde_json::Value> = config::config_meta()
        .into_iter()
        .filter(|s| {
            if is_admin {
                return true;
            }
            requested.contains(&s.key)
        })
        .map(|section| {
            let fields: Vec<serde_json::Value> = section
                .fields
                .into_iter()
                .map(|field| {
                    let pointer = format!("/{}/{}", section.key, field.key);
                    let value = config_json
                        .pointer(&pointer)
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    serde_json::json!({
                        "key": field.key,
                        "label": field.label,
                        "description": field.description,
                        "value": value,
                    })
                })
                .collect();
            serde_json::json!({
                "key": section.key,
                "label": section.label,
                "fields": fields,
            })
        })
        .collect();

    Ok(Json(serde_json::json!(sections)))
}

pub async fn put_config(
    session: Session,
    Json(update): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let requester = iac::requester_from_session(&session).await;
    if !iac::is_sudo(&requester) {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut current = serde_json::to_value(&*CFG).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for key in ["basic", "auth", "storage", "shell"] {
        if let Some(val) = update.get(key) {
            if let Some(obj) = current.get_mut(key) {
                *obj = val.clone();
            }
        }
    }

    let new_config: config::Config =
        serde_json::from_value(current).map_err(|_| StatusCode::BAD_REQUEST)?;

    let errors = new_config.validate();
    if !errors.is_empty() {
        return Ok(Json(serde_json::json!({
            "success": false,
            "errors": errors,
        })));
    }

    let toml =
        toml::to_string_pretty(&new_config).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    std::fs::write(config::config_path(), toml).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Configuration saved. Restart the server to apply changes.",
    })))
}

fn substitute(
    template: &str,
    key: &str,
    label: &str,
    desc: &str,
    value: &serde_json::Value,
) -> String {
    let id = format!("cfg-{key}");
    let mut out = template
        .replace("{key}", key)
        .replace("{label}", label)
        .replace("{description}", desc)
        .replace("{id}", &id);
    match value {
        serde_json::Value::Bool(b) => {
            out = out.replace("{checked}", if *b { "checked" } else { "" })
        }
        serde_json::Value::Number(n) => out = out.replace("{value}", &n.to_string()),
        serde_json::Value::Array(arr) => {
            let csv = arr
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            out = out.replace("{value}", &csv);
        }
        _ => out = out.replace("{value}", value.as_str().unwrap_or("")),
    }
    out
}

fn extract_block(content: &str, name: &str) -> Option<String> {
    let start = format!("<!--MAST_FIELD:{name}-->");
    let end = format!("<!--/MAST_FIELD:{name}-->");
    let s = content.find(&start)?;
    let e = content[s + start.len()..].find(&end)?;
    Some(content[s + start.len()..s + start.len() + e].to_string())
}

fn render_field(
    fields_html: &str,
    field: &config::FieldMeta,
    value: &serde_json::Value,
) -> Option<String> {
    let name = match value {
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::Array(_) => "array",
        _ => "text",
    };
    let block = extract_block(fields_html, name)?;
    Some(substitute(
        &block,
        field.key,
        field.label,
        field.description,
        value,
    ))
}

fn render_config_sections(
    content: &str,
    shell_dir: &std::path::Path,
    cfg: &config::Config,
) -> Result<String, StatusCode> {
    let fields_html = std::fs::read_to_string(shell_dir.join("admin/configfields.html"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let config_json = serde_json::to_value(cfg).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut out = content.to_string();
    for section in config::config_meta() {
        let mut rendered = String::new();
        for field in &section.fields {
            let pointer = format!("/{}/{}", section.key, field.key);
            let value = config_json
                .pointer(&pointer)
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            rendered.push_str(&render_field(&fields_html, field, &value).unwrap_or_default());
        }
        out = out.replace(&format!("<!--MAST_FIELDS:{}-->", section.key), &rendered);
    }
    Ok(out)
}

pub async fn serve_config_manage(session: Session) -> Result<Html<String>, StatusCode> {
    let requester = iac::requester_from_session(&session).await;
    if !iac::is_sudo(&requester) {
        return Err(StatusCode::NOT_FOUND);
    }
    let base = CFG.base_dir.join("src/shells").join(&CFG.shell.shell);
    let content = std::fs::read_to_string(base.join("admin/configmanage.html"))
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let content = content.replace("<!--MAST_CONFIG_NOTICE-->", "");
    let rendered = render_config_sections(&content, &base, &CFG)?;
    let ctx = PageContext {
        username: requester.username.clone(),
    };
    Ok(Html(inject(&rendered, "", &ctx)))
}

pub async fn handle_config_manage(
    session: Session,
    Form(data): Form<std::collections::HashMap<String, String>>,
) -> Result<Html<String>, StatusCode> {
    let requester = iac::requester_from_session(&session).await;
    if !iac::is_sudo(&requester) {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut current = serde_json::to_value(&*CFG).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for (flat_key, raw_val) in &data {
        let parts: Vec<&str> = flat_key.splitn(2, '.').collect();
        if parts.len() != 2 {
            continue;
        }
        let (section, field) = (parts[0], parts[1]);

        if let Some(obj) = current.get_mut(section) {
            if let Some(field_val) = obj.get(field) {
                let new_val = match field_val {
                    serde_json::Value::Bool(_) => serde_json::Value::Bool(raw_val == "on"),
                    serde_json::Value::Number(n) => {
                        if n.is_f64() {
                            raw_val
                                .parse::<f64>()
                                .map(|v| serde_json::json!(v))
                                .unwrap_or_else(|_| field_val.clone())
                        } else {
                            raw_val
                                .parse::<i64>()
                                .map(|v| serde_json::json!(v))
                                .unwrap_or_else(|_| field_val.clone())
                        }
                    }
                    serde_json::Value::Array(_) => {
                        let arr: Vec<serde_json::Value> = raw_val
                            .split(',')
                            .map(|s| serde_json::json!(s.trim()))
                            .collect();
                        serde_json::Value::Array(arr)
                    }
                    _ => serde_json::json!(raw_val),
                };
                if let Some(v) = obj.get_mut(field) {
                    *v = new_val;
                }
            }
        }
    }

    let new_config: config::Config =
        serde_json::from_value(current).map_err(|_| StatusCode::BAD_REQUEST)?;

    let errors = new_config.validate();
    if !errors.is_empty() {
        let base = CFG.base_dir.join("src/shells").join(&CFG.shell.shell);
        let content = std::fs::read_to_string(base.join("admin/configmanage.html"))
            .map_err(|_| StatusCode::NOT_FOUND)?;

        let error_html = format!(
            r#"<div class="alert alert-error mb-4">{}</div>"#,
            errors
                .iter()
                .map(|e| format!("<p>{e}</p>"))
                .collect::<Vec<_>>()
                .join("")
        );
        let content = content.replace("<!--MAST_CONFIG_NOTICE-->", &error_html);
        let rendered = render_config_sections(&content, &base, &new_config)?;
        let ctx = PageContext {
            username: requester.username.clone(),
        };
        return Ok(Html(inject(&rendered, "", &ctx)));
    }

    let toml =
        toml::to_string_pretty(&new_config).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    std::fs::write(config::config_path(), toml).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let base = CFG.base_dir.join("src/shells").join(&CFG.shell.shell);
    let content = std::fs::read_to_string(base.join("admin/configmanage.html"))
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let success_html = r#"<div class="alert alert-success mb-4">Configuration saved. Restart the server to apply changes.</div>"#;
    let content = content.replace("<!--MAST_CONFIG_NOTICE-->", success_html);
    let rendered = render_config_sections(&content, &base, &new_config)?;
    let ctx = PageContext {
        username: requester.username.clone(),
    };
    Ok(Html(inject(&rendered, "", &ctx)))
}

fn status_markup(severity: &str, count: usize, message: &str) -> String {
    match severity {
        "update" => format!(
            r#"<div class="inline-grid *:[grid-area:1/1]">
               <div class="status status-info animate-ping"></div>
               <div class="status status-info"></div>
               </div> {message}"#
        ),
        "warning" => format!(
            r#"<div class="inline-grid *:[grid-area:1/1]">\
               <div class="status status-warning animate-ping"></div>\
               <div class="status status-warning"></div>\
               </div> {count} warning(s) — <a href="/admin/logs" class="link">open logs</a>"#
        ),
        "error" => format!(
            r#"<div class="inline-grid *:[grid-area:1/1]">\
               <div class="status status-error animate-ping"></div>\
               <div class="status status-error"></div>\
               </div> {count} error(s) — <a href="/admin/logs" class="link">open logs</a>"#
        ),
        _ => format!(
            r#"<div class="inline-grid *:[grid-area:1/1]">
               <div class="status status-success animate-ping"></div>
               <div class="status status-success"></div>
               </div> Wiki is operational"#
        ),
    }
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

pub async fn serve_user_page(
    session: Session,
    Path(requested): Path<String>,
) -> Result<Html<String>, StatusCode> {
    let users = crate::auth::load_users();
    let canonical = users
        .keys()
        .find(|k| k.eq_ignore_ascii_case(&requested))
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;

    let requester = iac::requester_from_session(&session).await;
    let acl = iac::load_acl();
    if !iac::can_access(&acl, &requester, &format!("/user/{canonical}"), PV::R) {
        return Err(StatusCode::NOT_FOUND);
    }

    let base = CFG.base_dir.join(&CFG.storage.location);
    let file_path = safe_path(&base, &format!("user/{canonical}.txt"))?;
    let content = std::fs::read_to_string(&file_path).unwrap_or_else(|_| {
        format!(
            "<h1>{}</h1>\n\n<p>This is {}'s page. Save the page <code>user/{}</code> to customise it.</p>",
            canonical, canonical, canonical
        )
    });
    let page = converter::render_page(&content);
    let ctx = PageContext {
        username: requester.username.clone(),
    };
    Ok(Html(inject(&page.html, &page.toc, &ctx)))
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
