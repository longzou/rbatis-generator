const MAIN_TMPL:&str = r#"
#[macro_use]
extern crate actix_web;

// #[macro_use]
// extern crate wechat;
#[macro_use]
use rbatis::Value;
use rbatis::rbatis::Rbatis;

#[macro_use]
extern crate lazy_static;

// use awc::Client;

use actix_web::http::StatusCode;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Result};

${generated_mod_list}

mod utils;
use tokio::time;

use crate::utils::{AppConfig, WebServerConfig};

#[get("/")]
async fn index_handler(_req: HttpRequest) -> Result<HttpResponse> {
    // response
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html; charset=utf-8")
        .body("App is running."))
}


/// 应用启动入口
//#[actix_web::main]
#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> std::io::Result<()> {
    // 加载配置文件
    let conf_path = std::env::current_dir().unwrap().as_os_str().to_str().unwrap().to_owned() + "/conf/app.yml";
    log::info!("Current Path: {}", conf_path);
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    // env_logger::init();
    

    match fast_log::init(fast_log::config::Config::new().console()) {
        Ok (_) => {}
        Err(err) => {
            log::info!("An error occurred on the Logger initializing. {}", err);
        }
    };

    // // 加载配置信息
    let mut appconf = AppConfig::get().lock().unwrap();
    appconf.load_yaml(&conf_path.clone()); 
    let conf = appconf.clone();
    drop(appconf);

    let rb = utils::get_rbatis();

    // 启动web服务
    start_web_server(&conf.webserver_conf).await
}

/// web服务启动
async fn start_web_server(webconf: &WebServerConfig) -> std::io::Result<()> {
    // 设置服务器运行ip和端口信息
    let ip = format!("{}:{}", "0.0.0.0", webconf.port.clone());
    log::info!("App is listening on {}.", ip.clone());
    // 启动一个web服务
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            // .wrap(
            //     // 设置允许跨域请求
            //     actix_cors::Cors::default()
            //         .allow_any_origin()
            //         .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            //         .max_age(3600),
            // )
            // .app_data(Client::new())
            .service(index_handler)
${generated_service_list}
    })
    .bind(ip)?
    .run()
    .await
}

"#;


pub fn format_main_template(modlist: Vec<String>, servicelist: Vec<String>) -> String {
    let mut mod_text = String::new();
    let mut svc_text = String::new();
    for xl in modlist {
        mod_text.push_str(format!("mod {};\n", xl).as_str());
    }

    for xl in servicelist {
        svc_text.push_str(format!("            .service({})\n", xl).as_str());
    }

    let cp = MAIN_TMPL.clone();

    cp.replace("${generated_mod_list}", mod_text.as_str())
        .replace("${generated_service_list}", svc_text.as_str())
    
}