use std::collections::HashMap;
use std::fs::File;
use std::mem::MaybeUninit;
use std::sync::{Mutex, Once};
use std::fmt::{Debug};
use serde_derive::{Deserialize, Serialize};
use rbatis::rbatis::{Rbatis};
use substring::Substring;
use change_case::snake_case;
use yaml_rust::Yaml;

#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    pub mysql_conf: MysqlConfig,
    pub codegen_conf: CodeGenConfig,
}


#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MysqlConfig {
    pub url: String,
    pub username: String,
    pub password: String
}


lazy_static!{
    pub static ref RUST_KEY_RENAME_MAP: HashMap<String, String> = {
      let mut hm = HashMap::new();
      hm.insert("type".to_string(), "r#type".to_string());
      hm.insert("struct".to_string(), "r#struct".to_string());
      hm.insert("pub".to_string(), "r#pub".to_string());
      hm.insert("static".to_string(), "r#static".to_string());
      hm.insert("else".to_string(), "r#else".to_string());
      hm.insert("while".to_string(), "r#while".to_string());
      hm.insert("async".to_string(), "r#async".to_string());
      hm.insert("const".to_string(), "r#const".to_string());
      hm.insert("use".to_string(), "r#use".to_string());
      hm.insert("mod".to_string(), "r#mod".to_string());
      hm.insert("main".to_string(), "r#main".to_string());
      hm.insert("match".to_string(), "r#match".to_string());
      hm.insert("let".to_string(), "r#let".to_string());
      hm.insert("mut".to_string(), "r#mut".to_string());
      hm.insert("crate".to_string(), "r#crate".to_string());
      hm.insert("if".to_string(), "r#if".to_string());
      hm.insert("return".to_string(), "r#return".to_string());
      hm.insert("self".to_string(), "r#self".to_string());
      
      return hm;
    };
}


pub fn safe_struct_field_name (oldname: &String) -> String {
    if (RUST_KEY_RENAME_MAP.contains_key(&oldname.to_lowercase())) {
        match RUST_KEY_RENAME_MAP.get(&oldname.to_lowercase()) {
            Some(tn) => {
                tn.to_owned()
            }
            None => {
                oldname.to_owned()
            }
        }
    } else {
        oldname.to_owned()
    }
}

lazy_static!{
    pub static ref RB: Rbatis = {
      let rb = Rbatis::new();
      // log!("Connect to database {} ", conf.mysql_conf.url.clone());
      // tokio::runtime::Handle::current().block_on(async {
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
        let url = conf.mysql_conf.url.clone();
        
        async_std::task::block_on(async {
            let rb = Rbatis::new();
            log::info!("Make the database connection {}", url.clone());
            match rb.link(&url).await {
                Ok(_) => {
                    log::info!("Connected.");
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
          mysql_conf: MysqlConfig { url: "".to_string(), username: "".to_string(), password: "".to_string() },
            codegen_conf: CodeGenConfig::default(),
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
    let mysql = &doc["database"];
    let myconf = MysqlConfig {
      url: if let Some(s) = mysql["url"].as_str() {
          s.to_owned()
      } else {
          "".to_owned()
      },
      username: if let Some(s) = mysql["username"].as_str() {
          s.to_owned()
      } else {
          "".to_owned()
      },
      password: if let Some(s) = mysql["password"].as_str() {
          s.to_owned()
      } else {
          "".to_owned()
      }
    };
    
    self.mysql_conf = myconf;
    self.codegen_conf = CodeGenConfig::load_from_yaml(&doc["codegen"]);
  }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TableConfig {
    pub name: String,
    pub struct_name: String,
    pub primary_key: String,
    pub api_handler_name: String,
    pub all_field_option: bool,
    pub update_skip_fields: Option<String>,
    pub update_seletive: bool,
    pub page_query: bool,
    pub logic_deletion: bool,
    pub generate_handler: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct QueryParam {
    pub column_names: Option<String>,
    pub column_types: Option<String>,
    pub column_express: Option<String>,
    pub variant: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct QueryConfig {
    pub base_sql: String,
    pub single_result: bool,
    pub struct_name: String,
    pub api_handler_name: String,
    pub generate_handler: bool,
    pub params: Vec<QueryParam>,
    pub variant_params: Vec<QueryParam>,
}

impl QueryConfig {
    
    fn load_from_yaml_node(node: &Yaml) -> Vec<Self> {
        let mut list = vec![];
        match node.as_vec() {
            Some(ns) => {
                for nd in ns.clone() {
                    let mut st = Self::default();
                    st.load_by_node(&nd);
                    list.push(st);
                }
            }
            None => {
            }
        };
        list
    }

    fn load_by_node(&mut self, node: &Yaml) {
        self.struct_name = node["struct-name"].as_str().unwrap_or_default().to_string();
        self.api_handler_name = match node["api-handler-name"].as_str() {
            Some (tstr) => tstr.to_string(),
            None => {
                let stname = snake_case(self.struct_name.clone().as_str());
                match stname.find("_") {
                    Some(us) => {
                        stname.substring(us + 1, stname.len()).to_string()
                    }
                    None => {
                        stname
                    }
                }
            }
        };
        self.base_sql = node["base-sql"].as_str().unwrap_or_default().to_string();
        self.single_result = match node["single-result"].as_bool() {
            Some(tt) => tt,
            None => false
        };
        self.generate_handler = node["generate-handler"].as_bool().unwrap_or_default();

        let mut params = Vec::new();
        let mut vtparams = Vec::new();
        match node["params"].as_vec() {
            Some(vp) => {
                for mcn in vp.clone() {
                    let pm = QueryParam {
                        column_names: match mcn["column-names"].as_str() {
                            Some(s) => {
                                Some(s.to_string())
                            }
                            None => {
                                None
                            }
                        },
                        column_types: match mcn["column-types"].as_str() {
                            Some(s) => {
                                Some(s.to_string())
                            }
                            None => {
                                None
                            }
                        },
                        column_express: match mcn["column-express"].as_str() {
                            Some(s) => {
                                Some(s.to_string())
                            }
                            None => {
                                None
                            }
                        },
                        default_value: match mcn["default-value"].as_str() {
                            Some(s) => {
                                Some(s.to_string())
                            }
                            None => {
                                None
                            }
                        },
                        variant: false
                    };
                    params.push(pm);
                }
            }
            None => {
                
            }
        };
        match node["variant-params"].as_vec() {
            Some(vp) => {
                for mcn in vp.clone() {
                    let pm = QueryParam {
                        column_names: match mcn["column-names"].as_str() {
                            Some(s) => {
                                Some(s.to_string())
                            }
                            None => {
                                None
                            }
                        },
                        column_types: match mcn["column-types"].as_str() {
                            Some(s) => {
                                Some(s.to_string())
                            }
                            None => {
                                None
                            }
                        },
                        column_express: match mcn["column-express"].as_str() {
                            Some(s) => {
                                Some(s.to_string())
                            }
                            None => {
                                None
                            }
                        },
                        default_value: match mcn["default-value"].as_str() {
                            Some(s) => {
                                Some(s.to_string())
                            }
                            None => {
                                None
                            }
                        },
                        variant: true
                    };
                    vtparams.push(pm);
                }
            }
            None => {
                
            }
        };
        self.params = params;
        self.variant_params = vtparams;
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CodeGenConfig {
    pub app_authors: String,
    pub app_edition: String,
    pub app_name: String,
    pub app_version: String,
    pub output_path: String,
    pub entity_in_one_file: bool,
    pub generate_for_lib: bool,
    pub always_generate_handler: bool,
    pub always_generate_entity: bool,
    pub api_handler_prefix: String,
    pub schema_name: String,
    pub webserver_port: String,
    pub database_url: String,
    pub tables: Vec<TableConfig>,
    pub queries: Vec<QueryConfig>,
}


impl CodeGenConfig {

    pub fn load_from_yaml(node: &Yaml) -> Self {
        let mut tables = Vec::new();

        let gh = if let Some(s) = node["always-generate-handler"].as_bool() {
            s.to_owned()
        } else {
            false
        };

        match node["tables"].as_vec() {
            Some(t) => {
                for tbn in t {
                    tables.push(TableConfig {
                        name: tbn["name"].as_str().unwrap_or_default().to_string(),
                        struct_name: tbn["struct-name"].as_str().unwrap_or_default().to_string(),
                        primary_key: tbn["primary-key"].as_str().unwrap_or_default().to_string(),
                        api_handler_name: match tbn["api-handler-name"].as_str() {
                            Some (tstr) => tstr.to_string(),
                            None => {
                                match tbn["struct-name"].as_str() {
                                    Some(sstr) => {
                                        let stname = snake_case(sstr.to_string().as_str());
                                        match stname.find("_") {
                                            Some(us) => {
                                                stname.substring(us + 1, stname.len()).to_string()
                                            }
                                            None => {
                                                stname
                                            }
                                        }
                                    }
                                    None =>  {
                                        let tblname = tbn["name"].as_str().unwrap().to_string().to_lowercase();
                                        match tblname.find("_") {
                                            Some(us) => {
                                                tblname.substring(us + 1, tblname.len()).to_string()
                                            }
                                            None => {
                                                tblname
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        update_skip_fields: match tbn["update-skip-fields"].as_str() {
                            Some(tstr) => {
                                Some(tstr.to_string())
                            }
                            None => {
                                None
                            }
                        },
                        all_field_option: match tbn["all-field-option"].as_bool() {
                            Some(ff) => {
                                ff
                            }
                            None => {
                                true
                            }
                        },
                        update_seletive: tbn["update-seletive"].as_bool().unwrap_or_default(),
                        page_query: tbn["page-query"].as_bool().unwrap_or_default(),
                        logic_deletion: tbn["logic-deletion"].as_bool().unwrap_or_default(),
                        generate_handler: match tbn["generate-handler"].as_bool() {
                            Some(ff) => {
                                ff
                            }
                            None => {
                                gh
                            }
                        },
                    });
                }
            }
            None => {

            }
        };

        let queries = QueryConfig::load_from_yaml_node(&node["queries"]);

        Self {
            app_authors: if let Some(s) = node["app-authors"].as_str() {
                s.to_owned()
            } else {
                "None".to_string()
            },
            app_edition: if let Some(s) = node["app-edition"].as_str() {
                s.to_owned()
            } else {
                "2021".to_string()
            },
            app_name: if let Some(s) = node["app-name"].as_str() {
                s.to_owned()
            } else {
                "codegen_test".to_string()
            },
            app_version: if let Some(s) = node["app-version"].as_str() {
                s.to_owned()
            } else {
                "0.0.1".to_string()
            },
            output_path: if let Some(s) = node["output-path"].as_str() {
                s.to_owned()
            } else {
                "./".to_string()
            },
            schema_name: if let Some(s) = node["schema-name"].as_str() {
                s.to_owned()
            } else {
                "".to_string()
            },
            webserver_port: if let Some(s) = node["webserver-port"].as_str() {
                s.to_owned()
            } else {
                "".to_string()
            },
            api_handler_prefix: if let Some(s) = node["api-handler-prefix"].as_str() {
                s.to_owned()
            } else {
                "".to_string()
            },
            database_url: "".to_string(),
            entity_in_one_file: if let Some(s) = node["entity-in-one-file"].as_bool() {
                s.to_owned()
            } else {
                false
            },
            generate_for_lib: if let Some(s) = node["generate-for-lib"].as_bool() {
                s.to_owned()
            } else {
                false
            },
            always_generate_handler: gh,
            always_generate_entity:  if let Some(s) = node["always-generate-entity"].as_bool() {
                s.to_owned()
            } else {
                true
            },
            tables: tables,
            queries: queries,
        }
    }
}
