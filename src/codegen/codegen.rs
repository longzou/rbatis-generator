use std::collections::HashMap;
use std::fmt::{Debug};
use std::io::{self, Write};
use std::path::Path;
use std::path::PathBuf;
use std::fs::{create_dir, create_dir_all, OpenOptions};
use change_case::{pascal_case, snake_case};
use chrono::format::format;
use rbatis::rbatis::Rbatis;
use serde_derive::{Deserialize, Serialize};
use crate::codegen::parse_table_as_struct;
use crate::tmpl::format_conf_tmpl;
use crate::config::{CodeGenConfig, TableConfig, get_rbatis, safe_struct_field_name};
use crate::schema::{TableInfo, ColumnInfo};
use substring::Substring;

use super::{generate_actix_handler_for_table, execute_sql, parse_query_as_struct, parse_query_as_file, parse_query_handler_as_file};

pub trait CodeWriter {
    fn write(&self, ro: &mut RustOutput);
}

/**
 * 代码生成的上下文
 * 它的主要功能是解释配置文件，并根据配置文件来准备代码生成所需要对应的一些参数
 * 这些参数如：生成后所存放的路径等
 */
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct GenerateContext {
    pub codegen_conf: CodeGenConfig,
    pub tables: Vec<TableInfo>,
    pub columns: HashMap<String, Vec<ColumnInfo>>,
    pub structs: Vec<RustStruct>,
    pub queries: Vec<RustStruct>,
}

impl GenerateContext {

    pub fn create(cgconf: &CodeGenConfig) -> Self { 
        Self { 
            codegen_conf: cgconf.clone(), 
            tables: vec![], 
            columns: HashMap::new(), 
            structs: vec![],
            queries: vec![],
        }
    }

    pub fn get_root_path(&self) -> &str {
        self.codegen_conf.output_path.as_str()
    }

    pub fn get_entity_path(&self) -> String {
        self.get_root_path().to_owned() + "/entity"
    }

    pub fn get_controller_path(&self) -> String {
        self.get_root_path().to_owned() + "/controller"
    }

    pub fn get_facade_path(&self) -> String {
        self.get_root_path().to_owned() + "/facade"
    }

    pub fn is_all_entity_in_one_file(&self) -> bool {
        self.codegen_conf.entity_in_one_file
    }

    pub fn is_generate_lib(&self) -> bool {
        self.codegen_conf.generate_for_lib
    }

    pub fn add_struct(&mut self, st: &RustStruct) {
        self.structs.push(st.clone());
    }

    pub fn add_query(&mut self, st: &RustStruct) {
        self.queries.push(st.clone());
    }    

    pub fn add_table(&mut self, tb: &TableInfo, cols: &Vec<ColumnInfo>) {
        if tb.table_name.is_some() {
            log::info!("Add the table {} into the context.", tb.table_name.clone().unwrap_or_default());
            self.tables.push(tb.clone());
            self.columns.insert(tb.table_name.clone().unwrap(), cols.clone());
        }
    }

    pub fn table_for_each<F>(&mut self, func: &mut F)
    where
        Self: Sized,
        F: FnMut((TableInfo, Vec<ColumnInfo>)),
    {
        self.tables.clone().into_iter().for_each(|f| {
            let cols = self.columns.get(&f.table_name.clone().unwrap_or_default());
            
            match cols {
                Some(cs) => {
                    func((f, cs.to_vec()));
                }
                None => {}
            }
        });
    }

    pub fn get_struct_name(&self, tbl: &String) -> Option<String> {
        for tc in self.codegen_conf.tables.clone() {
            if tc.name == tbl.clone() {
                if tc.struct_name.is_empty() {
                    return Some(pascal_case(tc.name.clone().as_str()));
                } else {
                    return Some(tc.struct_name.clone());
                }
            }
        }
        None
    }

    pub fn get_table_conf(&self, tbl: &String) -> Option<TableConfig> {
        for tc in self.codegen_conf.tables.clone() {
            if tc.name == tbl.clone() {
                return Some(tc.clone());
            }
        }
        None
    }

    pub fn get_table_info(&self, tbl: &String) -> Option<TableInfo> {
        for tc in self.tables.clone() {
            if tc.table_name.clone().unwrap_or_default() == tbl.clone() {
                return Some(tc.clone());
            }
        }
        None
    }

    pub fn get_table_columns(&self, tbl: &String) -> Vec<ColumnInfo> {
        match self.columns.get(&tbl.clone()) {
            Some(st) => {
                st.to_owned()
            }
            None => {
                vec![]
            }
        }
    }

    /**
     * 从TableConfig中的主键来解决
     */
    pub fn get_table_column_by_primary_key(&self, tbl: &String) -> Vec<ColumnInfo> {
        let mut pkeys = vec!();
        let cols = self.columns.get(&tbl.clone());
        let tbconf = self.get_table_conf(tbl);
        if tbconf.is_some() {
            let tbc = tbconf.unwrap();
            match cols {
                Some(tcls) => {
                    tbc.primary_key.split(&",".to_string()).for_each(|f| {
                        for cl in tcls {
                            if cl.column_name.clone().unwrap_or_default() == f.to_string() {
                                pkeys.push(cl.clone());
                                break;
                            }
                        }
                    });
                }
                None => { }
            };
        }
        
        pkeys
    }

    pub fn get_table_pkey_column(&self, tbl: &String) -> Vec<ColumnInfo> {
        let mut pkeys = vec!();
        let cols = self.get_table_columns(tbl);
        for cl in cols {
            if cl.column_key.clone().unwrap_or_default().to_lowercase() == "pri" {
                pkeys.push(cl.clone());
                break;
            }
        }
        
        pkeys
    }

    pub fn get_table_auto_incremnt_column(&self, tbl: &String) -> Option<ColumnInfo> {
        let cols = self.get_table_columns(tbl);
        for cl in cols {
            if cl.extra.clone().unwrap_or_default().to_lowercase() == "auto_increment" {
                return Some(cl.clone());
            }
        }
        
        None
    }    



}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RustFunc {
    pub is_struct_fn: bool,
    pub is_self_fn: bool,
    pub is_self_mut: bool,
    pub is_pub: bool,
    pub is_async: bool,
    pub func_name: String,
    pub return_is_option: bool,
    pub return_is_result: bool,
    pub return_type: Option<String>,
    pub params: Vec<(String, String)>,
    pub bodylines: Vec<String>,
    pub macros: Vec<String>,
}

impl RustFunc {

    pub fn add_params(&mut self, name: &String, rtype: &String) {
        self.params.push((name.clone(), rtype.clone()));
    }


    pub fn add_bodyline(&mut self, line: &String ){
        self.bodylines.push(line.clone());
    }

    pub fn add_bodylines(&mut self, lines: &mut Vec<String>) {
        self.bodylines.append(lines);
    }
}

impl CodeWriter for RustFunc {
    fn write(&self, ro: &mut RustOutput) {
        let fnname = if self.is_pub {
            if self.is_async {
                format!("pub async fn {}(", self.func_name)
            } else {
                format!("pub fn {}(", self.func_name)
            }
        } else {
            if self.is_async {
                format!("async fn {}(", self.func_name)
            } else {
                format!("fn {}(", self.func_name)
            }
        };

        let mut first = format!("{}", fnname);

        if self.is_struct_fn && self.is_self_fn { // Should be in an struct, the self fn will valid
            if self.is_self_mut {
                first.push_str("&mut self,");
            } else {
                first.push_str("&self,");
            }
        }

        for pm in self.params.clone() {
            first.push_str(&format!("{}: {},", pm.0.to_string(), pm.1.to_string()));
        }

        if first.ends_with(",") {
            // do sub string process
            first = first.substring(0, first.len() - 1).to_string();
        }
        first.push(')');
        if self.return_is_result {
            if self.return_is_option {
                first.push_str(&format!(" -> Result<Option<{}>, Error> {{", self.return_type.clone().unwrap_or_default()));
            } else {
                first.push_str(&format!(" -> Result<{}, Error> {{", self.return_type.clone().unwrap_or_default()));
            }
        } else {
            if self.return_is_option {
                first.push_str(&format!(" -> Option<{}> {{", self.return_type.clone().unwrap_or_default()));
            } else {
                first.push_str(&format!(" -> {} {{", self.return_type.clone().unwrap_or_default()));
            }
        }
        let mut space = 0i32;
        if self.is_struct_fn {
            for mc in self.macros.clone() {
                ro.write_line(&format!("    {}", mc));
            }
            ro.write_line(&format!("    {}", first));
            space = 2;
        } else {
            for mc in self.macros.clone() {
                ro.write_line(&format!("{}", mc));
            }
            ro.write_line(&format!("{}", first));
            space = 1;
        }

        for ln in self.bodylines.clone() {
            if ln.trim().starts_with("}") {
                space -= 1;
            }
            let mut blankspace = String::new();
            let mut t  = 0;
            while t < space {
                blankspace.push_str("    ");
                t += 1;
            }
            ro.write_line(&format!("{}{}", blankspace, ln));
            if ln.trim().ends_with("{") {
                space += 1;
            }
        }

        if self.is_struct_fn {
            ro.write_line("    }");
        } else {
            ro.write_line("}");
        }
        ro.write_line("");
        ro.write_line("");
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RustStructField {
    pub is_pub: bool,
    pub column_name: String,
    pub field_name: String,
    pub field_type: String,
    pub is_option: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RustStruct {
    pub is_pub: bool,
    pub struct_name: String,
    pub annotations: Vec<String>,
    pub fields: Vec<RustStructField>,
    pub funclist: Vec<RustFunc>,
}

impl RustStruct {
    pub fn add_field(&mut self, fd: &RustStructField) {
        self.fields.push(fd.clone())
    }

    pub fn add_func(&mut self, fd: &RustFunc) {
        self.funclist.push(fd.clone())
    }

    pub fn add_annotation(&mut self, fd: &String) {
        if !self.annotations.contains(fd) {
            self.annotations.push(fd.clone())
        }
    }

}

impl CodeWriter for RustStruct {

    fn write(&self, ro: &mut RustOutput) {
        for ln in self.annotations.clone() {
            ro.write_line(&ln);
        }
        if self.is_pub {
            ro.write_line(&format!("pub struct {} {{", self.struct_name.clone())); 
        } else {
            ro.write_line(&format!("struct {} {{", self.struct_name.clone()));
        }

        for fd in self.fields.clone() {
            let ret = if fd.is_option {
                format!("Option<{}>", fd.field_type.clone())
            } else {
                format!("{}", fd.field_type.clone())
            };

            if fd.column_name.len() > 0 && fd.column_name != fd.field_name {
                ro.write_line(&format!("    #[serde(rename(deserialize=\"{}\"))]", fd.column_name.clone()));
            }
            
            if fd.is_pub {
                ro.write_line(&format!("    pub {}: {},", fd.field_name.clone(), ret));
            } else {
                ro.write_line(&format!("    {}: {},", fd.field_name.clone(), ret));
            }
        }
        ro.write_line("}");
        ro.write_line("");
        ro.write_line("");
        if !self.funclist.is_empty() {
            ro.write_line(&format!("impl {} {{", self.struct_name.clone()));

            for func in self.funclist.clone() {
                func.write(ro);
            }

            ro.write_line("}");
            ro.write_line("");
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RustOutput {
    outputs: Vec<String>,
}

impl RustOutput {
    
    pub fn write_line(&mut self, line: &str) {
        let newline = line.to_string() + "\n";
        self.outputs.push(newline);
    }

    pub fn print_out(&self) {
        for ln in self.outputs.clone() {
            println!("{}", ln);
        }
    }
}

/**
 * Rust程序文件结构
 */
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RustFileImpl {
    pub file_name: String,
    pub mod_name: String, // Save into a folder
    pub caretlist: Vec<String>,
    pub usinglist: Vec<String>,
    pub structlist: Vec<RustStruct>,
    pub funclist: Vec<RustFunc>,
}


impl RustFileImpl {
    
    pub fn add_using(&mut self, us: &String) {
        self.usinglist.push(us.clone());
    }

    pub fn add_caret(&mut self, us: &String) {
        self.caretlist.push(us.clone());
    }

    pub fn add_struct(&mut self, us: &RustStruct) {
        self.structlist.push(us.clone());
    }

    pub fn add_func(&mut self, us: &RustFunc) {
        self.funclist.push(us.clone());
    }

    pub fn write_out(&self, filename: &String) -> std::io::Result<()> {
        let mut ro = RustOutput::default();
        ro.write_line("/**");
        ro.write_line(format!(" * Generate the file for {}, ", self.file_name.clone()).as_str());
        ro.write_line(" */");
        for crt in self.caretlist.clone() {
            ro.write_line(format!("extern caret {};", crt).as_str());
        }
        ro.write_line("");
        for usingline in self.usinglist.clone() {
            ro.write_line(format!("use {};", usingline).as_str());
        }
        ro.write_line("");

        for st in self.structlist.clone() {
            st.write(&mut ro);
        }
        for func in self.funclist.clone() {
            func.write(&mut ro);
        }

        let mut file = OpenOptions::new()
                                .write(true)
                                .append(false)
                                .create(true)
                                .truncate(true)
                                .open(filename)?;
        for lt in ro.outputs.clone() {
            file.write_all(lt.as_bytes())?;
        }

        file.flush()?;

        Ok(())
    }
}


#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CodeGenerator {
    pub ctx: GenerateContext,
    pub files: Vec<RustFileImpl>,
    pub default_entity_using: Vec<String>,
    pub default_handler_using: Vec<String>,
}

impl CodeGenerator {
    
    /**
     * Step 1
     * 创建代码生成器实例
     */
    pub fn new(ctx: &GenerateContext) -> Self {
        Self {
            ctx: ctx.clone(),
            files: vec![],
            default_entity_using: Self::get_default_entity_using(),
            default_handler_using: Self::get_default_handler_using(),
        }
    }

    pub fn get_default_entity_using() -> Vec<String> {
        let mut list = vec![];
        list.push("std::fmt::{Debug}".to_string());
        list.push("rbatis::crud::{CRUD, Skip}".to_string());
        list.push("serde_derive::{Deserialize, Serialize}".to_string());
        list.push("rbatis::{sql, crud_table}".to_string());
        list.push("rbatis::rbatis::{Rbatis}".to_string());
        list.push("rbatis::executor::{ Executor, ExecutorMut }".to_string());
        list.push("rbatis::error::Error".to_string());
        list.push("rbatis::DateTimeNative".to_string());
        list.push("rbatis::Page".to_string());
        list.push("rbatis::PageRequest".to_string());
        list.push("rbson::Bson".to_string());
        list
    }

    pub fn get_default_handler_using() -> Vec<String> {
        let mut list = vec![];

        list.push("crate::utils::{ApiResult, get_rbatis}".to_string());
        list.push("actix_web::{web, HttpRequest, HttpResponse, Result}".to_string());
        list.push("rbatis::Page".to_string());

        list
    }

    /**
     * Step 2
     * 加载数据库表
     * 根据从Yaml文件中加载的配置来进行处理
     */
    pub async fn load_tables(&mut self) {
        let rb = get_rbatis();
        let ts = self.ctx.codegen_conf.schema_name.clone();
        let tables = self.ctx.codegen_conf.tables.clone();
        for f in tables {
            let tn = f.name.clone();
            match TableInfo::load_table(rb, &ts.clone(), &tn.clone()).await {
                Ok(tbop) => {
                    match tbop {
                        Some(tb) => {
                            log::info!("Columns of table {} {} {} will be fetching.", tb.table_name.clone().unwrap_or_default(), tb.table_schema.clone().unwrap_or_default(), tb.table_catalog.clone().unwrap_or_default());
                            match ColumnInfo::load_columns(rb, &ts.clone(), &tn.clone()).await {
                                Ok(cols) => {
                                    log::info!("The table {} will be added.", tb.table_name.clone().unwrap_or_default());
                                    self.ctx.add_table(&tb, &cols);
                                }
                                Err(err) => {
                                    log::info!("Load the columns for table {} with an error {}", &f.name, err);
                                }
                            };
                        }
                        None => {
                            log::info!("Could not found the table {}", &f.name);
                        }
                    }
                }
                Err(err) => {
                    log::info!("Load the table {} with an error {}", &f.name, err);
                }
            };
            log::info!("Table: {}, PK: {}", f.name, f.primary_key);
        }


        for qry in self.ctx.codegen_conf.queries.clone() {
            let mut fds = Vec::new();
            for st in qry.params.clone() {
                fds.push(st.default_value.clone().unwrap_or_default());
            }

            match execute_sql(qry.base_sql.as_str(), &fds).await {
                Ok(rt) => {
                    for fd in rt.fields.clone() {
                        log::info!("Field of query: {} {}", fd.field_name, fd.field_type);
                    }
                    let st = parse_query_as_file(&self.ctx, &qry, &rt);
                    let hl = parse_query_handler_as_file(&self.ctx, &qry, &rt);
                    self.files.push(st);
                    self.files.push(hl);
                }
                Err(err) => {
                    log::info!("Execute the query with an error {}", err);
                }
            }
        }        
    }

    /**
     * Step 3
     * 根据Table来进行代码生成
     */
    pub fn generate(&mut self) {
        for tbl in self.ctx.tables.clone() {
            let columns = self.ctx.get_table_columns(&tbl.table_name.clone().unwrap_or_default());
            let st = parse_table_as_struct(&self.ctx, &tbl, &columns);
            self.ctx.add_struct(&st);
        }

        // 组织文件结构
        for sts in self.ctx.structs.clone() {
            let rfi = RustFileImpl {
                file_name: format!("{}.rs", snake_case(sts.struct_name.clone().as_str())),
                mod_name: "entity".to_string(),
                caretlist: vec![],
                usinglist: self.default_entity_using.clone(),
                structlist: vec![sts.clone()],
                funclist: vec![],
            };
            self.files.push(rfi);
        }

        for sts in self.ctx.queries.clone() {
            let rfi = RustFileImpl {
                file_name: format!("{}.rs", snake_case(sts.struct_name.clone().as_str())),
                mod_name: "query".to_string(),
                caretlist: vec![],
                usinglist: self.default_entity_using.clone(),
                structlist: vec![sts.clone()],
                funclist: vec![],
            };
            self.files.push(rfi);
        }

        for tbl in self.ctx.tables.clone() {
            let mut usinglist = vec![];
            let tbc = self.ctx.get_table_conf(&tbl.table_name.clone().unwrap_or_default()).unwrap();
            if tbc.generate_handler {
                let funclist = generate_actix_handler_for_table(&self.ctx, &tbl.clone(), &mut usinglist);

                usinglist.append(&mut self.default_handler_using.clone());
                // let tbc =  self.ctx.get_table_conf(&tbl.table_name.clone().unwrap_or_default()).unwrap();
                let rfi = RustFileImpl {
                    file_name: format!("{}.rs", snake_case(self.ctx.get_struct_name(&tbl.table_name.clone().unwrap_or_default()).unwrap().as_str()).to_string()),
                    mod_name: "handler".to_string(),
                    caretlist: vec![],
                    usinglist: usinglist,
                    structlist: vec![],
                    funclist: funclist,
                };
                self.files.push(rfi);
            }
        }


    }


    /**
     * Step 4
     * 写到文件
     * |--entity
     * |--handler
     * |--utils
     * |--main.rs
     * |--cargo.toml
     */
    pub fn write_out(&self) -> std::io::Result<()>{
        let str = self.ctx.codegen_conf.output_path.clone().as_str().to_owned();
        let root_path = Path::new(&str);
        if !root_path.exists() {
            // should create the path
            create_dir_all(root_path)?;
        }
        let src = root_path.join("src");
        if !src.exists() {
            // should create the path
            create_dir(src.clone())?;
        }
        let utils = src.join("utils");
        if !utils.exists() {
            // should create the path
            create_dir(utils.clone())?;
        }

        let utilsfile = utils.join("mod.rs");
        self.write_content(&utilsfile.to_str().unwrap_or_default().to_string(), crate::tmpl::UTILS_TMPL)?;


        let conf = root_path.join("conf");
        if !conf.exists() {
            // should create the path
            create_dir(conf.clone())?;
        }

        let cargotoml = root_path.join("Cargo.toml");
        self.write_content(&cargotoml.to_str().unwrap_or_default().to_string(), &crate::tmpl::replace_cargo_toml(&self.ctx.codegen_conf))?;

        let scoconf = conf.join("app.yml");
        let conftext = format_conf_tmpl(&self.ctx.codegen_conf.database_url.clone(), &self.ctx.codegen_conf.webserver_port.clone());
        self.write_content(&scoconf.as_path().to_str().unwrap_or_default().to_string(), conftext.as_str()) ?;

        let mut modmap = HashMap::<String, Vec<String>>::new();
        let mut service_func: Vec<String> = Vec::new();

        for fl in self.files.clone() {
            let modpath = src.join(fl.mod_name.clone());
            if !modmap.contains_key(&fl.mod_name) {
                modmap.insert(fl.mod_name.clone(), vec![]);
            }
            let mut modfiles = modmap.get(&fl.mod_name).unwrap().clone();
            modfiles.push(fl.file_name.clone());
            modmap.insert(fl.mod_name.clone(), modfiles);

            if !modpath.exists() {
                // create the path
                create_dir(modpath.clone())?;
            }

            let filename = modpath.join(fl.file_name.clone());
            fl.write_out(&filename.to_str().unwrap_or_default().to_string())?;

            if fl.mod_name == "handler" {
                for func in fl.funclist {
                    service_func.push(format!("crate::{}::{}", fl.mod_name, func.func_name).to_string());
                }
            }
        }

        let mut mainmods: Vec<String> = Vec::new(); //生成用于main.rs的mod声明
        

        for mkey in modmap {
            let mn = mkey.0.clone();
            mainmods.push(mn.clone());
            let tj = src.join(mkey.0.clone()).join("mod.rs");  // Generate the mod.rs for each folder
            let mut tjfile = OpenOptions::new()
                                .create(true)
                                .truncate(true)
                                .write(true)
                                .open(tj.as_path())?;
            for ln in mkey.1.clone() {
                let nameonly = ln.substring(0, ln.len() - 3);
                let modfmt = format!("mod {};\n", nameonly.clone());
                let usingfmt = format!("pub use {}::*;\n", nameonly.clone());
                tjfile.write_all(modfmt.as_bytes())?;
                tjfile.write_all(usingfmt.as_bytes())?;
                tjfile.write_all("\r\n".as_bytes())?;
            }
            tjfile.flush()?;
        }

        let main = src.join("main.rs");
        self.write_content(&main.to_str().unwrap_or_default().to_string(), crate::tmpl::format_main_template(mainmods, service_func).as_str())?;


        Ok(())
    }


    fn write_content(&self, filename: &String, content: &str) -> std::io::Result<()>{

        let mut file = OpenOptions::new()
                                .write(true)
                                .append(false)
                                .create(true)
                                .truncate(true)
                                .open(filename)?;
        file.write_all(content.as_bytes())?;
        file.flush()?;
        Ok(())
    }

}


pub fn parse_data_type_as_rust_type(dt: &String) -> String {
    match dt.as_str() {
        "smallint" => "i16".to_string(),
        "smallint unsigned" => "u16".to_string(),
        "int" => "i32".to_string(),
        "int unsigned" => "u32".to_string(),
        "bigint" => "i64".to_string(),
        "bigint unsigned" => "u64".to_string(),
        "tinyint" => "bool".to_string(),
        "bit" => "bool".to_string(),
        "longtext" => "String".to_string(),
        "text" => "String".to_string(),
        "mediumtext" => "String".to_string(),
        "char" => "String".to_string(),
        "varchar" => "String".to_string(),
        "float" => "f32".to_string(),
        "double" => "f64".to_string(),
        "decimal" => "rbatis::Decimal".to_string(),
        "datetime" => "rbatis::DateTimeNative".to_string(),
        "date" => "rbatis::DateNative".to_string(),
        "timestamp" => "rbatis::Timestamp".to_string(),
        "time" => "rbatis::TimeNative".to_string(),
        "blob" => "rbatis::Bytes".to_string(),
        "binary" => "rbatis::Bytes".to_string(),
        "varbinary" => "rbatis::Bytes".to_string(),
        "mediumblob" => "rbatis::Bytes".to_string(),
        "longblob" => "rbatis::Bytes".to_string(),
        "json" => "rbatis::Json".to_string(),
        _ => "String".to_string(),
    }
}

pub fn parse_column_as_field(ctx: &GenerateContext, tbl: &TableConfig, col: &ColumnInfo) -> RustStructField {
    RustStructField {
        is_pub: true,
        column_name: col.column_name.clone().unwrap_or_default(),
        field_name: safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_lowercase()),
        field_type: parse_data_type_as_rust_type(&col.data_type.clone().unwrap_or_default().to_lowercase()),
        is_option: if tbl.all_field_option {
            true
        } else {
             col.is_nullable.clone().unwrap_or_default().to_lowercase() == "yes"
        },
    }
}

pub fn parse_column_list(ctx: &GenerateContext, tbl: &TableConfig, cols: &Vec<ColumnInfo>, columns: &mut String) -> Vec<RustStructField> {
    let mut fields = vec![];

    for col in cols {
        let colname = col.column_name.clone().unwrap_or_default();
        columns.push_str(colname.as_str());
        columns.push(',');
        fields.push(parse_column_as_field(ctx, tbl, &col));
    }
    fields
}

pub fn make_skip_columns(ctx: &GenerateContext, tbl: &TableConfig) -> String {
    let mut skips = String::new();
    match tbl.update_skip_fields.clone() {
        Some(sk) => {
            for fd in sk.split(",").into_iter() {
                skips.push_str(format!("Skip::Column(\"{}\"),", fd.trim()).as_str());
            }
        }
        None => {

        }
    };

    skips
}

