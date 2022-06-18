pub fn format_conf_tmpl(db: &String, wp: &String) -> String {
  format!("database:\n    url: {}\nwebserver:\n    port: {}\n", db, wp).to_string()
}