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
#[serde(default)]
pub struct Basic {
    pub name: String,
    pub image_as_home: bool,
    pub image_path: Option<String>,
    pub pinned_pages: Option<Vec<String>>,
    pub wikipage_directory_prefix: String,
    pub default_wikipage: String,
}
// Default values for specific fields should they be missing. That said, there are certain fields that should not be empty.
// if there are missing values, and the field requires them, the validate() function should catch them. If not, may cause issues
impl Default for Basic {
    fn default() -> Self {
        Self {
            name: String::new(),
            image_as_home: false.into(),
            image_path: None,
            pinned_pages: None,
            wikipage_directory_prefix: "/wiki/".into(),
            default_wikipage: "home".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Auth {
    pub auth_methods: Option<Vec<String>>,
    pub users_file: String,
    pub username_signup_requires_email: bool,
    pub base_user_permission: Option<char>,
    pub auth_user_permission: Option<char>,
}
impl Default for Auth {
    fn default() -> Self {
        Self {
            auth_methods: None,
            users_file: "conf/users.toml".into(),
            username_signup_requires_email: true.into(),
            base_user_permission: None,
            auth_user_permission: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Storage {
    #[serde(rename = "type")]
    pub storage_type: Option<String>,
    pub location: Option<char>,
}
impl Default for Storage {
    fn default() -> Self {
        Self {
            storage_type: None,
            location: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Shell {
    pub shell: String,
}
impl Default for Shell {
    fn default() -> Self {
        Self {
            shell: "default".into(),
        }
    }
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

    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        errors.extend(self.validate_basic());

        errors
    }

    fn validate_basic(&self) -> Vec<String> {
        let mut e = Vec::new();

        for val in [&self.basic.name] {
            if val.is_empty() {
                e.push(format!("Please set {val}"));
            }
        }

        for (flag, val) in [(&self.basic.image_as_home, &self.basic.image_path)] {
            if *flag && val.is_none() {
                e.push(format!(
                    "When {flag} is set to true (or defaulted to true), {val} must be set."
                ))
            }
        }

        e
    }

    fn validate_auth(&self) {
        let mut e = Vec::new();

        let valid_pv = ['n', 'r', 'u', 'c', 'd'];

        for val in [
            &self.auth.base_user_permission,
            &self.auth.auth_user_permission,
        ] {
            if val.is_empty() {
                e.push(format!(
                    "IAC initialisation seemed to have gone wrong, no {val} is set."
                ));
            }
        }

        for (val, valid) in [
            (&self.auth.base_user_permission, &valid_pv[..]),
            (&self.auth.auth_user_permission, &valid_pv[..]),
        ] {
            if !valid.contains(&val.unwrap()) {
                e.push(format!("{val:?} is not a valid selection."));
            }
        }
    }
}
