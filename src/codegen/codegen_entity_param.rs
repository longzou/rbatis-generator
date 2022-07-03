use std::clone;

use change_case::{pascal_case, snake_case};
use crate::codegen::{RustStructField, GenerateContext, RustStruct, RustFunc, parse_data_type_as_rust_type, parse_column_list, make_skip_columns, parse_data_type_annotions};
use crate::config::{TableConfig, get_rbatis, safe_struct_field_name, SimpleFuncation};
use crate::schema::{TableInfo, ColumnInfo};
use substring::Substring;



pub fn parse_column_as_param_field(ctx: &GenerateContext, tbl: &TableConfig, col: &ColumnInfo) -> RustStructField {
    let field_type = parse_data_type_as_rust_type(&col.data_type.clone().unwrap_or_default().to_lowercase());

    let annts = parse_data_type_annotions(ctx, &field_type);
    let original_field_name = safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_lowercase());
    let field_name = original_field_name.clone();

    RustStructField {
        is_pub: true,
        column_name: col.column_name.clone().unwrap_or_default(),
        field_name: field_name,
        field_type: field_type.clone(),
        orignal_field_name: None,
        is_option: if tbl.all_field_option {
            true
        } else {
            col.is_nullable.clone().unwrap_or_default().to_lowercase() == "yes"
        },
        annotations: annts,
    }
}

fn is_date_time_type(dt: &String) -> bool {
    match dt.as_str() {
        "rbatis::DateTimeNative" => true,
        "rbatis::DateNative" => true,
        "rbatis::TimeNative" => true,
        "rbatis::DateTimeUtc" => true,
        "rbatis::DateUtc" => true,
        "rbatis::TimeUtc" => true,
        "rbatis::Timestamp" => true,
        "rbatis::TimestampZ" => true,
        "DateTimeNative" => true,
        "DateNative" => true,
        "TimeNative" => true,
        "DateTimeUtc" => true,
        "DateUtc" => true,
        "TimeUtc" => true,
        "Timestamp" => true,
        "TimestampZ" => true,
        _ => false
    }
}

fn is_multi_item_field(safe_fdname: &String) -> bool {
    safe_fdname.ends_with("id") 
        || safe_fdname.ends_with("status") 
        || safe_fdname.ends_with("category") 
        || safe_fdname.ends_with("_no") 
        || safe_fdname.ends_with("_type")
        || safe_fdname.ends_with("code")
}

/**
 * 与Entity的struct不同的地方在于：
 * 1. 有sort_by字段，vec<String>
 * 2. 日期都为vec<DateTimeNative>, 可以使用 between and进行查询
 * 3. 以id结尾的都提供一个ids的vec<xx>的字段，可以进行多选
 */
fn parse_param_column_list(ctx: &GenerateContext, tbl: &TableConfig, cols: &Vec<ColumnInfo>) -> Vec<RustStructField> {
    let mut fields = vec![];

    for col in cols {
        let colname = col.column_name.clone().unwrap_or_default();
        let cp = parse_column_as_param_field(ctx, tbl, &col);
        

        if cp.field_name.ends_with("id") 
            || cp.field_name.ends_with("status") 
            || cp.field_name.ends_with("category") 
            || cp.field_name.ends_with("_no") 
            || cp.field_name.ends_with("_type") {
            let mut cpmt = cp.clone();
            cpmt.field_name = format!("{}s", cp.field_name.clone());
            cpmt.orignal_field_name = Some(format!("{}s", cp.field_name.clone()));
            cpmt.field_type = format!("Vec<{}>", cp.field_type.clone());
            cpmt.annotations = vec!["#[serde(default)]".to_string()];
            cpmt.is_option = false;
            fields.push(cpmt);
            fields.push(cp.clone());
        } else if is_date_time_type(&cp.field_type) {
            let mut cpmt = cp.clone();
            cpmt.field_name = format!("{}", cp.field_name.clone());
            cpmt.field_type = format!("Vec<{}>", cp.field_type.clone());
            cpmt.annotations = vec!["#[serde(default)]".to_string()];
            cpmt.is_option = false;
            fields.push(cpmt);
        } else {
            fields.push(cp.clone());
        }
    }
    fields
}

pub fn parse_table_as_request_param_struct(ctx: &GenerateContext, tbl: &TableInfo, cols: &Vec<ColumnInfo>) -> RustStruct {
    let mut columns = String::new();
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone());

    if tbc.is_none() {
        return RustStruct::default();
    }

    let tbconf = tbc.unwrap();

    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    
    let fields = parse_param_column_list(ctx, &tbconf, cols);
    if columns.ends_with(",") {
        columns = columns.substring(0, columns.len() - 1).to_string();
    }
    
    let anno = vec!["#[derive(Debug, Clone, Default, Deserialize, Serialize)]".to_string()];

    let mut funclist = vec![];

    if tbconf.page_query {
        let page_func = generate_func_page_query_for_struct(ctx, tbl);
        funclist.push(page_func);
    }
    
    let query_list_func = generate_func_list_query_for_struct(ctx, tbl);
    funclist.push(query_list_func);

    if tbconf.tree_parent_field.is_some() {
        let treefunc = generate_func_tree_query_for_struct(ctx, tbl);
        funclist.push(treefunc);
    }

    RustStruct {
        is_pub: true,
        has_paging: tbconf.page_query,
        struct_name: match ctx.get_struct_name(&tbl_name.clone()) {
            Some(t) => format!("{}Query", t),
            None => {
                let st = pascal_case(tbl_name.clone().as_str());
                format!("{}Query", st)
            }
        },
        annotations: anno,
        fields: fields,
        funclist: funclist,
    }
}


/**
 * 分页查询
 * 根据字段来处理
 */
fn generate_func_page_query_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    let tbl_struct = tbconf.struct_name.clone();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut allcols = ctx.get_table_columns(&tbl_name.clone());
    
    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));
    params.push(("curr".to_string(), "u64".to_string()));
    params.push(("ps".to_string(), "u64".to_string()));
    
   
    let mut body = vec![];
    
    body.push(format!("let wp = rb.new_wrapper()"));
    for col in allcols.clone() {
        let field_type = parse_data_type_as_rust_type(&col.data_type.clone().unwrap_or_default().to_lowercase());
        let safe_fdname = safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_string().to_lowercase());
        if tbconf.tree_parent_field.clone().unwrap_or_default().to_lowercase() == safe_fdname.clone() {
            body.push(format!("         .r#if(self.{}.clone().is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
            if tbconf.tree_root_value.clone().unwrap_or_default().to_lowercase() == "null".to_string() {
                body.push(format!("         .r#if(self.{}.clone().is_none(), |w| w.and().is_null(\"{}\"))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default()));
            } else {
                body.push(format!("         .r#if(self.{}.clone().is_none(), |w| w.and().eq(\"{}\", Some({})))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), tbconf.tree_root_value.clone().unwrap_or_default().to_lowercase()));
            }
        } else {
            if is_date_time_type(&field_type) {
                body.push(format!("         .r#if(self.{}.clone().len() == 1, |w| w.and().gt(\"{}\", self.{}[0].clone()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
                body.push(format!("         .r#if(self.{}.clone().len() >= 2, |w| w.and().between(\"{}\", self.{}[0].clone(), self.{}[1].clone()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone(), safe_fdname.clone()));
            } else if is_multi_item_field(&safe_fdname) {
                body.push(format!("         .r#if(self.{}.clone().is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
                body.push(format!("         .r#if(self.{}s.clone().is_empty() == false, |w| w.and().r#in(\"{}\", &self.{}s.clone()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
            } else {
                body.push(format!("         .r#if(self.{}.clone().is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
            }
        }
    }
    // body.remove(body.len() - 1);
    let last = body.remove(body.len() - 1);
    body.push(last + ";");

    let mut savestr = String::new();
    savestr.push_str(format!("rb.fetch_page_by_wrapper::<{}>(wp, &PageRequest::new(curr, ps)).await", tbl_struct.clone()).as_str());

    body.push(savestr);
    RustFunc { 
        is_struct_fn: true, 
        is_self_fn: true,
        is_self_mut: false,
        is_pub: true, 
        is_async: true, 
        func_name: "query_paged".to_string(), 
        return_is_option: false,
        return_is_result: true, 
        return_type: Some(format!("Page<{}>", tbl_struct.clone())),
        params: params, 
        bodylines: body,
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(tbconf.comment.clone()),
        api_method: None,
        api_pattern: None,
    }
}


/**
 * 分页查询
 * 根据字段来处理
 */
fn generate_func_list_query_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    let tbl_struct = tbconf.struct_name.clone();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut allcols = ctx.get_table_columns(&tbl_name.clone());
    
    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));
   
    let mut body = vec![];
    
    body.push(format!("let wp = rb.new_wrapper()"));
    for col in allcols.clone() {
        let field_type = parse_data_type_as_rust_type(&col.data_type.clone().unwrap_or_default().to_lowercase());
        // let field_name = safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_lowercase());
        let safe_fdname = safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_string().to_lowercase());
        if tbconf.tree_parent_field.clone().unwrap_or_default().to_lowercase() == safe_fdname.clone() {
            body.push(format!("         .r#if(self.{}.clone().is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
            if tbconf.tree_root_value.clone().unwrap_or_default().to_lowercase() == "null".to_string() {
                body.push(format!("         .r#if(self.{}.clone().is_none(), |w| w.and().is_null(\"{}\"))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default()));
            } else {
                body.push(format!("         .r#if(self.{}.clone().is_none(), |w| w.and().eq(\"{}\", Some({})))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), tbconf.tree_root_value.clone().unwrap_or_default().to_lowercase()));
            }
        } else {
            if is_date_time_type(&field_type) {
                body.push(format!("         .r#if(self.{}.clone().len() == 1, |w| w.and().gt(\"{}\", self.{}[0].clone()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
                body.push(format!("         .r#if(self.{}.clone().len() >= 2, |w| w.and().between(\"{}\", self.{}[0].clone(), self.{}[1].clone()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone(), safe_fdname.clone()));
            } else if is_multi_item_field(&safe_fdname) {
                body.push(format!("         .r#if(self.{}.clone().is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
                body.push(format!("         .r#if(self.{}s.clone().is_empty() == false, |w| w.and().r#in(\"{}\", &self.{}s.clone()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
            } else {
                body.push(format!("         .r#if(self.{}.clone().is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
            }
        }        
    }
    // body.remove(body.len() - 1);
    let last = body.remove(body.len() - 1);
    body.push(last + ";");

    let mut savestr = String::new();
    savestr.push_str(format!("rb.fetch_list_by_wrapper::<{}>(wp).await", tbl_struct.clone()).as_str());

    body.push(savestr);
    RustFunc { 
        is_struct_fn: true, 
        is_self_fn: true,
        is_self_mut: false,
        is_pub: true, 
        is_async: true, 
        func_name: "query_list".to_string(), 
        return_is_option: false,
        return_is_result: true, 
        return_type: Some(format!("Vec<{}>", tbl_struct.clone())),
        params: params, 
        bodylines: body,
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(tbconf.comment.clone()),
        api_method: None,
        api_pattern: None,
    }
}

/**
 * 分页查询
 * 根据字段来处理
 */
fn generate_func_tree_query_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbc = tblinfo.unwrap();
    let tbl_struct = tbc.struct_name.clone();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut allcols = ctx.get_table_columns(&tbl_name.clone());
    
    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));

    let treecolopt = ctx.find_table_column(&tbl_name.clone(), &tbc.tree_parent_field.unwrap_or_default());
    let treecol = treecolopt.unwrap();

    params.push(("pid".to_string(), format!("&Option<{}>", parse_data_type_as_rust_type(&treecol.data_type.clone().unwrap_or_default()))));
   
    let root_value = tbc.tree_root_value.unwrap_or_default();
    let mut body = vec![];
    
    body.push(format!("let wp = rb.new_wrapper()"));
    // for col in allcols.clone() {
    let safe_fdname = safe_struct_field_name(&treecol.column_name.clone().unwrap_or_default().to_string().to_lowercase());
    body.push(format!("         .r#if({}.clone().is_some(), |w| w.and().eq(\"{}\", {}.unwrap()))", safe_fdname.clone(), treecol.column_name.clone().unwrap_or_default(), safe_fdname.clone()));

    if root_value == "null" {
        body.push(format!("         .r#if({}.clone().is_none(), |w| w.and().is_null(\"{}\"))", safe_fdname.clone(), treecol.column_name.clone().unwrap_or_default()));
    } else {
        body.push(format!("         .r#if({}.clone().is_none(), |w| w.and().eq(\"{}\", {}))", safe_fdname.clone(), treecol.column_name.clone().unwrap_or_default(), root_value));
    }
    //}
    // body.remove(body.len() - 1);
    let last = body.remove(body.len() - 1);
    body.push(last + ";");

    let mut savestr = String::new();
    savestr.push_str(format!("rb.fetch_list_by_wrapper::<{}>(wp).await", tbl_struct.clone()).as_str());

    body.push(savestr);
    RustFunc { 
        is_struct_fn: true, 
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true, 
        is_async: true, 
        func_name: "query_tree".to_string(), 
        return_is_option: false,
        return_is_result: true, 
        return_type: Some(format!("Vec<{}>", tbl_struct.clone())),
        params: params, 
        bodylines: body,
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(tbc.comment.clone()),
        api_method: None,
        api_pattern: None,
    }
}

