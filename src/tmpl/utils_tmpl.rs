pub const UTILS_TMPL: &str = r#"
use std::fmt::{Debug};
use std::time::{SystemTime};
use std::fs::File;
use std::mem::MaybeUninit;
use std::sync::{Mutex, Once};
use rbatis::rbatis::{Rbatis};
use serde_derive::{Deserialize, Serialize};
use chrono::offset::Local;
use chrono::DateTime;
use yaml_rust::Yaml;

pub fn num_to_string (n:i64) -> String {
    let base_codec = ['A','B','C','D','E','F','G','H','J','K','L','M','N','O', 'P','Q','R','S','T','U','V','W','X','Y','Z','2','3','4','5','7','8','9'];
    let len = base_codec.len() as i64;
    let mut t = n;
    let mut result = "".to_string();
    while t > 0 {
        let idx = (t % len as i64) as usize;
        let ch = base_codec[idx];
        t = t / len;
        result.insert(0, ch);
    }
    result
}

pub fn f32_to_decimal(f: f32) -> Option<rbatis::Decimal> {
    match rbatis::Decimal::from_str(format!("{:.2}", f).as_str()) {
        Ok(r) => {
            Some(r)
        }
        Err(_) => {
            None
        }
    }
}

pub fn decimal_to_f32(dc: Option<rbatis::Decimal>) -> f32 {
    match dc {
        Some(r) => {
            match r.to_string().parse::<f32>() {
                Ok(t) => {
                    t
                }
                Err(_) => {
                    0f32
                }
            }
        }
        None => {
            0f32
        }
    }
}

pub fn make_decimal_negative(dc: Option<rbatis::Decimal>) -> Option<rbatis::Decimal> {
    match dc {
        Some(r) => {
            match r.to_string().parse::<f32>() {
                Ok(t) => {
                    f32_to_decimal(-t)
                }
                Err(_) => {
                    f32_to_decimal(0f32)
                }
            }
        }
        None => {
            f32_to_decimal(0f32)
        }
    }
}

pub fn generate_rand_string (len: usize) -> String {
    let mut retkey = "".to_string();

    while retkey.len() < len {
        let rng = rand::random::<u16>();
        let key = num_to_string(rng as i64);
        retkey = retkey + key.as_str();
    }

    retkey.chars().take(len).collect()
}

pub fn get_local_timestamp() -> u64 {
    let now = SystemTime::now();
    let date:DateTime<Local> = now.into();
    date.timestamp_millis() as u64
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct  ApiResult <T> {
    pub status: i32,
    pub message: String,
    pub data: Option<T>,
    pub timestamp: Option<u64>,
}

impl<T> ApiResult<T> {

    pub fn ok (dt: T) -> Self {
        ApiResult {
            status: 200,
            message: "OK".to_string(),
            data: Option::Some(dt),
            timestamp: Some(get_local_timestamp())
        }
    }

    pub fn error (code: i32, msg: &String) -> Self {
        ApiResult {
            status: code,
            message: msg.to_owned(),
            data: None,
            timestamp: Some(get_local_timestamp())
        }
    }

    pub fn new (code: i32, msg: &String, data: T, ts: u64) -> Self {
        ApiResult {
            status: code,
            message: msg.to_owned(),
            data: Some(data),
            timestamp: Some(ts)
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    pub db_conf: DatabaseConfig,
    pub webserver_conf: WebServerConfig,
}

#[derive(Debug, Clone, Default)]
pub struct WebServerConfig {
    pub port: i64,
}


#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
}


lazy_static!{
    pub static ref RB: Rbatis = {
      let rb = Rbatis::new();
      return rb;
    };
}


pub fn get_rbatis() -> &'static Rbatis {
    // 使用MaybeUninit延迟初始化
    static mut STATIC_RB: MaybeUninit<Rbatis> = MaybeUninit::uninit();
    // Once带锁保证只进行一次初始化
    static ONCE: Once = Once::new();

    ONCE.call_once(|| unsafe {
        // CONF = 1u64;
        let conf = AppConfig::get().lock().unwrap().to_owned();
        let url = conf.db_conf.url.clone();
        
        async_std::task::block_on(async {
            let rb = Rbatis::new();
            match rb.link(&url).await {
                Ok(_) => {
                    log::info!("Database was connected. Rbatis was initialized successfully.");
                }
                Err(err) => {
                    log::info!("Error: {}", err);
                }
            };
            STATIC_RB.as_mut_ptr().write(rb);
        });
    });
    unsafe { &*STATIC_RB.as_ptr() }
}


impl AppConfig {
    pub fn get() -> &'static Mutex<AppConfig> {
      // 使用MaybeUninit延迟初始化
      static mut CONF: MaybeUninit<Mutex<AppConfig>> = MaybeUninit::uninit();
      // Once带锁保证只进行一次初始化
      static ONCE: Once = Once::new();
  
      ONCE.call_once(|| unsafe {
          CONF.as_mut_ptr().write(Mutex::new(AppConfig {
            db_conf: DatabaseConfig { url: "".to_string() },
            webserver_conf: WebServerConfig { port: 10089i64 },
          }));
      });
      unsafe { &*CONF.as_ptr() }
    }
  
  
    pub fn load_yaml(&mut self, conf_path: &str) {
      use yaml_rust::yaml;
      // open file
      let mut f = match File::open(conf_path) {
          Ok(f) => f,
          Err(_) => {
              return
          }
      };
      let mut s = String::new();
      use std::io::Read;
      
      match f.read_to_string(&mut s) {
        Ok(_) => {}
        Err(_) => {}
      };
      // f.read_to_string(&mut s).unwrap(); // read file content to s
      // load string to yaml loader
      let docs = yaml::YamlLoader::load_from_str(&s).unwrap();
      // get first yaml hash doc
      // get server value
      // let server = yaml_doc["weapp"].clone();
      let doc = &docs[0];
      let db = &doc["database"];
      let web = &doc["webserver"];
      let dbconf = DatabaseConfig {
        url: if let Some(s) = db["url"].as_str() {
            s.to_owned()
        } else {
            "".to_owned()
        }
      };
      let webconf = WebServerConfig {
        port: if let Some(s) = web["port"].as_i64() {
            s.to_owned()
        } else {
            10089i64
        }
      };
      
      self.db_conf = dbconf;
      self.webserver_conf = webconf;
    }
  }

"#;
