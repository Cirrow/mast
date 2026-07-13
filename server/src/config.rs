use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub basic: Basic,
    pub auth: Auth,
    pub storage: Storage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Basic {
    pub name: String,
    pub image_as_home: bool,
    pub image_path: Option<String>,
    pub pinned_pages: Vec<String>,
    pub dev_url: Option<String>,
    pub url_directory_prefix: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auth {
    pub allow_signup: bool,
    pub edit_requires_auth: bool,
    pub auth_methods: Vec<String>,
    pub users_file: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storage {
    #[serde(rename = "type")]
    pub storage_type: String,
    pub location: String,
}


impl Config {
    pub fn load() -> Self {

        let config_path = if PathBuf::from("./mast-config.toml").exists() {
            PathBuf::from("./mast-config.toml")
        } else if PathBuf::from("../mast-config.toml").exists() {
            PathBuf::from("../mast-config.toml")
        } else {
            panic!("mast-config.toml not found in ./ or ../ from the working directory");
        };
        let content: String = std::fs::read_to_string(&config_path)
            .expect("mast-config.toml not found");
        let mut config: Config = toml::from_str(&content)
            .expect("Failed to parse mast config");
        
        if config.basic.dev_url.is_none() {
            config.basic.dev_url = Some(
                std::env::var("MAST_URL").unwrap_or_else(|_| "http://localhost:4321".to_string())
            )
        }
        config
    }
}