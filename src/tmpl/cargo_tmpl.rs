use crate::config::CodeGenConfig;

const CARGO_TMPL:&str = r#"
[package]
authors = ["${authors}"]
edition = "${edition}"
name = "${app_name}"
version = "${app_version}"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
actix-rt = "2.7.0"
actix-utils = "3.0.0"
actix-web = "4.0.1"
awc = {version = "3.0.0", features = ["openssl"], optional = true}
base64 = "0.13.0"
bytes = "1.1.0"
env_logger = "0.9.0"
form_urlencoded = "1.0.1"
futures = "0.3.21"
http = "0.2.6"
json = "0.12.4"
lazy_static = "1.4.0"
md5 = "0.7.0"
rsa = "0.6.1"
percent-encoding = "2.1.0"
serde = "1.0.136"
serde_derive = "1.0.136"
serde_json = "1.0.79"
serde-xml-rs = "0.5.1"
url = "2.2.2"
yaml-rust = "0.4.5"
rbson = "2.0"
log = "0.4"
rand = "0.8.5"
fast_log = "1.3"
substring = "1.4.0"
change-case = "0.2.0"
chrono = "0.4.19"
async-std = "1.7.0"
rbatis = {version = "3.1.11", features = ["debug_mode"]}
tokio = {version = "1.10", features = ["full", "rt-multi-thread"] }
chimes-auth = {version = "0.1.0", features = ["session"]}
captcha = "0.0.9"
jsonwebtoken = "8.1.1"
"#;


pub fn replace_cargo_toml(ctx: &CodeGenConfig) -> String {
    CARGO_TMPL.replace("${authors}", ctx.app_authors.as_str())
              .replace("${edition}", ctx.app_edition.as_str())
              .replace("${app_name}", ctx.app_name.as_str())
              .replace("${app_version}", ctx.app_version.as_str())
}