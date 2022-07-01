use std::clone;

use change_case::{pascal_case, snake_case};
use crate::codegen::{RustStructField, GenerateContext, RustStruct, RustFunc, parse_data_type_as_rust_type, parse_column_list, make_skip_columns};
use crate::config::{TableConfig, get_rbatis, safe_struct_field_name, SimpleFuncation};
use crate::schema::{TableInfo, ColumnInfo};
use substring::Substring;


pub fn parse_table_as_struct(ctx: &GenerateContext, tbl: &TableInfo, cols: &Vec<ColumnInfo>) -> RustStruct {
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

    let fields = parse_column_list(ctx, &tbconf, cols, &mut columns, false);
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


    if pkcols.len() == 1 {
        let delete_ids = generate_func_delete_ids_for_struct(ctx, tbl);
        funclist.push(delete_ids);
    }

    if tbconf.page_query {
        let page_func = generate_func_page_query_for_struct(ctx, tbl);
        funclist.push(page_func);
    }
    
    let query_list_func = generate_func_list_query_for_struct(ctx, tbl);
    funclist.push(query_list_func);

    let query_all_func = generate_func_all_query_for_struct(ctx, tbl);
    funclist.push(query_all_func);

    for smpfun in tbconf.simple_funclist.clone() {
        let simplefunc = generate_func_simple_func_for_struct(ctx, tbl, &smpfun);
        funclist.push(simplefunc);
    }

    if tbconf.tree_parent_field.is_some() {
        let treefunc = generate_func_tree_query_for_struct(ctx, tbl);
        funclist.push(treefunc);
    }

    RustStruct {
        is_pub: true,
        has_paging: tbconf.page_query,
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
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
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
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(tbconf.comment.clone()),
        api_method: None,
        api_pattern: None,
    }
}

/**
 * 生成Save方法
 * Save方法会自动加载last_update_id
 */
pub fn generate_func_save_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.clone().unwrap();
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
    // When the record is saving, the PK should not be skip, but the auto_increment key should be skip
    let autokey = ctx.get_table_auto_incremnt_column(&tbl_name.clone());
    if autokey.is_some() {
        let autokeycol = autokey.unwrap();
        savestr.push_str(format!("Skip::Column(\"{}\"),", autokeycol.column_name.clone().unwrap_or_default()).as_str());
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
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(tbconf.comment.clone()),
        api_method: None,
        api_pattern: None,        
    }
}

/**
 * 执行Update操作
 */
pub fn generate_func_update_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.clone().unwrap();
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
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(tbconf.comment.clone()),
        api_method: None,
        api_pattern: None,
    }
}

/**
 * 执行Update Seletive操作
 */
pub fn generate_func_update_selective_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
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
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(tbconf.comment.clone()),
        api_method: None,
        api_pattern: None,
    }
}


/**
 * 执行Update Seletive操作
 */
pub fn generate_func_delete_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
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
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(tbconf.comment.clone()),
        api_method: None,
        api_pattern: None,        
    }
}


/**
 * 执行Delete More操作
 */
pub fn generate_func_delete_ids_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));

    for col in pkcols.clone() {
        params.push(("ids".to_string(), format!("&[{}]", parse_data_type_as_rust_type(&col.data_type.clone().unwrap_or_default().to_lowercase()))));
    }

    let mut body = vec![];
    
    body.push(format!("let wp = rb.new_wrapper()"));
    for col in pkcols.clone() {
        body.push(format!("    .r#in(\"{}\", ids)", col.column_name.clone().unwrap_or_default()));
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
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true, 
        is_async: true, 
        func_name: "remove_ids".to_string(), 
        return_is_option: false,
        return_is_result: true, 
        return_type: Some("u64".to_string()), 
        params: params, 
        bodylines: body,
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(tbconf.comment.clone()),
        api_method: None,
        api_pattern: None,        
    }
}


/**
 * 执行Update Seletive操作
 */
pub fn generate_func_delete_batch_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
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
    // body.remove(body.len() - 1);
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
pub fn generate_func_page_query_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
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
        if tbconf.tree_parent_field.clone().unwrap_or_default().to_lowercase() == safe_fdname.clone() {
            body.push(format!("         .r#if(self.{}.clone().is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
            if tbconf.tree_root_value.clone().unwrap_or_default().to_lowercase() == "null".to_string() {
                body.push(format!("         .r#if(self.{}.clone().is_none(), |w| w.and().is_null(\"{}\"))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default()));
            } else {
                body.push(format!("         .r#if(self.{}.clone().is_none(), |w| w.and().eq(\"{}\", Some({})))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), tbconf.tree_root_value.clone().unwrap_or_default().to_lowercase()));
            }
        } else {
            body.push(format!("         .r#if(self.{}.clone().is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
        }
    }
    // body.remove(body.len() - 1);
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
pub fn generate_func_list_query_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut allcols = ctx.get_table_columns(&tbl_name.clone());
    
    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));
   
    let mut body = vec![];
    
    body.push(format!("let wp = rb.new_wrapper()"));
    for col in allcols.clone() {
        let safe_fdname = safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_string().to_lowercase());
        if tbconf.tree_parent_field.clone().unwrap_or_default().to_lowercase() == safe_fdname.clone() {
            body.push(format!("         .r#if(self.{}.clone().is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
            if tbconf.tree_root_value.clone().unwrap_or_default().to_lowercase() == "null".to_string() {
                body.push(format!("         .r#if(self.{}.clone().is_none(), |w| w.and().is_null(\"{}\"))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default()));
            } else {
                body.push(format!("         .r#if(self.{}.clone().is_none(), |w| w.and().eq(\"{}\", Some({})))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), tbconf.tree_root_value.clone().unwrap_or_default().to_lowercase()));
            }
        } else {
            body.push(format!("         .r#if(self.{}.clone().is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
        }
        
    }
    // body.remove(body.len() - 1);
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
pub fn generate_func_all_query_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut allcols = ctx.get_table_columns(&tbl_name.clone());
    
    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));
   
    let mut body = vec![];
    
    body.push(format!("let wp = rb.new_wrapper();"));
    let mut savestr = String::new();
    savestr.push_str("rb.fetch_list_by_wrapper::<Self>(wp).await");

    body.push(savestr);
    RustFunc { 
        is_struct_fn: true, 
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true, 
        is_async: true, 
        func_name: "query_all".to_string(), 
        return_is_option: false,
        return_is_result: true, 
        return_type: Some("Vec<Self>".to_string()), 
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
pub fn generate_func_tree_query_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbc = tblinfo.unwrap();
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
    savestr.push_str("rb.fetch_list_by_wrapper::<Self>(wp).await");

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
        return_type: Some("Vec<Self>".to_string()), 
        params: params, 
        bodylines: body,
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(tbc.comment.clone()),
        api_method: None,
        api_pattern: None,
    }
}


/**
 * 生成SimpleFunction
 * 根据字段来处理
 */
pub fn generate_func_simple_func_for_struct(ctx: &GenerateContext, tbl: &TableInfo, simplefun: &SimpleFuncation) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut allcols = ctx.get_table_columns(&tbl_name.clone());
    
    let mut params = Vec::new();

    let sp = simplefun.condition.split(",");
    let mut condcols = vec![];
    for row in sp.into_iter() {
        for lc in allcols.clone() {
            if lc.column_name == Some(row.to_string()) {
                condcols.push(lc.clone());
                break;
            }
        }
    }

    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));

    if !simplefun.is_self {
        // add the params
        for col in condcols.clone() {
            let safe_fdname = safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_string().to_lowercase());
            let dt = parse_data_type_as_rust_type(&col.data_type.clone().unwrap_or_default().to_lowercase());
            params.push((safe_fdname, format!("&{}", dt)));
        }
    }

    if simplefun.is_paged {
        params.push(("curr".to_string(), "&u64".to_string()));
        params.push(("size".to_string(), "&u64".to_string()));
    }
   
    let mut body = vec![];
    
    body.push(format!("let wp = rb.new_wrapper()"));
    for col in condcols.clone() {
        let safe_fdname = safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_string().to_lowercase());
        if simplefun.is_self {
            body.push(format!("         .and().eq(\"{}\", self.{}.clone().unwrap())", col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
        } else {
            body.push(format!("         .and().eq(\"{}\", {}.clone())", col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
        }
    }
    // body.remove(body.len() - 1);
    let last = body.remove(body.len() - 1);
    body.push(last + ";");

    let mut savestr = String::new();
    if simplefun.is_paged {
        savestr.push_str("rb.fetch_page_by_wrapper::<Self>(wp, &PageRequest::new(curr, ps)).await");
    } else if simplefun.is_list {
        savestr.push_str("rb.fetch_list_by_wrapper::<Self>(wp).await");
    } else {
        savestr.push_str("rb.fetch_by_wrapper::<Option<Self>>(wp).await");
    }
    

    body.push(savestr);
    RustFunc { 
        is_struct_fn: true, 
        is_self_fn: simplefun.is_self,
        is_self_mut: false,
        is_pub: true, 
        is_async: true, 
        func_name: simplefun.fun_name.clone(), 
        return_is_option: !simplefun.is_list && !simplefun.is_paged,
        return_is_result: true, 
        return_type: if simplefun.is_paged {
            Some("Page<Self>".to_string())
        } else if simplefun.is_list {
            Some("Vec<Self>".to_string())
        } else {
            Some("Self".to_string())
        },
        params: params, 
        bodylines: body,
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(tbconf.comment.clone()),
        api_method: None,
        api_pattern: None,
    }
}




pub fn parse_table_as_value_object_struct(ctx: &GenerateContext, tbl: &TableInfo, cols: &Vec<ColumnInfo>) -> RustStruct {
    let mut columns = String::new();
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone());

    if tbc.is_none() {
        return RustStruct::default();
    }

    let tbconf = tbc.unwrap();

    let valobjstruct_name = match ctx.get_value_object_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => {
            format!("{}Value", pascal_case(tbl_name.clone().as_str()))
        }
    };

    let mut fields = parse_column_list(ctx, &tbconf, cols, &mut columns, true);
    
    let mut annts = vec![];
    
    if ctx.codegen_conf.allow_bool_widecard {
        annts.push("#[serde(default)]".to_string());
        annts.push("#[serde(deserialize_with=\"bool_from_str\")]".to_string());
    }
    

    if tbconf.tree_parent_field.is_some() {
        let has_children = RustStructField {
            is_pub: true,
            column_name: "has_children".to_string(),
            field_name: "has_children".to_string(),
            field_type: "bool".to_string(),
            orignal_field_name: None,
            is_option: false,
            annotations: vec![],
        };

        let leaf = RustStructField {
            is_pub: true,
            column_name: "leaf".to_string(),
            field_name: "leaf".to_string(),
            field_type: "bool".to_string(),
            is_option: false,
            orignal_field_name: None,
            annotations: vec![],
        };

        let label = RustStructField {
            is_pub: true,
            column_name: "label".to_string(),
            field_name: "label".to_string(),
            field_type: "String".to_string(),
            is_option: true,
            orignal_field_name: None,
            annotations: vec![],
        };        

        let children = RustStructField {
            is_pub: true,
            column_name: "children".to_string(),
            field_name: "children".to_string(),
            field_type: format!("Vec<{}>", valobjstruct_name),
            is_option: false,
            orignal_field_name: None,
            annotations: vec!["#[serde(default)]".to_string()]
        };

        fields.push(leaf);
        fields.push(label);
        fields.push(has_children);
        fields.push(children);
    }

    

    
    let anno = vec!["#[derive(Debug, Clone, Default, Deserialize, Serialize)]".to_string()];
    let mut funclist = vec![];

    let fromfunc = generate_func_value_object_from_entity(ctx, tbl, true);
    let compxfromfunc = generate_func_value_object_from_entity(ctx, tbl, false);
    let tofunc = generate_func_value_object_to_entity(ctx, tbl);
    let btreefunc = generate_func_build_tree_for_value(ctx, tbl);
    let btreefunc_rec = generate_fun_build_tree_rec_for_value(ctx, tbl);

    funclist.push(fromfunc);
    funclist.push(compxfromfunc);
    funclist.push(tofunc);
    funclist.push(btreefunc_rec);
    funclist.push(btreefunc);

    RustStruct {
        is_pub: true,
        has_paging: tbconf.page_query,
        struct_name: valobjstruct_name.clone(),
        annotations: anno,
        fields: fields,
        funclist: funclist,
    }
}


fn guess_label_field(cols: &Vec<ColumnInfo>) -> String {
    let mut first_string = None;
    let mut lable_field = None;
    for col in cols.clone() {
        let dt = parse_data_type_as_rust_type(&col.data_type.clone().unwrap_or_default());
        let fd_name = safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_lowercase());
        if dt == "String" && first_string.is_none() {
            first_string = Some(fd_name.clone());
        }

        if fd_name == "label" {
            return fd_name
        } else if fd_name == "title" {
            lable_field = Some(fd_name);
            break;
        } else if fd_name == "name" {
            if lable_field.is_none() {
                lable_field = Some(fd_name);
            }
        } else if fd_name == "caption" {
            if lable_field.is_none() || lable_field == Some("name".to_string()) {
                lable_field = Some(fd_name);
            }
        }
    }
    if lable_field.is_none() {
        if first_string.is_some() {
            first_string.unwrap()
        } else {
            if cols.is_empty() {
                return String::new()
            } else {
                let fd_name = safe_struct_field_name(&cols[0].column_name.clone().unwrap_or_default().to_lowercase());
                return fd_name
            }
        }
    } else {
        lable_field.unwrap()
    }
}

fn generate_func_value_object_to_entity(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap();
    let tbc = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tbc.unwrap();
    let mut body = vec![];

    body.push(format!("{} {{", tbconf.struct_name.clone()));
    
    let mut columns = String::new();
    let cols = ctx.get_table_columns(&tbl_name.clone());
    let parsed_fields = parse_column_list(ctx, &tbconf, &cols, &mut columns, true);
    for fd in parsed_fields {
        let fname = fd.field_name.clone();
        if fd.orignal_field_name.is_none() {
            body.push(format!("{}: self.{}.clone(),", safe_struct_field_name(&fname), safe_struct_field_name(&fname)));
        } else {
            body.push(format!("{}: self.{}.clone(),", safe_struct_field_name(&fd.orignal_field_name.clone().unwrap_or_default()), safe_struct_field_name(&fname)));
        }
    }
    
    body.push(format!("}}"));

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    // params.push(("param".to_string(), "&".to_owned() + tbconf.struct_name.clone().as_str()));

    RustFunc { 
        is_struct_fn: true, 
        is_self_fn: true, 
        is_self_mut: false, 
        is_pub: true, 
        is_async: false, 
        func_name: format!("to_entity"), 
        return_is_option: false, 
        return_is_result: false, 
        return_type: Some(tbconf.struct_name.clone()), 
        params: params,
        bodylines: body, 
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(tbconf.comment.clone()),
        api_method: None,
        api_pattern: None,
    }
}


fn generate_func_value_object_from_entity(ctx: &GenerateContext, tbl: &TableInfo, simple: bool) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap();
    let tbc = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tbc.unwrap();
    let mut body = vec![];

    body.push(format!("Self {{"));
    
    let mut columns = String::new();
    let cols = ctx.get_table_columns(&tbl_name.clone());
    let parsed_fields = parse_column_list(ctx, &tbconf, &cols, &mut columns, true);
    for fd in parsed_fields {
        let fname = fd.field_name.clone();
        if fd.orignal_field_name.is_none() {
            body.push(format!("{}: param.{}.clone(),", safe_struct_field_name(&fname), safe_struct_field_name(&fname)));
        } else {
            body.push(format!("{}: param.{}.clone(),", safe_struct_field_name(&fname), safe_struct_field_name(&fd.orignal_field_name.clone().unwrap_or_default())));
        }
    }
    if simple {
        body.push(format!("has_children: false,"));
        body.push("leaf: false,".to_string());
        body.push(format!("children: vec![],"));
    } else {
        body.push(format!("has_children: haschild,"));
        body.push("leaf: haschild == false,".to_string());
        body.push(format!("children: children.clone(),"));
    }
    body.push(format!("label: param.{}.clone(),", guess_label_field(&cols)));

    body.push(format!("}}"));

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("param".to_string(), "&".to_owned() + tbconf.struct_name.clone().as_str()));

    if !simple {
        params.push(("haschild".to_string(), "bool".to_string()));
        params.push(("children".to_string(), format!("&Vec<Self>")));
    }
    RustFunc { 
        is_struct_fn: true, 
        is_self_fn: false, 
        is_self_mut: false, 
        is_pub: true, 
        is_async: false, 
        func_name: if simple {
            format!("from_entity")
        } else {
            format!("from_entity_with")
        },
        return_is_option: false, 
        return_is_result: false, 
        return_type: Some("Self".to_string()), 
        params: params,
        bodylines: body, 
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(tbconf.comment.clone()),
        api_method: None,
        api_pattern: None,
    }
}


fn generate_func_build_tree_for_value(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap();
    let tbc = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tbc.unwrap();
    let mut body = vec![];
    let mut params = vec![];
    params.push(("items".to_string(), "&Vec<Self>".to_string()));

    body.push(format!("let mut tmptree = vec![];"));
    body.push(format!("for xip in items.clone() {{"));
    body.push(format!("if xip.pid.is_none() || xip.pid == Some(0) {{"));
    body.push(format!("tmptree.push(xip.clone());"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("let mut tree = vec![];"));
    body.push(format!("for mut it in tmptree {{"));
    body.push(format!("Self::recurse_build_tree(items, &mut it);"));
    body.push(format!("tree.push(it);"));
    body.push(format!("}}"));
    body.push(format!("tree"));
    RustFunc { 
        is_struct_fn: true, 
        is_self_fn: false, 
        is_self_mut: false, 
        is_pub: true, 
        is_async: false, 
        func_name: format!("build_tree"),
        return_is_option: false, 
        return_is_result: false, 
        return_type: Some("Vec<Self>".to_string()), 
        params: params,
        bodylines: body, 
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(tbconf.comment.clone()),
        api_method: None,
        api_pattern: None,
    }
}

fn generate_fun_build_tree_rec_for_value(ctx: &GenerateContext, tbl: &TableInfo) ->RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap();
    let tbc = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tbc.unwrap();
    let mut body = vec![];
    let mut params = vec![];
    params.push(("items".to_string(), "&Vec<Self>".to_string()));
    params.push(("parent_item".to_string(), "&mut Self".to_string()));

    body.push(format!("for xip in items.clone() {{"));
    body.push(format!("if xip.pid == parent_item.id {{"));
    body.push(format!("let mut mip = xip;"));
    body.push(format!("Self::recurse_build_tree(items, &mut mip);"));
    body.push(format!("if mip.children.is_empty() {{"));
    body.push(format!("mip.leaf = true;"));
    body.push(format!("mip.has_children = false;"));
    body.push(format!("}}"));
    body.push(format!("parent_item.children.push(mip);"));
    body.push(format!("}}"));
    body.push(format!("}}"));

    RustFunc { 
        is_struct_fn: true, 
        is_self_fn: false, 
        is_self_mut: false, 
        is_pub: false, 
        is_async: false, 
        func_name: format!("recurse_build_tree"),
        return_is_option: false, 
        return_is_result: false, 
        return_type: None, 
        params: params,
        bodylines: body,
        macros: vec![],
        comment: Some(tbconf.comment.clone()),
        api_method: None,
        api_pattern: None,
    }

}