pub fn format_conf_tmpl(db: &String, wp: &String) -> String {
    format!("database:\n    url: {}\nwebserver:\n    port: {}\n", db, wp).to_string()
}

pub fn format_redis_conf_tmpl(
    host: &String,
    port: i64,
    username: &Option<String>,
    password: &Option<String>,
    db: i64,
) -> String {
    if username.is_some() {
        format!(
            "redis:\n    host: {}\n    port: {}\n    username: {}\n    password: {}\n    db: {}\n",
            host,
            port,
            username.clone().unwrap_or_default(),
            password.clone().unwrap_or_default(),
            db
        )
    } else {
        format!(
            "redis:\n    host: {}\n    port: {}\n    password: {}\n    db: {}\n",
            host,
            port,
            password.clone().unwrap_or_default(),
            db
        )
    }
}
