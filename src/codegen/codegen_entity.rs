use std::collections::HashMap;
use change_case::{pascal_case, snake_case};
use rbatis::rbatis::Rbatis;
use crate::codegen::{RustStructField, GenerateContext, RustStruct, RustFunc, parse_data_type_as_rust_type, parse_column_list, make_skip_columns};
use crate::config::{TableConfig, get_rbatis, safe_struct_field_name};
use crate::schema::{TableInfo, ColumnInfo};
use substring::Substring;

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


pub fn parse_table_as_struct(ctx: &GenerateContext, tbl: &TableInfo, cols: &Vec<ColumnInfo>) -> RustStruct {
    let mut columns = String::new();
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone());

    if tbc.is_none() {
        return RustStruct::default();
    }

    let tbconf = tbc.unwrap();

    let fields = parse_column_list(ctx, &tbconf, cols, &mut columns);
    if columns.ends_with(",") {
        columns = columns.substring(0, columns.len() - 1).to_string();
    }
    
    let crudtbl = format!("#[crud_table(table_name:\"{}\"|table_columns:\"{}\")]", tbl_name.clone(), columns);
    let anno = vec![crudtbl, "#[derive(Debug, Clone, Default, Deserialize, Serialize)]".to_string()];

    let mut funclist = vec![];
    let from_id = generate_func_from_pkey_for_struct(ctx, tbl);
    funclist.push(from_id);
    let save_func = generate_func_save_for_struct(ctx, tbl);
    funclist.push(save_func);
    let update_func = generate_func_update_for_struct(ctx, tbl);
    funclist.push(update_func);
    if tbconf.update_seletive {
        let update_slct_func = generate_func_update_selective_for_struct(ctx, tbl);
        funclist.push(update_slct_func);
    }

    let delete_batch_func = generate_func_delete_batch_for_struct(ctx, tbl);
    funclist.push(delete_batch_func);
    let delete_func = generate_func_delete_for_struct(ctx, tbl);
    funclist.push(delete_func);
    if tbconf.page_query {
        let page_func = generate_func_page_query_for_struct(ctx, tbl);
        funclist.push(page_func);
    }
    
    let query_list_func = generate_func_list_query_for_struct(ctx, tbl);
    funclist.push(query_list_func);

    RustStruct {
        is_pub: true,
        struct_name: match ctx.get_struct_name(&tbl_name.clone()) {
            Some(t) => t,
            None => {
                pascal_case(tbl_name.clone().as_str())
            }
        },
        annotations: anno,
        fields: fields,
        funclist: funclist,
    }
}


pub fn generate_func_from_pkey_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let _tblinfo = ctx.get_table_conf(&tbl_name.clone());
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));
    for col in pkcols.clone() {
        let dt = parse_data_type_as_rust_type(&col.data_type.unwrap_or_default());
        params.push((col.column_name.unwrap_or_default().to_lowercase(), "&".to_string() + dt.as_str()));
    }

    let mut body = vec![];
    body.push(format!("let wp = rb.new_wrapper()"));
    for col in pkcols.clone() {
        body.push(format!("    .eq(\"{}\", {})", col.column_name.clone().unwrap_or_default(), col.column_name.clone().unwrap_or_default().to_string().to_lowercase()));
        body.push(format!("    .and()"));
    }
    body.remove(body.len() - 1);
    let last = body.remove(body.len() - 1);
    body.push(last + ";");
    
    body.push("rb.fetch_by_wrapper::<Option<Self>>(wp).await".to_string());
    RustFunc { 
        is_struct_fn: true, 
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true, 
        is_async: true, 
        func_name: "from_id".to_string(), 
        return_is_option: true,
        return_is_result: true, 
        return_type: Some("Self".to_string()), 
        params: params, 
        bodylines: body,
        macros: vec![]
    }
}

/**
 * 生成Save方法
 * Save方法会自动加载last_update_id
 */
pub fn generate_func_save_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));
   
    let mut body = vec![];
    let mut savestr = String::new();
    savestr.push_str("match rb.save(self, &[");
    if tblinfo.is_some() {
        let skips = make_skip_columns(ctx, &tblinfo.clone().unwrap());
        savestr.push_str(skips.as_str());
    }
    for pk in pkcols.clone() {
        savestr.push_str(format!("Skip::Column(\"{}\"),", pk.column_name.clone().unwrap_or_default()).as_str());
    }
    if savestr.ends_with(",") {
        savestr = savestr.substring(0, savestr.len() - 1).to_string();
    }
    savestr.push_str("]).await {");
    body.push(savestr);

    body.push("Ok(ds) => {".to_string());
    // we will update the column for self
    match ctx.get_table_auto_incremnt_column(&tbl_name.clone()) {
        Some(tcl) => {
            if tblinfo.is_some() {
                let tbc = tblinfo.unwrap();
                let safe_fdname = safe_struct_field_name(&tcl.column_name.clone().unwrap_or_default().to_lowercase());
                if tbc.all_field_option {
                    body.push(format!("self.{} = ds.last_insert_id;", safe_fdname));
                } else {
                    if tcl.is_nullable.clone().unwrap_or_default().to_lowercase() == "yes" {
                        body.push(format!("self.{} = ds.last_insert_id;", safe_fdname));
                    } else {
                        body.push(format!("self.{} = ds.last_insert_id.unwrap_or_default();", safe_fdname));
                    }
                }
            } else {
                let safe_fdname = safe_struct_field_name(&tcl.column_name.clone().unwrap_or_default().to_lowercase());
                if tcl.is_nullable.clone().unwrap_or_default().to_lowercase() == "yes" {
                    body.push(format!("self.{} = ds.last_insert_id;", safe_fdname));
                } else {
                    body.push(format!("self.{} = ds.last_insert_id.unwrap_or_default();", safe_fdname));
                }
            }
        }
        None => {}
    };
    
    body.push("Ok(ds.rows_affected)".to_string());
    body.push("}".to_string());
    body.push("Err(err) => {".to_string());
    body.push("Err(err)".to_string());
    body.push("}".to_string());
    body.push("}".to_string());

    RustFunc { 
        is_struct_fn: true, 
        is_self_fn: true,
        is_self_mut: true,
        is_pub: true, 
        is_async: true, 
        func_name: "save".to_string(), 
        return_is_option: false,
        return_is_result: true, 
        return_type: Some("u64".to_string()), 
        params: params, 
        bodylines: body,
        macros: vec![]
    }
}

/**
 * 执行Update操作
 */
pub fn generate_func_update_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));
   


    let mut body = vec![];
    
    body.push(format!("let wp = rb.new_wrapper()"));
    for col in pkcols.clone() {
        body.push(format!("    .eq(\"{}\", self.{})", col.column_name.clone().unwrap_or_default(), safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_string().to_lowercase())));
        body.push(format!("    .and()"));
    }
    body.remove(body.len() - 1);
    let last = body.remove(body.len() - 1);
    body.push(last + ";");

    let mut savestr = String::new();
    savestr.push_str("rb.update_by_wrapper(self, wp, &[");
    if tblinfo.is_some() {
        let skips = make_skip_columns(ctx, &tblinfo.unwrap());
        savestr.push_str(skips.as_str());
    }

    for pk in pkcols.clone() {
        savestr.push_str(format!("Skip::Column(\"{}\"),", safe_struct_field_name(&pk.column_name.clone().unwrap_or_default())).as_str());
    }
    if savestr.ends_with(",") {
        savestr = savestr.substring(0, savestr.len() - 1).to_string();
    }

    savestr.push_str("]).await");

    body.push(savestr);
    RustFunc { 
        is_struct_fn: true, 
        is_self_fn: true,
        is_self_mut: false,
        is_pub: true, 
        is_async: true, 
        func_name: "update".to_string(), 
        return_is_option: false,
        return_is_result: true, 
        return_type: Some("u64".to_string()), 
        params: params, 
        bodylines: body,
        macros: vec![]
    }
}

/**
 * 执行Update Seletive操作
 */
pub fn generate_func_update_selective_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));
   


    let mut body = vec![];
    
    body.push(format!("let wp = rb.new_wrapper()"));
    for col in pkcols.clone() {
        body.push(format!("    .eq(\"{}\", self.{})", col.column_name.clone().unwrap_or_default(), safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_string().to_lowercase())));
        body.push(format!("    .and()"));
    }
    body.remove(body.len() - 1);
    let last = body.remove(body.len() - 1);
    body.push(last + ";");

    let mut savestr = String::new();
    savestr.push_str("rb.update_by_wrapper(self, wp, &[Skip::Value(Bson::Null)]).await");
    body.push(savestr);
    RustFunc { 
        is_struct_fn: true, 
        is_self_fn: true,
        is_self_mut: false,
        is_pub: true, 
        is_async: true, 
        func_name: "update_selective".to_string(), 
        return_is_option: false,
        return_is_result: true, 
        return_type: Some("u64".to_string()), 
        params: params, 
        bodylines: body,
        macros: vec![]
    }
}


/**
 * 执行Update Seletive操作
 */
pub fn generate_func_delete_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));

    let mut body = vec![];
    
    body.push(format!("let wp = rb.new_wrapper()"));
    for col in pkcols.clone() {
        body.push(format!("    .eq(\"{}\", self.{})", col.column_name.clone().unwrap_or_default(), safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_string().to_lowercase())));
        body.push(format!("    .and()"));
    }
    body.remove(body.len() - 1);
    let last = body.remove(body.len() - 1);
    body.push(last + ";");

    let mut savestr = String::new();
    savestr.push_str("rb.remove_by_wrapper::<Self>(wp).await");
    body.push(savestr);
    RustFunc { 
        is_struct_fn: true, 
        is_self_fn: true,
        is_self_mut: true,
        is_pub: true, 
        is_async: true, 
        func_name: "remove".to_string(), 
        return_is_option: false,
        return_is_result: true, 
        return_type: Some("u64".to_string()), 
        params: params, 
        bodylines: body,
        macros: vec![]
    }
}


/**
 * 执行Update Seletive操作
 */
pub fn generate_func_delete_batch_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut allcols = ctx.get_table_columns(&tbl_name.clone());
    
    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));
   


    let mut body = vec![];
    
    body.push(format!("let wp = rb.new_wrapper()"));
    for col in allcols.clone() {
        let safe_fdname = safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_string().to_lowercase());
        body.push(format!("         .r#if(self.{}.clone().is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
    }
    body.remove(body.len() - 1);
    let last = body.remove(body.len() - 1);
    body.push(last + ";");

    let mut savestr = String::new();
    savestr.push_str("rb.remove_by_wrapper::<Self>(wp).await");
    body.push(savestr);
    RustFunc { 
        is_struct_fn: true, 
        is_self_fn: true,
        is_self_mut: false,
        is_pub: true, 
        is_async: true, 
        func_name: "remove_batch".to_string(), 
        return_is_option: false,
        return_is_result: true, 
        return_type: Some("u64".to_string()), 
        params: params, 
        bodylines: body,
        macros: vec![]
    }
}


/**
 * 分页查询
 * 根据字段来处理
 */
pub fn generate_func_page_query_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
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
        let safe_fdname = safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_string().to_lowercase());
        body.push(format!("         .r#if(self.{}.clone().is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
    }
    body.remove(body.len() - 1);
    let last = body.remove(body.len() - 1);
    body.push(last + ";");

    let mut savestr = String::new();
    savestr.push_str("rb.fetch_page_by_wrapper::<Self>(wp, &PageRequest::new(curr, ps)).await");

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
        return_type: Some("Page<Self>".to_string()), 
        params: params, 
        bodylines: body,
        macros: vec![]
    }
}


/**
 * 分页查询
 * 根据字段来处理
 */
pub fn generate_func_list_query_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut allcols = ctx.get_table_columns(&tbl_name.clone());
    
    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));
   
    let mut body = vec![];
    
    body.push(format!("let wp = rb.new_wrapper()"));
    for col in allcols.clone() {
        let safe_fdname = safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_string().to_lowercase());
        body.push(format!("         .r#if(self.{}.clone().is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
    }
    body.remove(body.len() - 1);
    let last = body.remove(body.len() - 1);
    body.push(last + ";");

    let mut savestr = String::new();
    savestr.push_str("rb.fetch_list_by_wrapper::<Self>(wp).await");

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
        return_type: Some("Vec<Self>".to_string()), 
        params: params, 
        bodylines: body,
        macros: vec![]
    }
}
