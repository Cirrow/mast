use crate::config::CFG;
use crate::pages::inject;
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

pub async fn serve_install() -> Html<String> {
    let path = CFG.base_dir.join("src/install.html");
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    Html(inject(content))
}
