use crate::config::{self, config_path};
use crate::pages::{PageContext, inject};

use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
};
use axum::response::{Html, IntoResponse, Redirect};
use serde::{Deserialize, Deserializer};
use std::fs;
use std::sync::atomic::Ordering;

#[derive(Deserialize)]
pub struct InstallRequest {
    pub site_name: String,
    pub sudo_username: String,
    pub sudo_email: String,
    pub sudo_pwd: String,
    pub sudo_pwd_confirm: String,
    pub acl_policy: String, // "open", "public", "private"
    #[serde(default, deserialize_with = "deserialize_one_or_many")]
    pub auth_methods: Vec<String>, //  # supported: ["username", "github", "gitlab", "google", "apple", "facebook"]. If empty, users cannot register.
    pub storage_type: String,
}

fn deserialize_one_or_many<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, SeqAccess, Visitor};
    use std::fmt;

    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "string or sequence")
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<Vec<String>, E> {
            Ok(vec![v.to_string()])
        }
        fn visit_seq<A: SeqAccess<'de>>(self, a: A) -> Result<Vec<String>, A::Error> {
            Vec::deserialize(de::value::SeqAccessDeserializer::new(a))
        }
    }
    deserializer.deserialize_any(V)
}

pub async fn serve_install_page() -> impl IntoResponse {
    if config::initcheck() {
        return Redirect::to("/").into_response();
    }
    let base_dir = config_path()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();
    let path = base_dir.join("src/install.html");
    let content = fs::read_to_string(&path).expect("install.html not found");
    Html(inject(&content, "", &PageContext::default())).into_response()
}

pub async fn handle_install(req: axum::Form<InstallRequest>) -> impl IntoResponse {
    if crate::config::initcheck() {
        return Redirect::to("/").into_response();
    }

    let base_dir = config_path()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();

    if req.site_name.is_empty() || req.sudo_username.is_empty() || req.storage_type.is_empty() {
        return Redirect::to("/install?error=missing_fields").into_response();
    }
    if req.sudo_pwd != req.sudo_pwd_confirm {
        return Redirect::to("/install?error=password_mismatch").into_response();
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(req.sudo_pwd.as_bytes(), &salt)
        .unwrap()
        .to_string();

    // IAC initialisation. Read the IAC documentation.
    // PVs are stored as raw numbers (N=0 R=1 E=2 C=4 U=16 D=255); the alpha
    // characters are only used for display.
    let (base_user_permission, auth_user_permission, iac_string) = match req.acl_policy.as_str() {
        "open" => ("4", "4", "[\"/\"]\n4 = [\"ALL\", \"ALL_AUTH\"]"),
        "public" => ("1", "4", "[\"/\"]\n1 = [\"ALL\"]\n4 = [\"ALL_AUTH\"]"),
        "private" => ("0", "1", "[\"/\"]\n0 = [\"ALL\"]\n1 = [\"ALL_AUTH\"]"),
        _ => ("0", "0", ""), // Should be unreachable with proper input
    };

    let auth_methods = req
        .auth_methods
        .iter()
        .map(|m| format!("\"{m}\""))
        .collect::<Vec<_>>()
        .join(", ");

    let storage_type = match req.storage_type.as_str() {
        "local" => "local_git",
        "remote_git" => "remote_git",
        _ => "local_git",
    };

    // Directory generation
    let conf_dir = base_dir.join("conf");
    fs::create_dir_all(&conf_dir).unwrap();

    let storage_dir = base_dir.join(".wiki/wiki");
    fs::create_dir_all(&storage_dir).unwrap();

    // Generate config
    let config_content = format!(
        r#"# ----- WIKI SETUP -----
[basic]
name = "{site_name}"

[auth]
auth_methods = [{auth_methods}]
base_user_permission = {base_user_permission}
auth_user_permission = {auth_user_permission}

[storage]
type = "{storage_type}"
location = ".wiki/wiki"

# ----- WIKI CUSTOMISATION -----
[shell]
shell = "default"
"#,
        site_name = req.site_name,
        auth_methods = auth_methods,
        base_user_permission = base_user_permission,
        auth_user_permission = auth_user_permission,
        storage_type = storage_type
    );
    fs::write(base_dir.join("mast-config.toml"), config_content).unwrap();

    // Write conf/users.toml
    let users_content = format!(
        r#"[sudo]
pwd_hash = "{hash}"
email = "{email}"
groups = ["sudo"]
"#,
        hash = hash,
        email = req.sudo_email,
    );
    fs::write(conf_dir.join("users.toml"), users_content).unwrap();

    // Write conf/acl.toml
    let acl_content = format!(
        "# Auto-generated IAC list
# Read the IAC documentation on the mast website to learn more.
# If this file has been manually created/edited, this comment should also be manually removed.
{iac_string}
",
        iac_string = iac_string,
    );
    fs::write(conf_dir.join("acl.toml"), acl_content).unwrap();

    crate::config::MAST_INIT.store(true, Ordering::Relaxed);
    Redirect::to("/").into_response()
}
