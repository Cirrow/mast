use crate::config::CFG;
use axum::Json;
use std::path::Path;

fn collect(dir: &Path, prefix: &str, routes: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    let p = if prefix.is_empty() {
                        name.to_string()
                    } else {
                        format!("{}/{}", prefix, name)
                    };
                    collect(&path, &p, routes);
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("txt") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let slug = if prefix.is_empty() {
                        stem.to_string()
                    } else {
                        format!("{}/{}", prefix, stem)
                    };
                    routes.push(slug);
                }
            }
        }
    }
}

pub async fn get_routes() -> Json<Vec<String>> {
    let mut routes: Vec<String> = Vec::new();
    let wiki_dir = CFG.base_dir.join(&CFG.storage.location);

    collect(&wiki_dir, "", &mut routes);
    routes.sort();

    Json(routes)
}
