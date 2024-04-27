use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{ErrorKind, Write};
use std::path::Path;

use change_case::{pascal_case, snake_case};
use std::fs::{create_dir, create_dir_all, OpenOptions};

use crate::codegen::{generate_vue_view_for_table, parse_table_as_struct};
use crate::config::{
    get_rbatis, safe_struct_field_name, CodeGenConfig, QueryConfig, RedisConfig, RelationConfig,
    TableConfig,
};
use crate::permission::ChimesPermissionInfo;
use crate::schema::{ColumnInfo, TableInfo};
use crate::tmpl::{format_conf_tmpl, format_redis_conf_tmpl};
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use substring::Substring;

use super::{
    execute_sql, generate_actix_handler_for_table, generate_js_api_for_table, generate_relation_form, is_copied_data_type, parse_data_type_as_rust_type, parse_query_as_file, parse_query_handler_as_file, parse_relation_as_file, parse_relation_handlers_as_file, parse_table_as_composite_struct, parse_table_as_request_param_struct, parse_table_as_value_object_struct, parse_yaml_as_file
};

pub trait CodeWriter {
    fn write(&self, ro: &mut RustOutput);
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RelationTable {
    pub table_info: Option<TableInfo>,
    pub table_conf: Option<TableConfig>,
    pub major_field_name: Option<String>,
    pub fields: Vec<RustStructFieldExtend>,
    pub dialog_form: bool, // 使用Dialog来进行输入，主要是针对one_to_many (one_many = true)的情况，该值为true，则输入时弹出对话框，否则输入框放在表格中。
    // 通常地，如果需要输入的字段比较少，则可以直接在表格中输入。
    pub one_many: bool, // one_many == true 为1对多关系，否则为1对1关系
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RelationForm {
    pub codegen: CodeGenConfig,
    pub table_info: Option<TableInfo>,
    pub table_conf: Option<TableConfig>,
    pub relation_conf: Option<RelationConfig>,
    pub relation_count: u64,
    pub relation_map: HashMap<String, TableConfig>,
    pub dict_list: Vec<String>,
    pub has_area: bool,
    pub fields: Vec<RustStructFieldExtend>,
    pub relations: Vec<RelationTable>,
}

/**
 * 代码生成的上下文
 * 它的主要功能是解释配置文件，并根据配置文件来准备代码生成所需要对应的一些参数
 * 这些参数如：生成后所存放的路径等
 */
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct GenerateContext {
    pub codegen_conf: CodeGenConfig,
    pub redis_conf: RedisConfig,
    pub tables: Vec<TableInfo>,
    pub columns: HashMap<String, Vec<ColumnInfo>>,
    pub structs: Vec<RustStruct>,
    pub queries: Vec<RustStruct>,
    pub permissions: Vec<RustPermission>,
}

impl GenerateContext {
    pub fn create(cgconf: &CodeGenConfig, redisconf: &RedisConfig) -> Self {
        Self {
            codegen_conf: cgconf.clone(),
            redis_conf: redisconf.clone(),
            tables: vec![],
            columns: HashMap::new(),
            structs: vec![],
            queries: vec![],
            permissions: vec![],
        }
    }

    #[allow(dead_code)]
    pub fn get_root_path(&self) -> &str {
        self.codegen_conf.output_path.as_str()
    }

    #[allow(dead_code)]
    pub fn get_entity_path(&self) -> String {
        self.get_root_path().to_owned() + "/entity"
    }

    #[allow(dead_code)]
    pub fn get_controller_path(&self) -> String {
        self.get_root_path().to_owned() + "/controller"
    }

    #[allow(dead_code)]
    pub fn get_facade_path(&self) -> String {
        self.get_root_path().to_owned() + "/facade"
    }

    #[allow(dead_code)]
    pub fn is_all_entity_in_one_file(&self) -> bool {
        self.codegen_conf.entity_in_one_file
    }

    #[allow(dead_code)]
    pub fn is_generate_lib(&self) -> bool {
        self.codegen_conf.generate_for_lib
    }

    #[allow(dead_code)]
    pub fn add_struct(&mut self, st: &RustStruct) {
        self.structs.push(st.clone());
    }

    #[allow(dead_code)]
    pub fn add_query(&mut self, st: &RustStruct) {
        self.queries.push(st.clone());
    }

    #[allow(dead_code)]
    pub fn add_table(&mut self, tb: &TableInfo, cols: &Vec<ColumnInfo>) {
        if tb.table_name.is_some() {
            // log::info!("Add the table {} into the context.", tb.table_name.clone().unwrap_or_default());
            self.tables.push(tb.clone());
            self.columns
                .insert(tb.table_name.clone().unwrap(), cols.clone());
        }
    }

    #[allow(dead_code)]
    pub fn add_permission(&mut self, tb: &TableInfo, funclist: &Vec<RustFunc>) {
        let tbname = tb.table_name.clone().unwrap_or_default();
        let tbc = self.get_table_conf(&tbname.clone()).unwrap();
        let alias = tbc.api_handler_name.clone();
        let name = if tbc.comment.is_empty() {
            let tbcmt = tb.table_comment.clone().unwrap();
            if tbcmt.trim().len() > 0 {
                tbcmt.trim().to_string()
            } else {
                tbc.struct_name.to_uppercase()
            }
        } else {
            tbc.comment.clone()
        };

        let mut children = vec![];
        for mk in funclist.clone() {
            if mk.api_method.is_some() {
                let child = RustPermission {
                    name: mk.comment.clone().unwrap_or_default(),
                    alias: mk.func_name.to_uppercase(),
                    service_id: self.codegen_conf.app_name.clone(),
                    module_id: alias.clone(),
                    api_pattern: mk.api_pattern.clone(),
                    api_method: mk.api_method.clone(),
                    api_bypass: Some("user".to_string()),
                    children: vec![],
                };
                children.push(child);
            }
        }

        let perm = RustPermission {
            name: name,
            alias: alias.to_uppercase(),
            service_id: self.codegen_conf.app_name.clone(),
            module_id: alias.clone(),
            api_pattern: Some(format!(
                "{}/{}",
                self.codegen_conf.api_handler_prefix.clone(),
                tbc.api_handler_name.clone()
            )),
            api_method: None,
            api_bypass: None,
            children: children,
        };
        self.permissions.push(perm);
    }

    #[allow(dead_code)]
    pub fn add_permission_for_relation(&mut self, tb: &RelationConfig, funclist: &Vec<RustFunc>) {
        let alias = tb.api_handler_name.clone().unwrap_or_default();
        let name = if tb.comment.trim().is_empty() {
            tb.struct_name.to_uppercase()
        } else {
            tb.comment.clone()
        };

        let mut children = vec![];
        for mk in funclist.clone() {
            if mk.api_method.is_some() {
                let child = RustPermission {
                    name: mk.comment.clone().unwrap_or_default(),
                    alias: mk.func_name.to_uppercase(),
                    service_id: self.codegen_conf.app_name.clone(),
                    module_id: alias.clone(),
                    api_pattern: mk.api_pattern.clone(),
                    api_method: mk.api_method.clone(),
                    api_bypass: Some("user".to_string()),
                    children: vec![],
                };
                children.push(child);
            }
        }

        let perm = RustPermission {
            name: name,
            alias: alias.to_uppercase(),
            service_id: self.codegen_conf.app_name.clone(),
            module_id: alias.clone(),
            api_pattern: Some(format!(
                "{}/{}",
                self.codegen_conf.api_handler_prefix.clone(),
                tb.api_handler_name.clone().unwrap_or_default()
            )),
            api_method: None,
            api_bypass: None,
            children: children,
        };
        self.permissions.push(perm);
    }

    #[allow(dead_code)]
    pub fn add_permission_for_query(&mut self, tb: &QueryConfig, funclist: &Vec<RustFunc>) {
        let alias = tb.api_handler_name.clone();
        let name = if tb.comment.trim().is_empty() {
            tb.struct_name.to_uppercase()
        } else {
            tb.comment.clone()
        };

        let mut children = vec![];
        for mk in funclist.clone() {
            if mk.api_method.is_some() {
                let child = RustPermission {
                    name: mk.comment.clone().unwrap_or_default(),
                    alias: mk.func_name.to_uppercase(),
                    service_id: self.codegen_conf.app_name.clone(),
                    module_id: alias.clone(),
                    api_pattern: mk.api_pattern.clone(),
                    api_method: mk.api_method.clone(),
                    api_bypass: Some("user".to_string()),
                    children: vec![],
                };
                children.push(child);
            }
        }

        let perm = RustPermission {
            name: name,
            alias: alias.to_uppercase(),
            service_id: self.codegen_conf.app_name.clone(),
            module_id: alias.clone(),
            api_pattern: Some(format!(
                "{}/{}",
                self.codegen_conf.api_handler_prefix.clone(),
                tb.api_handler_name.clone()
            )),
            api_method: None,
            api_bypass: None,
            children: children,
        };
        self.permissions.push(perm);
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn get_value_object_struct_name(&self, tbl: &String) -> Option<String> {
        for tc in self.codegen_conf.tables.clone() {
            if tc.name == tbl.clone() {
                if tc.struct_name.is_empty() {
                    return Some(format!("{}Value", pascal_case(tc.name.clone().as_str())));
                } else {
                    return Some(format!("{}Value", tc.struct_name.clone().as_str()));
                }
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn get_table_conf(&self, tbl: &String) -> Option<TableConfig> {
        for tc in self.codegen_conf.tables.clone() {
            if tc.name == tbl.clone() {
                return Some(tc.clone());
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn get_table_conf_by_struct_name(&self, tbl: &String) -> Option<TableConfig> {
        for tc in self.codegen_conf.tables.clone() {
            if tc.struct_name == tbl.clone() {
                return Some(tc.clone());
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn get_table_info(&self, tbl: &String) -> Option<TableInfo> {
        for tc in self.tables.clone() {
            if tc.table_name.clone().unwrap_or_default() == tbl.clone() {
                return Some(tc.clone());
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn get_relation_config(&self, tbl: &String) -> Option<RelationConfig> {
        for tc in self.codegen_conf.relations.clone() {
            if tc.major_table.clone() == tbl.clone() {
                return Some(tc.clone());
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn get_relation_config_in_relationship(&self, tbl: &String) -> Vec<RelationConfig> {
        let mut rls = vec![];
        for tc in self.codegen_conf.relations.clone() {
            for oneone in tc.one_to_one.clone() {
                if oneone.table_name == Some(tbl.clone()) {
                    rls.push(tc.clone());
                    continue;
                }
            }
            for onemany in tc.one_to_many.clone() {
                if onemany.middle_table.is_some() {
                    if onemany.middle_table == Some(tbl.clone()) {
                        rls.push(tc.clone());
                        continue;
                    }
                } else {
                    if onemany.table_name == Some(tbl.clone()) {
                        rls.push(tc.clone());
                        continue;
                    }
                }
            }
        }
        rls
    }

    #[allow(dead_code)]
    pub fn get_table_columns(&self, tbl: &String) -> Vec<ColumnInfo> {
        match self.columns.get(&tbl.clone()) {
            Some(st) => st.to_owned(),
            None => {
                vec![]
            }
        }
    }

    /**
     * check the columns is the basic info column
     * 1. 以_name, _caption, _title, 为结尾的
     * 2. 注释中包含基本信息
     * 3. 数据类型不是text这样的大字段，长度大于200的字段
     * 参数中bl的意义是用于决定哪些未确定的字段是否为基础信息。
     */
    #[allow(dead_code)]
    pub fn is_basic_column(col: &ColumnInfo, bl: bool) -> bool {
        let fd = col.column_name.clone().unwrap_or_default();
        let cmt = col.column_comment.clone().unwrap_or_default();
        let len = col.character_maximum_length.clone();
        if fd.ends_with("_name") || fd.ends_with("_caption") || fd.ends_with("_title") {
            return true;
        }
        if cmt.contains("基本信息") {
            return true;
        }
        if len.is_some() {
            let clen = len.unwrap();
            if clen > 200 {
                return false;
            }
        }
        return bl; //
    }

    #[allow(dead_code)]
    pub fn get_table_basic_columns(&self, tbl: &String, bl: bool) -> Vec<ColumnInfo> {
        match self.columns.get(&tbl.clone()) {
            Some(st) => {
                let mut basiclist = vec![];
                let mcx = st.to_owned();
                for cl in mcx.clone() {
                    if Self::is_basic_column(&cl, bl) {
                        basiclist.push(cl.clone());
                    }
                }
                basiclist
            }
            None => {
                vec![]
            }
        }
    }

    #[allow(dead_code)]
    pub fn find_table_column(&self, tbl: &String, col: &String) -> Option<ColumnInfo> {
        match self.columns.get(&tbl.clone()) {
            Some(st) => {
                for xs in st.clone() {
                    if xs.column_name.clone().unwrap_or_default().to_lowercase()
                        == col.to_lowercase()
                    {
                        return Some(xs.clone());
                    }
                }
                None
            }
            None => None,
        }
    }

    #[allow(dead_code)]
    pub fn is_copied_data_type(&self, tbl: &String, col: &String) -> bool {
        if let Some(col) = self.find_table_column(tbl, col) {
            log::info!("The column was found. {} {} {}", col.column_name.clone().unwrap_or_default(), col.data_type.clone().unwrap_or_default(), col.column_type.clone().unwrap_or_default());
            is_copied_data_type(&col.data_type.clone().unwrap_or_default()) 
        } else {
            log::info!("The column was not found. ");
            false
        }
    }

    /**
     * 从TableConfig中的主键来解决
     */
    #[allow(dead_code)]
    pub fn get_table_column_by_primary_key(&self, tbl: &String) -> Vec<ColumnInfo> {
        let mut pkeys = vec![];
        let cols = self.columns.get(&tbl.clone());
        let tbconf = self.get_table_conf(tbl);
        if tbconf.is_some() {
            let tbc = tbconf.unwrap();
            match cols {
                Some(tcls) => {
                    tbc.primary_key.split(&",".to_string()).for_each(|f| {
                        for cl in tcls {
                            if cl.column_name.clone().unwrap_or_default() == f.trim().to_string() {
                                pkeys.push(cl.clone());
                                // break;
                            }
                        }
                    });
                }
                None => {}
            };
        }

        pkeys
    }

    #[allow(dead_code)]
    pub fn get_table_pkey_column(&self, tbl: &String) -> Vec<ColumnInfo> {
        let mut pkeys = vec![];
        let cols = self.get_table_columns(tbl);
        for cl in cols {
            if cl.column_key.clone().unwrap_or_default().to_lowercase() == "pri" {
                pkeys.push(cl.clone());
                break;
            }
        }

        pkeys
    }

    #[allow(dead_code)]
    pub fn get_table_auto_incremnt_column(&self, tbl: &String) -> Option<ColumnInfo> {
        let cols = self.get_table_columns(tbl);
        for cl in cols {
            if cl.extra.clone().unwrap_or_default().to_lowercase() == "auto_increment" {
                return Some(cl.clone());
            }
        }

        None
    }

    /**
     * find the tbl name by relatation
     */
    pub fn get_relation_table_freginkeys(&self, tbl: &String) -> Vec<String> {
        let mut freginkeys = vec![];
        for rel in self.codegen_conf.relations.clone() {
            rel.one_to_many
                .into_iter()
                .filter(|f| f.table_name == Some(tbl.clone()))
                .for_each(|f| {
                    freginkeys.push(f.major_field.unwrap_or_default());
                });
        }
        freginkeys
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
    pub comment: Option<String>,
    pub api_method: Option<String>,
    pub api_pattern: Option<String>,
}

impl RustFunc {
    #[allow(dead_code)]
    pub fn add_params(&mut self, name: &String, rtype: &String) {
        self.params.push((name.clone(), rtype.clone()));
    }

    #[allow(dead_code)]
    pub fn add_bodyline(&mut self, line: &String) {
        self.bodylines.push(line.clone());
    }

    #[allow(dead_code)]
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

        if self.is_struct_fn && self.is_self_fn {
            // Should be in an struct, the self fn will valid
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
        if self.return_type.is_some() {
            if self.return_is_result {
                if self.return_is_option {
                    first.push_str(&format!(
                        " -> Result<Option<{}>, Error> {{",
                        self.return_type.clone().unwrap_or_default()
                    ));
                } else {
                    first.push_str(&format!(
                        " -> Result<{}, Error> {{",
                        self.return_type.clone().unwrap_or_default()
                    ));
                }
            } else {
                if self.return_is_option {
                    first.push_str(&format!(
                        " -> Option<{}> {{",
                        self.return_type.clone().unwrap_or_default()
                    ));
                } else {
                    first.push_str(&format!(
                        " -> {} {{",
                        self.return_type.clone().unwrap_or_default()
                    ));
                }
            }
        } else {
            first.push_str(" {");
        }
        let mut space;
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
            let mut t = 0;
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
    pub schema_name: Option<String>,
    pub column_name: String,
    pub field_name: String,
    pub orignal_field_name: Option<String>,
    pub comment: Option<String>,
    pub field_type: String,
    pub is_option: bool,
    pub length: i64,
    pub annotations: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RustStructFieldExtend {
    pub field: RustStructField,
    pub caption: String,
    pub dict: Option<String>, // 关联的字典项，在注释写明，字典{dict_name}
    pub relation: Option<String>, // 关联的其它实体，在注释写明，关联表{_name}
    pub condition: bool,      // 作为查询条件，，在注释写明，作为条件，如果日期，自动为
    pub display_table: bool,  // 显示在表格中，，在注释写明，显示在表格中，或不显示
    pub hidden: bool,         // 隐藏不需要输入，在注释写明，隐藏或无须输入
    pub required: bool,       // 是否必填，在注释写明，必填
    pub validate: Option<String>, // 是否必填，在注释写明，验证validate_type
    pub regex_check: bool,    // 指出validate包含正则表达式
    pub flag: i64,            // 万能标识
    pub alias: Option<String>, // 表别名
}

impl RustStructFieldExtend {
    fn check_dict(cmts: &Vec<&str>) -> Option<String> {
        for m in cmts.clone() {
            if m.contains("字典") {
                let dict = m.substring(2, m.len());
                return Some(dict.to_string());
            }
        }
        return None;
    }

    fn check_relation(cmts: &Vec<&str>) -> Option<String> {
        for m in cmts.clone() {
            if m.contains("关联表") {
                let dict = m.substring(3, m.len());
                return Some(dict.to_string());
            }
        }
        return None;
    }

    fn check_validate(cmts: &Vec<&str>) -> Option<String> {
        for m in cmts.clone() {
            if m.contains("验证模式") {
                let dict = m.substring(4, m.len());
                return Some(dict.to_string());
            } else if m.contains("验证") {
                let dict = m.substring(2, m.len());
                return Some(dict.to_string());
            }
        }
        return None;
    }

    fn check_regex(cmts: &Vec<&str>) -> bool {
        for m in cmts.clone() {
            if m.contains("验证模式") {
                return true;
            }
        }
        return false;
    }

    fn check_hidden(cmts: &Vec<&str>) -> bool {
        for m in cmts.clone() {
            if m.contains("隐藏") || m.contains("无须输入") {
                return true;
            }
        }
        return false;
    }

    fn check_condition(cmts: &Vec<&str>) -> bool {
        for m in cmts.clone() {
            if m.contains("作为条件") {
                return true;
            }
        }
        return false;
    }

    fn check_required(cmts: &Vec<&str>) -> bool {
        for m in cmts.clone() {
            if m.contains("必填") {
                return true;
            }
        }
        return false;
    }

    fn check_display(cmts: &Vec<&str>) -> bool {
        for m in cmts.clone() {
            if m.contains("不显示") {
                return false;
            } else if m.contains("显示") {
                return true;
            }
        }
        return true;
    }

    fn parse_internal(field: &RustStructField) -> RustStructFieldExtend {
        if field.comment.is_none() {
            RustStructFieldExtend {
                field: field.clone(),
                caption: field.field_name.clone(),
                dict: None,
                relation: None,
                condition: false,
                display_table: false,
                hidden: false,
                required: false,
                validate: None,
                regex_check: false,
                flag: 0i64,
                alias: None,
            }
        } else {
            let cmt = field
                .comment
                .clone()
                .unwrap_or_default()
                .replace("，", ",")
                .replace("；", ";");
            let seperator = Regex::new(r"([ ,;]+)").expect("Invalid regex");

            let ncmt = seperator
                .split(cmt.as_str())
                .into_iter()
                .collect::<Vec<&str>>();
            if ncmt.is_empty() {
                RustStructFieldExtend {
                    field: field.clone(),
                    caption: field.field_name.clone(),
                    dict: None,
                    relation: None,
                    condition: false,
                    display_table: false,
                    hidden: false,
                    required: false,
                    validate: None,
                    regex_check: false,
                    flag: 0i64,
                    alias: None,
                }
            } else {
                let capt = ncmt[0].to_string();

                RustStructFieldExtend {
                    field: field.clone(),
                    caption: capt.clone(),
                    dict: Self::check_dict(&ncmt),
                    relation: Self::check_relation(&ncmt),
                    condition: Self::check_condition(&ncmt),
                    display_table: Self::check_display(&ncmt),
                    hidden: Self::should_hidden(&field.field_name) || Self::check_hidden(&ncmt),
                    required: Self::check_required(&ncmt),
                    regex_check: Self::check_regex(&ncmt),
                    validate: Self::check_validate(&ncmt),
                    flag: 0i64,
                    alias: None,
                }
            }
        }
    }

    pub fn parse(field: &RustStructField) -> Self {
        Self::parse_internal(field)
    }

    fn should_hidden(fd: &str) -> bool {
        if fd == "create_by"
            || fd == "modify_by"
            || fd == "create_user_id"
            || fd == "modify_user_id"
            || fd == "modify_userid"
            || fd == "create_userid"
            || fd == "create_time"
            || fd == "update_time"
            || fd == "create_date"
            || fd == "update_date"
        {
            return true;
        }
        return false;
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RustStruct {
    pub is_pub: bool,
    pub has_paging: bool,
    pub struct_name: String,
    pub annotations: Vec<String>,
    pub fields: Vec<RustStructField>,
    pub funclist: Vec<RustFunc>,
    pub usings: Vec<String>,
}

impl RustStruct {
    #[allow(dead_code)]
    pub fn add_field(&mut self, fd: &RustStructField) {
        self.fields.push(fd.clone())
    }

    #[allow(dead_code)]
    pub fn add_func(&mut self, fd: &RustFunc) {
        self.funclist.push(fd.clone())
    }

    #[allow(dead_code)]
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

            for annt in fd.annotations.clone() {
                if !annt.is_empty() {
                    ro.write_line(&format!("    {}", annt));
                }
            }

            if fd.orignal_field_name.is_none() {
                if fd.column_name.len() > 0 && fd.column_name != fd.field_name {
                    ro.write_line(&format!(
                        "    #[serde(rename(deserialize=\"{}\"))]",
                        fd.column_name.clone()
                    ));
                }
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
    #[allow(dead_code)]
    pub fn write_line(&mut self, line: &str) {
        let newline = line.to_string() + "\n";
        self.outputs.push(newline);
    }

    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn add_using(&mut self, us: &String) {
        self.usinglist.push(us.clone());
    }

    #[allow(dead_code)]
    pub fn add_caret(&mut self, us: &String) {
        self.caretlist.push(us.clone());
    }

    #[allow(dead_code)]
    pub fn add_struct(&mut self, us: &RustStruct) {
        self.structlist.push(us.clone());
    }

    #[allow(dead_code)]
    pub fn add_func(&mut self, us: &RustFunc) {
        self.funclist.push(us.clone());
    }

    #[allow(dead_code)]
    pub fn write_out(&self, filename: &String) -> std::io::Result<()> {
        let mut ro = RustOutput::default();
        ro.write_line("/**");
        ro.write_line(format!(" * Generate the file for {}, ", self.file_name.clone()).as_str());
        ro.write_line(" */");
        for crt in self.caretlist.clone() {
            ro.write_line(format!("extern caret {};", crt).as_str());
        }
        ro.write_line("");
        let mut usings = self.usinglist.clone();

        for mut st in self.structlist.clone() {
            usings.append(&mut st.usings);
        }

        usings.sort();
        usings.dedup();
        

        for usingline in usings {
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

/**
 * Rust程序文件结构
 */
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VueAndJSFileImpl {
    pub file_name: String,
    pub mod_name: String,
    pub js_vue: bool, // Save into a folder
    pub usinglist: Vec<String>,
    pub funclist: Vec<String>,
}

impl VueAndJSFileImpl {
    #[allow(dead_code)]
    pub fn add_using(&mut self, us: &String) {
        self.usinglist.push(us.clone());
    }

    #[allow(dead_code)]
    pub fn add_func(&mut self, us: &String) {
        self.funclist.push(us.clone());
    }

    #[allow(dead_code)]
    pub fn write_out(&self, filename: &String) -> std::io::Result<()> {
        let mut ro = RustOutput::default();
        for usingline in self.usinglist.clone() {
            ro.write_line(format!("import {};", usingline).as_str());
        }
        for func in self.funclist.clone() {
            ro.write_line(func.as_str());
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
pub struct RustPermission {
    pub name: String,
    pub alias: String,
    pub service_id: String,
    pub module_id: String,
    pub api_pattern: Option<String>,
    pub api_method: Option<String>,
    pub api_bypass: Option<String>,
    pub children: Vec<RustPermission>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CodeGenerator {
    pub ctx: GenerateContext,
    pub files: Vec<RustFileImpl>,
    pub vuejs: Vec<VueAndJSFileImpl>,
    //pub default_entity_using: Vec<String>,
    //pub default_handler_using: Vec<String>,
}

pub enum CodeModelType {
    Entity,
    Query,
    Relation,
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
            vuejs: vec![],
            // default_entity_using: Self::get_default_entity_using(true),
            // default_handler_using: Self::get_default_handler_using(true),
        }
    }

    pub fn get_default_entity_using(
        _ctx: &GenerateContext,
        paging: bool,
        attachment: bool,
        rel: CodeModelType,
    ) -> Vec<String> {
        let mut list = vec![];
        list.push("std::fmt::Debug".to_string());
        list.push("serde_derive::{Deserialize, Serialize}".to_string());
        list.push("rbatis::rbatis::Rbatis".to_string());
        // list.push("rbatis::executor::{ Executor, ExecutorMut }".to_string());
        list.push("rbatis::error::Error".to_string());
        if paging {
            list.push("rbatis::Page".to_string());
            list.push("rbatis::PageRequest".to_string());
        }
        match rel {
            CodeModelType::Entity => {
                list.push("rbatis::crud_table".to_string());
                list.push("rbatis::crud::{CRUD, CRUDMut, Skip}".to_string());
                list.push("chimes_utils::CommonSearch".to_string());
                list.push("rbson::Bson".to_string());
                list.push(
                    "rbatis::executor::{RbatisRef, RBatisTxExecutor}".to_string(),
                );
            },
            CodeModelType::Query => {
                list.push("rbatis::crud_table".to_string());
                list.push("rbatis::crud::CRUD".to_string());
            },
            _ => {
                list.push(
                    "rbatis::executor::{RbatisRef, RBatisTxExecutor}".to_string(),
                );
            }
        }
        
        if attachment {
            list.push("chimes_rust::{ChimesAttachmentInfo, ChimesAttachmentRefInfo}".to_string());
        }

        //if ctx.codegen_conf.allow_bool_widecard {
        //    list.push("chimes_utils::bool_from_str".to_string());
        //}
        //if ctx.codegen_conf.allow_number_widecard {
        //    list.push(
        //        "chimes_utils::{i64_from_str,u64_from_str,f64_from_str,f32_from_str}".to_string(),
        //    );
        //}
        list
    }

    pub fn get_default_handler_using(_ctx: &GenerateContext, _paging: bool, common_search: bool) -> Vec<String> {
        let mut list = vec![];

        list.push("chimes_rust::{ChimesUserInfo, SystemUser}".to_string());
        list.push("chimes_utils::get_rbatis".to_string());
        if common_search {
            list.push("chimes_utils::CommonSearch".to_string());
        }
        list.push("chimes_auth::ApiResult".to_string());
        list.push("actix_web::{web, HttpResponse, Result}".to_string());
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
                            // log::info!("Columns of table {} {} {} will be fetching.", tb.table_name.clone().unwrap_or_default(), tb.table_schema.clone().unwrap_or_default(), tb.table_catalog.clone().unwrap_or_default());
                            match ColumnInfo::load_columns(rb, &ts.clone(), &tn.clone()).await {
                                Ok(cols) => {
                                    // log::info!("The table {} will be added.", tb.table_name.clone().unwrap_or_default());
                                    self.ctx.add_table(&tb, &cols);
                                }
                                Err(err) => {
                                    log::info!(
                                        "Load the columns for table {} with an error {}",
                                        &f.name,
                                        err
                                    );
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
            // log::info!("Table: {}, PK: {}", f.name, f.primary_key);
        }

        for qry in self.ctx.codegen_conf.queries.clone() {
            let mut fds = Vec::new();
            for st in qry.params.clone() {
                fds.push(st.default_value.clone().unwrap_or_default());
            }
            log::info!("Query: {}", qry.base_sql);
            match execute_sql(&self.ctx, qry.base_sql.as_str(), &fds).await {
                Ok(rt) => {
                    let st = parse_query_as_file(&self.ctx, &qry, &rt);
                    self.files.push(st);
                    if qry.generate_handler {
                        let hl = parse_query_handler_as_file(&mut self.ctx, &qry, &rt);
                        self.files.push(hl);
                    }
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
        let mut hashm = HashMap::new();
        let mut paramhm = HashMap::new();
        let mut composite_map = HashMap::new();
        for tbl in self.ctx.tables.clone() {
            let columns = self
                .ctx
                .get_table_columns(&tbl.table_name.clone().unwrap_or_default());
            let st = parse_table_as_struct(&self.ctx, &tbl, &columns);
            self.ctx.add_struct(&st);

            let stprt = parse_table_as_composite_struct(&self.ctx, &tbl, &columns);
            if stprt.is_some() {
                self.ctx.add_struct(&stprt.clone().unwrap());
            }

            composite_map.insert(tbl.table_name.clone().unwrap_or_default(), stprt.clone());

            let tbcc = self
                .ctx
                .get_table_conf(&tbl.table_name.clone().unwrap_or_default())
                .unwrap();
            if tbcc.tree_parent_field.is_some() {
                let mut usings = vec![];
                let stvo = parse_table_as_value_object_struct(&self.ctx, &tbl, &columns, &mut usings);
                hashm.insert(st.struct_name.to_string(), stvo);
            }
            if tbcc.generate_param_struct {
                let mtvo = parse_table_as_request_param_struct(&self.ctx, &tbl, &columns);
                paramhm.insert(st.struct_name.to_string(), mtvo);
            }
        }

        // 组织文件结构
        for sts in self.ctx.structs.clone() {
            let mut stlist = vec![];
            stlist.push(sts.clone());
            if hashm.contains_key(&sts.struct_name.clone()) {
                let mx = hashm[&sts.struct_name.clone()].clone();
                stlist.push(mx.clone());
            }

            if paramhm.contains_key(&sts.struct_name.clone()) {
                let mx = paramhm[&sts.struct_name.clone()].clone();
                stlist.push(mx.clone());
            }

            let tbc = self.ctx.get_table_conf_by_struct_name(&sts.struct_name);
            let attachment = if tbc.is_none() {
                false
            } else {
                tbc.unwrap().with_attachment
            };

            let rfi = RustFileImpl {
                file_name: format!("{}.rs", snake_case(sts.struct_name.clone().as_str())),
                mod_name: "entity".to_string(),
                caretlist: vec![],
                usinglist: Self::get_default_entity_using(&self.ctx, sts.has_paging, attachment, CodeModelType::Entity),
                structlist: stlist,
                funclist: vec![],
            };
            self.files.push(rfi);
        }

        for sts in self.ctx.queries.clone() {
            let rfi = RustFileImpl {
                file_name: format!("{}.rs", snake_case(sts.struct_name.clone().as_str())),
                mod_name: "query".to_string(),
                caretlist: vec![],
                usinglist: Self::get_default_entity_using(&self.ctx, sts.has_paging, false, CodeModelType::Query),
                structlist: vec![sts.clone()],
                funclist: vec![],
            };
            self.files.push(rfi);
        }

        for tbl in self.ctx.tables.clone() {
            let mut usinglist = vec![];
            let tbl_name = tbl.table_name.clone().unwrap_or_default();
            let tbc = self.ctx.get_table_conf(&tbl_name.clone()).unwrap();
            let comp = &composite_map[&tbl_name.clone()];
            if tbc.generate_handler {
                let funclist = generate_actix_handler_for_table(
                    &mut self.ctx,
                    &tbl.clone(),
                    &mut usinglist,
                    comp,
                );

                usinglist.append(&mut Self::get_default_handler_using(
                    &self.ctx,
                    tbc.page_query,
                    tbc.using_common_search
                ));
                // let tbc =  self.ctx.get_table_conf(&tbl.table_name.clone().unwrap_or_default()).unwrap();
                let rfi = RustFileImpl {
                    file_name: format!(
                        "{}.rs",
                        snake_case(
                            self.ctx
                                .get_struct_name(&tbl.table_name.clone().unwrap_or_default())
                                .unwrap()
                                .as_str()
                        )
                        .to_string()
                    ),
                    mod_name: "handler".to_string(),
                    caretlist: vec![],
                    usinglist: usinglist,
                    structlist: vec![],
                    funclist: funclist,
                };
                self.files.push(rfi);

                let jsapi = generate_js_api_for_table(&mut self.ctx, &tbl);
                let vjsfile = VueAndJSFileImpl {
                    file_name: tbc.api_handler_name.clone() + ".js",
                    mod_name: "".to_string(),
                    js_vue: false,
                    usinglist: vec![],
                    funclist: vec![jsapi.clone()],
                };
                self.vuejs.push(vjsfile.clone());

                let jsvue = generate_vue_view_for_table(&mut self.ctx, &tbl, comp);
                let vuefile = VueAndJSFileImpl {
                    file_name: "index.vue".to_string(),
                    mod_name: tbc.api_handler_name.clone(),
                    js_vue: true,
                    usinglist: vec![],
                    funclist: jsvue.clone(),
                };
                self.vuejs.push(vuefile.clone());
            }
        }

        for rel in self.ctx.codegen_conf.relations.clone() {
            match parse_relation_as_file(&self.ctx, &rel) {
                Some(rfi) => {
                    self.files.push(rfi);
                }
                None => {
                    log::info!(
                        "Could not generated relation entity for {}",
                        rel.major_table
                    );
                }
            }

            if rel.generate_handler {
                match parse_relation_handlers_as_file(&mut self.ctx, &rel) {
                    Some(rfi) => {
                        self.files.push(rfi);
                    }
                    None => {
                        log::info!(
                            "Could not generated relation handler for {}",
                            rel.major_table
                        );
                    }
                }
            }

            if rel.generate_form {
                let text = generate_relation_form(&mut self.ctx, &rel);
                let jsvue = vec![text];
                let vuefile = VueAndJSFileImpl {
                    file_name: "form.vue".to_string(),
                    mod_name: rel.api_handler_name.clone().unwrap_or_default(),
                    js_vue: true,
                    usinglist: vec![],
                    funclist: jsvue.clone(),
                };
                self.vuejs.push(vuefile.clone());
            }
        }

        match self.ctx.codegen_conf.config_template_generate.clone() {
            // should generate the config template parse
            Some(fl) => {
                let rfi = parse_yaml_as_file(&fl, &"app_config.rs".to_string());
                if !rfi.structlist.is_empty() {
                    self.files.push(rfi);
                }
            }
            None => {}
        };
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
    pub fn write_out(&self) -> std::io::Result<()> {
        let str = self
            .ctx
            .codegen_conf
            .output_path
            .clone()
            .as_str()
            .to_owned();
        let root_path = Path::new(&str);
        if !root_path.exists() {
            // should create the path
            create_dir_all(root_path)?;
        }
        let frontpath = root_path.join("front");
        if !frontpath.exists() {
            // should create the path
            create_dir(frontpath.clone())?;
        }
        let frontapipath = frontpath.clone().join("api");
        if !frontapipath.exists() {
            // should create the path
            create_dir(frontapipath.clone())?;
        }
        let frontviewpath = frontpath.clone().join("views");
        if !frontviewpath.exists() {
            // should create the path
            create_dir(frontviewpath.clone())?;
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
        self.write_content(
            &utilsfile.to_str().unwrap_or_default().to_string(),
            crate::tmpl::UTILS_TMPL,
        )?;

        let conf = root_path.join("conf");
        if !conf.exists() {
            // should create the path
            create_dir(conf.clone())?;
        }

        let cargotoml = root_path.join("Cargo.toml");
        self.write_content(
            &cargotoml.to_str().unwrap_or_default().to_string(),
            &crate::tmpl::replace_cargo_toml(&self.ctx.codegen_conf),
        )?;

        let scoconf = conf.join("app.yml");
        let conftext = format_conf_tmpl(
            &self.ctx.codegen_conf.database_url.clone(),
            &self.ctx.codegen_conf.webserver_port.clone(),
        );

        if self.ctx.redis_conf.has_redis {
            let redisconf = format_redis_conf_tmpl(
                &self.ctx.redis_conf.host,
                self.ctx.redis_conf.port.clone(),
                &self.ctx.redis_conf.username,
                &self.ctx.redis_conf.password,
                self.ctx.redis_conf.db.clone(),
            );
            let wholeconf = conftext + redisconf.as_str();
            self.write_content(
                &scoconf.as_path().to_str().unwrap_or_default().to_string(),
                wholeconf.as_str(),
            )?;
        } else {
            self.write_content(
                &scoconf.as_path().to_str().unwrap_or_default().to_string(),
                conftext.as_str(),
            )?;
        }
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
                    service_func
                        .push(format!("crate::{}::{}", fl.mod_name, func.func_name).to_string());
                }
            }
        }

        let mut mainmods: Vec<String> = Vec::new(); //生成用于main.rs的mod声明

        for mkey in modmap {
            let mn = mkey.0.clone();
            mainmods.push(mn.clone());
            let tj = src.join(mkey.0.clone()).join("mod.rs"); // Generate the mod.rs for each folder
            let tjfile = if self.ctx.codegen_conf.always_override {
                OpenOptions::new()
                    .write(true)
                    .append(false)
                    .create(true)
                    .truncate(true)
                    .open(tj.as_path())
            } else {
                OpenOptions::new()
                    .write(true)
                    .append(false)
                    .create_new(true)
                    .truncate(true)
                    .open(tj.as_path())
            };
            match tjfile {
                Ok(mjfile) => {
                    let mut mutfile = mjfile;
                    for ln in mkey.1.clone() {
                        let nameonly = ln.substring(0, ln.len() - 3);
                        let modfmt = format!("mod {};\n", nameonly);
                        let usingfmt = format!("pub use {}::*;\n", nameonly);
                        mutfile.write_all(modfmt.as_bytes())?;
                        mutfile.write_all(usingfmt.as_bytes())?;
                        mutfile.write_all("\r\n".as_bytes())?;
                    }
                    mutfile.flush()?;
                }
                Err(err) => {
                    if err.kind() == ErrorKind::AlreadyExists {
                        log::info!("Skipped the existed file.");
                    } else {
                        log::info!("File was not create/opened. Becuase {}", err);
                    }
                }
            }
        }

        let main = src.join("main.rs");
        self.write_content(
            &main.to_str().unwrap_or_default().to_string(),
            crate::tmpl::format_main_template(mainmods, service_func).as_str(),
        )?;

        for fl in self.vuejs.clone() {
            if fl.js_vue == true {
                let schpath = frontviewpath
                    .clone()
                    .join(self.ctx.codegen_conf.schema_name.clone());
                if !schpath.exists() {
                    // should create the path
                    create_dir(schpath.clone())?;
                }
                let modpath = schpath.clone().join(fl.mod_name.clone());
                if !modpath.exists() {
                    // should create the path
                    create_dir(modpath.clone())?;
                }
                let filename = modpath.join(fl.file_name.clone());
                fl.write_out(&filename.to_str().unwrap_or_default().to_string())?;
            } else {
                let modpath = frontapipath
                    .clone()
                    .join(self.ctx.codegen_conf.schema_name.clone());
                if !modpath.exists() {
                    // should create the path
                    create_dir(modpath.clone())?;
                }
                let filename = modpath.join(fl.file_name.clone());
                fl.write_out(&filename.to_str().unwrap_or_default().to_string())?;
            }
        }

        Ok(())
    }

    fn write_content(&self, filename: &String, content: &str) -> std::io::Result<()> {
        let file = if self.ctx.codegen_conf.always_override {
            OpenOptions::new()
                .write(true)
                .append(false)
                .create(true)
                .truncate(true)
                .open(filename)
        } else {
            OpenOptions::new()
                .write(true)
                .append(false)
                .create_new(true)
                .truncate(true)
                .open(filename)
        };
        match file {
            Ok(mjfile) => {
                let mut mutfile = mjfile;
                mutfile.write_all(content.as_bytes())?;
                mutfile.flush()?;
            }
            Err(err) => {
                if err.kind() == ErrorKind::AlreadyExists {
                    log::info!("Skipped the existed file.");
                } else {
                    log::info!("File was not create/opened. Becuase {}", err);
                }
            }
        };
        Ok(())
    }

    /**
     * 将Permission写入到数据库
     */
    pub async fn write_permission(&self) {
        let rb = get_rbatis();
        for ele in self.ctx.permissions.clone() {
            let mut perm = ChimesPermissionInfo {
                id: None,
                alias: Some(ele.alias.clone()),
                create_time: None,
                name: Some(ele.name.clone()),
                pid: Some(0i64),
                api_pattern: ele.api_pattern,
                service_id: Some(ele.service_id.clone()),
                api_method: ele.api_method,
                api_bypass: ele.api_bypass,
            };
            // log::info!("Permission: {} {}", ele.name.clone(), ele.alias.clone());

            let mut query = ChimesPermissionInfo::default();
            query.alias = Some(ele.alias.clone());
            query.service_id = Some(ele.service_id.clone());
            let stid = match query.query_list(rb).await {
                Ok(rs) => {
                    if rs.len() > 0 {
                        let mut mp = rs[0].clone();
                        mp.name = perm.name.clone();
                        // mp.api_bypass = perm.api_bypass.clone();
                        mp.api_method = perm.api_method.clone();
                        mp.api_pattern = perm.api_pattern.clone();
                        match mp.update(rb).await {
                            Ok(_r) => rs[0].id.unwrap_or_default(),
                            Err(_) => rs[0].id.unwrap_or_default(),
                        }
                    } else {
                        match perm.save(rb).await {
                            Ok(_r) => perm.id.unwrap(),
                            Err(err) => {
                                log::info!("Error: {}", err.to_string());
                                0i64
                            }
                        }
                    }
                }
                Err(err) => {
                    log::info!("Error: {}", err.to_string());
                    0i64
                }
            };
            if stid != 0i64 {
                for chl in ele.children.clone() {
                    let mut chperm = ChimesPermissionInfo {
                        id: None,
                        alias: Some(chl.alias.clone()),
                        create_time: None,
                        name: Some(chl.name.clone()),
                        pid: Some(stid),
                        api_pattern: chl.api_pattern,
                        service_id: Some(chl.service_id.clone()),
                        api_method: chl.api_method,
                        api_bypass: chl.api_bypass,
                    };
                    match chperm.save_or_update(rb).await {
                        Ok(_) => {}
                        Err(err) => {
                            log::info!("Error: {}", err.to_string());
                        }
                    }
                }
            }
        }
    }
}

pub fn parse_data_type_annotions(ctx: &GenerateContext, field_type: &String, using_list: &mut Vec<String>) -> Vec<String> {
    let mut annts = vec![];
    if field_type == "bool" {
        if ctx.codegen_conf.allow_bool_widecard {
            annts.push("#[serde(default)]".to_string());
            annts.push("#[serde(deserialize_with=\"bool_from_str\")]".to_string());
            using_list.push("chimes_utils::bool_from_str".to_string());
        }
    }
    if field_type == "i64" {
        if ctx.codegen_conf.allow_number_widecard {
            annts.push("#[serde(default)]".to_string());
            annts.push("#[serde(deserialize_with=\"i64_from_str\")]".to_string());
            using_list.push("chimes_utils::i64_from_str".to_string());
        }
    }
    if field_type == "u64" {
        if ctx.codegen_conf.allow_number_widecard {
            annts.push("#[serde(default)]".to_string());
            annts.push("#[serde(deserialize_with=\"u64_from_str\")]".to_string());
            using_list.push("chimes_utils::u64_from_str".to_string());
        }
    }
    if field_type == "f64" {
        if ctx.codegen_conf.allow_number_widecard {
            annts.push("#[serde(default)]".to_string());
            annts.push("#[serde(deserialize_with=\"f64_from_str\")]".to_string());
            using_list.push("chimes_utils::f64_from_str".to_string());
        }
    }
    if field_type == "f32" {
        if ctx.codegen_conf.allow_number_widecard {
            annts.push("#[serde(default)]".to_string());
            annts.push("#[serde(deserialize_with=\"f32_from_str\")]".to_string());
            using_list.push("chimes_utils::f32_from_str".to_string());
        }
    }
    annts
}

pub fn parse_column_as_field(
    ctx: &GenerateContext,
    tbl: &TableConfig,
    col: &ColumnInfo,
    rename_id: bool,
    using_list: &mut Vec<String>
) -> RustStructField {
    let field_type =
        parse_data_type_as_rust_type(&col.data_type.clone().unwrap_or_default().to_lowercase());
    // log::info!("FieldType: {}", &field_type.clone());
    let annts = parse_data_type_annotions(ctx, &field_type, using_list);

    // log::info!("{} is {} -- {}.", col.column_name.clone().unwrap_or_default(), col.extra.clone().unwrap_or_default().to_lowercase(), col.column_key.clone().unwrap_or_default());

    let mut opt_field_name = None;
    let original_field_name =
        safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_lowercase());
    let field_name = if rename_id {
        if col.extra.clone().unwrap_or_default().to_lowercase() == "auto_increment"
            && col.column_key.clone().unwrap_or_default().to_lowercase() == "pri"
        {
            opt_field_name = Some(original_field_name.clone());
            "id".to_string()
        } else {
            original_field_name.clone()
        }
    } else {
        original_field_name.clone()
    };

    RustStructField {
        is_pub: true,
        schema_name: col.table_name.clone(),
        column_name: col.column_name.clone().unwrap_or_default(),
        field_name: field_name,
        field_type: field_type.clone(),
        orignal_field_name: opt_field_name,
        comment: col.column_comment.clone(),
        is_option: if tbl.all_field_option {
            true
        } else {
            col.is_nullable.clone().unwrap_or_default().to_lowercase() == "yes"
        },
        length: col.character_maximum_length.unwrap_or_default() as i64,
        annotations: annts,
    }
}

pub fn parse_column_list(
    ctx: &GenerateContext,
    tbl: &TableConfig,
    cols: &Vec<ColumnInfo>,
    columns: &mut String,
    rename_id: bool,
    using_list: &mut Vec<String>,
) -> Vec<RustStructField> {
    let mut fields = vec![];

    for col in cols {
        let colname = col.column_name.clone().unwrap_or_default();
        columns.push_str(colname.as_str());
        columns.push(',');
        fields.push(parse_column_as_field(ctx, tbl, &col, rename_id, using_list));
    }
    fields
}

pub fn make_skip_columns(_ctx: &GenerateContext, tbl: &TableConfig) -> String {
    let mut skips = String::new();
    match tbl.update_skip_fields.clone() {
        Some(sk) => {
            for fd in sk.split(",").into_iter() {
                skips.push_str(format!("Skip::Column(\"{}\"),", fd.trim()).as_str());
            }
        }
        None => {}
    };

    skips
}
