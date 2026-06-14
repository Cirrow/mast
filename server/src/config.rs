use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub basic: Basic,
    pub auth: Auth,
    pub storage: Storage,
    #[serde(skip)]
    pub content_dir: PathBuf
}

#[derive(Debug, Deserialize)]
pub struct Basic {
    pub name: String,
    pub image_as_home: bool,
    pub image_path: Option<String>,
    pub pinned_pages: Vec<String>,
    pub dev_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Auth {
    pub allow_signup: bool,
}
#[derive(Debug, Deserialize)]
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
        
        config.content_dir = std::env::var("CONTENT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("../.wiki/wiki"));

        if config.basic.dev_url.is_none() {
            config.basic.dev_url = Some(
                std::env::var("MAST_URL").unwrap_or_else(|_| "http://localhost:4321".to_string())
            )
        }
        config
    }
}