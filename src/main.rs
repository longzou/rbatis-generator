//! copyright © 了知信息科技 2021 - present
//! 应用
//! created by longzou 20220614
#[macro_use]
extern crate lazy_static;

extern crate rbatis;

mod schema;
mod utils;
mod config;
mod codegen;
mod tmpl;
mod permission;

use crate::codegen::{GenerateContext, CodeGenerator};
use crate::config::AppConfig;

//#[actix_web::main]
#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> std::io::Result<()> {
    // 加载配置文件
    std::env::set_var("RUST_LOG", "rbatis=info");
    let conf_path = std::env::current_dir().unwrap().as_os_str().to_str().unwrap().to_owned() + "/conf/rbatis.yml";
    println!("Current Path: {}", conf_path);

    match fast_log::init(fast_log::config::Config::new().console()) {
        Ok (_) => {}
        Err(err) => {
            log::info!("An error occurred on the Logger initializing. {}", err);
        }
    };

    // // 加载配置信息
    let mut webc = AppConfig::get().lock().unwrap();
    webc.load_yaml(&conf_path.clone()); 
    log::info!("MySQL: {}", webc.mysql_conf.url);
    let conf = webc.clone();
    drop(webc);

    let mut cgconf = conf.codegen_conf.clone();
    cgconf.database_url = conf.mysql_conf.url.clone();

    let ctx = GenerateContext::create(&cgconf.clone());

    let mut cg = CodeGenerator::new(&ctx);
    
    cg.load_tables().await;
    cg.generate();
    cg.write_out()?;

    cg.write_permission().await;

    std::thread::sleep_ms(500u32);
    Ok(())
}

