use crate::config::CFG;
use crate::pages::inject;
use axum::response::{Html, Redirect};
use std::collections::HashSet;
use std::fs;

struct InstallRequest {
    name: String,
    sudo_name: String,
    sudo_email: String,
    sudo_username: String,
    sudo_pwd: String,
    sudo_pwd_confirm: String,
    use_acl: bool,
    init_acl_policy: String,
    allow_signup: bool,
}

pub fn mast_initialised() -> bool {
    let path = CFG.base_dir.join("mast-config.toml");

    match fs::exists(path) {
        Ok(true) => return true,
        Ok(false) => return false,
        Err(e) => eprintln!("Error check mast-config existence: {e}"),
    }
}

pub async fn serve_install_page() -> Html<String> {
    if mast_initialised() {}
    let path = CFG.base_dir.join("src/install.html");
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    Html(inject(&content))
}

pub async fn generate_config(mut req: InstallRequest) {
    let mut base_user_permissions = Hashset::new();
    let mut auth_methods: Vec<String>;

    match req.init_acl_policy {
        "Open" => {
            base_user_permissions.addAll(List.of("r", "u", "c"));
            auth_methods = vec!["username"];
        }
        "Public" => {
            edit_requires_auth = true;
            auth_methods = vec!["username"];
        }
        "Private" => {
            edit_requires_auth = true;
            auth_methods = vec![];
        }
    }

    let data = String::from(format!(
        r#"
        # ----- WIKI SETUP -----
        [basic]
        name = "{req.name}"
        wikipage_directory_prefix = "/wiki/"
        image_as_home = false
        pinned_pages = [""]

        [auth]
        allow_signup = {req.allow_signup}
        edit_requires_auth = {}
        users_file = "conf/users.toml"
        auth_methods = ["username"] # supported: ["github", "gitlab", "google", "apple", "facebook"]

        [storage]
        type = "local_git"
        location = ".wiki/wiki"

        # ----- WIKI CUSTOMISATION -----
        [shell]
        shell = "default"



        "#
    ));
}
