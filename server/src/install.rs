use crate::config::{self, config_path};
use crate::pages::inject;
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
    Html(inject(&content, "")).into_response()
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

    // IAC initialisation. Read the IAC documentation
    let (base_user_permission, auth_user_permission, iac_string) = match req.acl_policy.as_str() {
        "open" => ("c", "c", "[[/]]\npv=\"c\"\ntarget=\"ALL\""),
        "public" => (
            "r",
            "c",
            "[[/]]\npv=\"r\"\ntarget=\"ALL\"\n[[/]]\npv=\"c\"\ntarget=\"ALL_AUTH\"",
        ),
        "private" => (
            "n",
            "r",
            "[[/]]\npv=\"n\"\ntarget=\"ALL\"\n[[/]]\npv=\"r\"\ntarget=\"ALL_AUTH\"",
        ),
        _ => ("n", "n", ""), // Should be unreachable with proper input
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
wikipage_directory_prefix = "/wiki/"
default_wikipage = "home"
image_as_home = false
pinned_pages = [""]

[auth]
users_file = "conf/users.toml"
auth_methods = [{auth_methods}]
username_signup_requires_email = true
base_user_permission = "{base_user_permission}"
auth_user_permission = "{auth_user_permission}"

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
