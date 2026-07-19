use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};

pub static MAST_INIT: AtomicBool = AtomicBool::new(false);

/// Run once at server start. Checks if mast-config.toml exists.
pub fn init() {
    MAST_INIT.store(config_path().exists(), Ordering::Relaxed);
}

pub fn initcheck() -> bool {
    MAST_INIT.load(Ordering::Relaxed)
}

// Returns the path to mast-config.toml.
// Checks MAST_CONFIG env var first, falls back to ./mast-config.toml.
pub fn config_path() -> PathBuf {
    if let Ok(env) = env::var("MAST_CONFIG") {
        PathBuf::from(env)
    } else {
        PathBuf::from("./mast-config.toml")
    }
}

pub static CFG: LazyLock<Config> = LazyLock::new(|| Config::load());

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub basic: Basic,
    pub auth: Auth,
    pub storage: Storage,
    pub shell: Shell,
    #[serde(skip)]
    pub base_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Basic {
    pub name: String,
    pub image_as_home: bool,
    pub image_path: Option<String>,
    pub pinned_pages: Vec<String>,
    pub wikipage_directory_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auth {
    pub auth_methods: Vec<String>,
    pub users_file: String,
    #[serde(default)]
    pub username_signup_requires_email: bool,
    pub base_user_permission: char,
    pub auth_user_permission: char,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storage {
    #[serde(rename = "type")]
    pub storage_type: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shell {
    pub shell: String,
}

pub fn base_dir() -> PathBuf {
    config_path()
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf()
}

impl Config {
    pub fn load() -> Self {
        let config_path = config_path();
        let content = std::fs::read_to_string(&config_path).expect("mast-config.toml not found");
        let mut config: Config = toml::from_str(&content).expect("Failed to parse mast config");

        config.base_dir = base_dir();
        config
    }
}
