//! copyright © 了知信息科技 2021 - present
//! 应用
//! created by longzou 20220614
#[macro_use]
extern crate lazy_static;
extern crate rbatis;

mod codegen;
mod config;
mod permission;
mod schema;
mod tmpl;
mod utils;

use std::time::Duration;

use crate::codegen::{CodeGenerator, GenerateContext};
use crate::config::AppConfig;

//#[actix_web::main]
#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> std::io::Result<()> {
    // 加载配置文件
    std::env::set_var("RUST_LOG", "rbatis=warn");
    let cfg = fast_log::config::Config::new()
        .console()
        .level(log::LevelFilter::Info);
    match fast_log::init(cfg) {
        Ok(_) => {}
        Err(err) => {
            log::info!("An error occurred on the Logger initializing. {}", err);
        }
    };

    let conf = std::env::args().nth(1);
    let conf_path = if conf.is_none() {
        std::env::current_dir()
            .unwrap()
            .as_os_str()
            .to_str()
            .unwrap()
            .to_owned()
            + "/conf/rbatis.yml"
    } else {
        let conf_base = conf.unwrap();
        if conf_base.starts_with("/") {
            conf_base
        } else {
            std::env::current_dir()
                .unwrap()
                .as_os_str()
                .to_str()
                .unwrap()
                .to_owned()
                + "/"
                + conf_base.as_str()
        }
    };

    log::info!("Parsing rust-generator config file: {}", conf_path);

    // // 加载配置信息
    let mut webc = AppConfig::get().lock().unwrap();
    webc.load_yaml(&conf_path.clone());
    log::info!("MySQL: {}", webc.mysql_conf.url);
    let conf = webc.clone();
    drop(webc);

    // let mut rcnn = AppConfig::redis();

    // match redis::cmd("GET").arg("h").query::<String>(&mut rcnn) {
    //     Ok(r) => {
    //         log::info!("Get the redis value: {}", r);
    //     }
    //     Err(err) => {
    //         log::info!("The error is {}", err.to_string());
    //     }
    // };

    let mut cgconf = conf.codegen_conf.clone();
    cgconf.database_url = conf.mysql_conf.url.clone();

    let ctx = GenerateContext::create(&cgconf.clone(), &conf.redis_conf);

    let mut cg = CodeGenerator::new(&ctx);

    cg.load_tables().await;
    cg.generate();
    cg.write_out()?;

    cg.write_permission().await;

    std::thread::sleep(Duration::from_secs(2));
    Ok(())
}
