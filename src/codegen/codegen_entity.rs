use crate::codegen::{
    make_skip_columns, parse_column_list, parse_data_type_as_rust_type, GenerateContext, RustFunc,
    RustStruct, RustStructField,
};
use crate::config::{
    safe_struct_field_name, RelationConfig, Relationship, SimpleFuncation, TableConfig,
};
use crate::schema::{ColumnInfo, TableInfo};
use change_case::pascal_case;
use substring::Substring;

use super::{is_copied_data_type, parse_column_as_field, RustStructFieldExtend};

pub fn parse_table_as_struct(
    ctx: &GenerateContext,
    tbl: &TableInfo,
    cols: &Vec<ColumnInfo>,
) -> RustStruct {
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

    let mut usings = vec![];
    let mut fields = parse_column_list(ctx, &tbconf, cols, &mut columns, false, &mut usings);
    if columns.ends_with(",") {
        columns = columns.substring(0, columns.len() - 1).to_string();
    }

    if tbconf.with_attachment {
        fields.push(RustStructField {
            is_pub: true,
            schema_name: None,
            column_name: "".to_string(),
            field_name: "attachments".to_string(),
            orignal_field_name: None,
            comment: None,
            field_type: "Vec<ChimesAttachmentInfo>".to_string(),
            is_option: false,
            length: 0,
            annotations: vec!["#[serde(default)]".to_string()],
        })
    }

    let crudtbl = format!(
        "#[crud_table(table_name:\"{}\"|table_columns:\"{}\")]",
        tbl_name.clone(),
        columns
    );
    let anno = vec![
        crudtbl,
        "#[derive(Debug, Clone, Default, Deserialize, Serialize)]".to_string(),
    ];

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
        let delete_not_ids = generate_func_delete_not_ids_for_struct(ctx, tbl);
        funclist.push(delete_not_ids);
        let load_ids = generate_func_load_ids_for_struct(ctx, tbl);
        funclist.push(load_ids);
    }

    let rels = ctx.get_relation_config_in_relationship(&tbl.table_name.clone().unwrap_or_default());
    for rel in rels.clone() {
        let relfunc = generate_func_delete_rel_ids_for_struct(ctx, tbl, &rel);
        if relfunc.is_some() {
            funclist.push(relfunc.unwrap());
        }
    }

    if tbconf.page_query {
        let page_func = generate_func_page_query_for_struct(ctx, tbl);
        funclist.push(page_func);
        let common_page_func = generate_func_common_page_query_for_struct(ctx, tbl);
        funclist.push(common_page_func);
    }

    let query_list_func = generate_func_list_query_for_struct(ctx, tbl);
    funclist.push(query_list_func);

    let common_query_list_func = generate_func_common_list_query_for_struct(ctx, tbl);
    funclist.push(common_query_list_func);

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

    if tbconf.with_attachment {
        let load_attachfun = generate_func_load_attachment(ctx, tbl);
        funclist.push(load_attachfun);
        let save_attachfun = generate_func_save_attachment(ctx, tbl);
        funclist.push(save_attachfun);
        let remove_attachfun = generate_func_remove_attachment(ctx, tbl);
        funclist.push(remove_attachfun);
        let remove_attachsfun = generate_func_remove_attachments(ctx, tbl);
        funclist.push(remove_attachsfun);
    }

    RustStruct {
        is_pub: true,
        has_paging: tbconf.page_query,
        struct_name: match ctx.get_struct_name(&tbl_name.clone()) {
            Some(t) => t,
            None => pascal_case(tbl_name.clone().as_str()),
        },
        annotations: anno,
        fields: fields,
        funclist: funclist,
        usings
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
        params.push((
            col.column_name.unwrap_or_default().to_lowercase(),
            "&".to_string() + dt.as_str(),
        ));
    }

    let mut body = vec![];
    body.push(format!("let wp = rb.new_wrapper()"));
    for col in pkcols.clone() {
        body.push(format!(
            "    .eq(\"{}\", {})",
            col.column_name.clone().unwrap_or_default(),
            col.column_name
                .clone()
                .unwrap_or_default()
                .to_string()
                .to_lowercase()
        ));
        body.push(format!("    .and()"));
    }
    body.remove(body.len() - 1);
    let last = body.remove(body.len() - 1);
    body.push(last + ";");
    if tbconf.with_attachment {
        body.push(format!(
            "match rb.fetch_by_wrapper::<Option<Self>>(wp).await {{"
        ));
        body.push(format!("Ok(mt) => {{"));
        body.push(format!("if mt.is_some() {{"));
        body.push(format!("let mut nt = mt.unwrap();"));
        body.push(format!("match nt.load_attachment(rb).await {{"));
        body.push(format!("Ok(_) => {{"));
        body.push(format!("Ok(Some(nt))"));
        body.push(format!("}}"));
        body.push(format!("Err(err) => {{"));
        body.push(format!("Err(err)"));
        body.push(format!("}}"));
        body.push(format!("}}"));
        body.push(format!("}} else {{"));
        body.push(format!("Ok(mt)"));
        body.push(format!("}}"));
        body.push(format!("}}"));
        body.push(format!("Err(err) => {{"));
        body.push(format!("Err(err)"));
        body.push(format!("}}"));
        body.push(format!("}}"));
    } else {
        body.push("rb.fetch_by_wrapper::<Option<Self>>(wp).await".to_string());
    }

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
    params.push(("rb".to_string(), "&mut RBatisTxExecutor<'_>".to_string()));

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
        savestr.push_str(
            format!(
                "Skip::Column(\"{}\"),",
                autokeycol.column_name.clone().unwrap_or_default()
            )
            .as_str(),
        );
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
                let safe_fdname = safe_struct_field_name(
                    &tcl.column_name.clone().unwrap_or_default().to_lowercase(),
                );
                if tbc.all_field_option {
                    body.push(format!("self.{} = ds.last_insert_id;", safe_fdname));
                } else {
                    if tcl.is_nullable.clone().unwrap_or_default().to_lowercase() == "yes" {
                        body.push(format!("self.{} = ds.last_insert_id;", safe_fdname));
                    } else {
                        body.push(format!(
                            "self.{} = ds.last_insert_id.unwrap_or_default();",
                            safe_fdname
                        ));
                    }
                }
            } else {
                let safe_fdname = safe_struct_field_name(
                    &tcl.column_name.clone().unwrap_or_default().to_lowercase(),
                );
                if tcl.is_nullable.clone().unwrap_or_default().to_lowercase() == "yes" {
                    body.push(format!("self.{} = ds.last_insert_id;", safe_fdname));
                } else {
                    body.push(format!(
                        "self.{} = ds.last_insert_id.unwrap_or_default();",
                        safe_fdname
                    ));
                }
            }
        }
        None => {}
    };

    if tbconf.with_attachment {
        body.push(format!("match self.save_attachment(rb).await {{"));
        body.push(format!("Ok(_) => {{}},"));
        body.push(format!("Err(_) => {{}}"));
        body.push(format!("}}"));
    }
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

    if ctx.codegen_conf.multi_tenancy {
        let allcols = ctx.get_table_columns(&tbl_name.clone());
        for cl in allcols.clone() {
            if cl.column_name == Some("company_id".to_string()) {
                pkcols.push(cl.clone());
            }
            if cl.column_name == Some("company_code".to_string()) {
                pkcols.push(cl.clone());
            }
        }
    }

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&mut RBatisTxExecutor<'_>".to_string()));

    let mut body = vec![];

    body.push(format!("let wp = rb.get_rbatis().new_wrapper()"));
    for col in pkcols.clone() {
        if  is_copied_data_type(&col.data_type.clone().unwrap_or_default()) {
            body.push(format!(
                "    .eq(\"{}\", self.{})",
                col.column_name.clone().unwrap_or_default(),
                safe_struct_field_name(
                    &col.column_name
                        .clone()
                        .unwrap_or_default()
                        .to_string()
                        .to_lowercase()
                )
            ));
        } else {
            body.push(format!(
                "    .eq(\"{}\", self.{}.clone())",
                col.column_name.clone().unwrap_or_default(),
                safe_struct_field_name(
                    &col.column_name
                        .clone()
                        .unwrap_or_default()
                        .to_string()
                        .to_lowercase()
                )
            ));
        }
    }
    let last = body.remove(body.len() - 1);
    body.push(last + ";");

    let mut savestr = String::new();

    if tbconf.with_attachment {
        body.push(format!("match self.save_attachment(rb).await {{"));
        body.push(format!("Ok(_) => {{}},"));
        body.push(format!("Err(_) => {{}}"));
        body.push(format!("}}"));
    }

    savestr.push_str("rb.update_by_wrapper(self, wp, &[");
    if tblinfo.is_some() {
        let skips = make_skip_columns(ctx, &tblinfo.unwrap());
        savestr.push_str(skips.as_str());
    }

    for pk in pkcols.clone() {
        savestr.push_str(
            format!(
                "Skip::Column(\"{}\"),",
                safe_struct_field_name(&pk.column_name.clone().unwrap_or_default())
            )
            .as_str(),
        );
    }

    if savestr.ends_with(",") {
        savestr = savestr.substring(0, savestr.len() - 1).to_string();
    }

    savestr.push_str("]).await");

    body.push(savestr);
    RustFunc {
        is_struct_fn: true,
        is_self_fn: true,
        is_self_mut: true,
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
pub fn generate_func_update_selective_for_struct(
    ctx: &GenerateContext,
    tbl: &TableInfo,
) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }
    let mut skipstr = String::new();
    if ctx.codegen_conf.multi_tenancy {
        let allcols = ctx.get_table_columns(&tbl_name.clone());
        for cl in allcols.clone() {
            if cl.column_name == Some("company_id".to_string()) {
                pkcols.push(cl.clone());
                skipstr.push_str(
                    format!(
                        ", Skip::Column(\"{}\"),",
                        safe_struct_field_name(&cl.column_name.clone().unwrap_or_default())
                    )
                    .as_str(),
                );
            }
            if cl.column_name == Some("company_code".to_string()) {
                pkcols.push(cl.clone());
                skipstr.push_str(
                    format!(
                        ", Skip::Column(\"{}\"),",
                        safe_struct_field_name(&cl.column_name.clone().unwrap_or_default())
                    )
                    .as_str(),
                );
            }
        }
    }

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&mut RBatisTxExecutor<'_>".to_string()));

    let mut body = vec![];

    body.push(format!("let wp = rb.get_rbatis().new_wrapper()"));
    for col in pkcols.clone() {
        if is_copied_data_type(&col.data_type.clone().unwrap_or_default()) {
            body.push(format!(
                "    .eq(\"{}\", self.{})",
                col.column_name.clone().unwrap_or_default(),
                safe_struct_field_name(
                    &col.column_name
                        .clone()
                        .unwrap_or_default()
                        .to_string()
                        .to_lowercase()
                )
            ));
        } else {
            body.push(format!(
                "    .eq(\"{}\", self.{}.clone())",
                col.column_name.clone().unwrap_or_default(),
                safe_struct_field_name(
                    &col.column_name
                        .clone()
                        .unwrap_or_default()
                        .to_string()
                        .to_lowercase()
                )
            ));
        }
    }

    let last = body.remove(body.len() - 1);
    body.push(last + ";");

    if tbconf.with_attachment {
        body.push(format!("match self.save_attachment(rb).await {{"));
        body.push(format!("Ok(_) => {{}},"));
        body.push(format!("Err(_) => {{}}"));
        body.push(format!("}}"));
    }

    let mut savestr = String::new();
    savestr.push_str(
        format!(
            "rb.update_by_wrapper(self, wp, &[Skip::Value(Bson::Null){}]).await",
            skipstr
        )
        .as_str(),
    );
    body.push(savestr);
    RustFunc {
        is_struct_fn: true,
        is_self_fn: true,
        is_self_mut: true,
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

    if ctx.codegen_conf.multi_tenancy {
        let allcols = ctx.get_table_columns(&tbl_name.clone());
        for cl in allcols.clone() {
            if cl.column_name == Some("company_id".to_string()) {
                pkcols.push(cl.clone());
            }
            if cl.column_name == Some("company_code".to_string()) {
                pkcols.push(cl.clone());
            }
        }
    }

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&mut RBatisTxExecutor<'_>".to_string()));

    let mut body = vec![];

    body.push(format!("let wp = rb.get_rbatis().new_wrapper()"));
    for col in pkcols.clone() {
        if is_copied_data_type(&col.data_type.clone().unwrap_or_default()) {
            body.push(format!(
                "    .eq(\"{}\", self.{})",
                col.column_name.clone().unwrap_or_default(),
                safe_struct_field_name(
                    &col.column_name
                        .clone()
                        .unwrap_or_default()
                        .to_string()
                        .to_lowercase()
                )
            ));
        } else {
            body.push(format!(
                "    .eq(\"{}\", self.{}.clone())",
                col.column_name.clone().unwrap_or_default(),
                safe_struct_field_name(
                    &col.column_name
                        .clone()
                        .unwrap_or_default()
                        .to_string()
                        .to_lowercase()
                )
            ));
        }
    }
    // body.remove(body.len() - 1);
    let last = body.remove(body.len() - 1);
    body.push(last + ";");

    if tbconf.with_attachment {
        body.push(format!("match self.remove_attachments(rb).await {{"));
        body.push(format!("Ok(_) => {{}},"));
        body.push(format!("Err(_) => {{}}"));
        body.push(format!("}}"));
    }

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
 * 执行Load by More Ids操作
 */
pub fn generate_func_load_ids_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    let struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));

    let mut body = vec![];
    let mut has_cond = false;

    body.push(format!("let wp = rb.new_wrapper()"));
    for col in pkcols.clone() {
        body.push(format!(
            "    .r#in(\"{}\", ids)",
            col.column_name.clone().unwrap_or_default()
        ));
    }
    if ctx.codegen_conf.multi_tenancy {
        let allcols = ctx.get_table_columns(&tbl_name.clone());
        for col in allcols.clone() {
            if col.column_name == Some("company_id".to_string())
                || col.column_name == Some("company_code".to_string())
            {
                body.push(format!(
                    "    .eq(\"{}\", cond.{}.clone())",
                    col.column_name.clone().unwrap_or_default(),
                    safe_struct_field_name(
                        &col.column_name
                            .clone()
                            .unwrap_or_default()
                            .to_string()
                            .to_lowercase()
                    )
                ));
                has_cond = true;
            }
        }
    }

    let last = body.remove(body.len() - 1);
    body.push(last + ";");

    for col in pkcols.clone() {
        params.push((
            "ids".to_string(),
            format!(
                "&[{}]",
                parse_data_type_as_rust_type(
                    &col.data_type.clone().unwrap_or_default().to_lowercase()
                )
            ),
        ));
    }

    if ctx.codegen_conf.multi_tenancy {
        if has_cond {
            params.push(("cond".to_string(), format!("&{}", struct_name.clone())));
        } else {
            params.push(("_cond".to_string(), format!("&{}", struct_name.clone())));
        }
    }

    let mut savestr = String::new();
    savestr.push_str("rb.fetch_list_by_wrapper::<Self>(wp).await");

    body.push(savestr);
    RustFunc {
        is_struct_fn: true,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: "load_ids".to_string(),
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
 * 执行Delete Not More操作
 */
pub fn generate_func_delete_not_ids_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    let struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let allcols = ctx.get_table_columns(&tbl_name.clone());

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&mut RBatisTxExecutor<'_>".to_string()));

    //}

    let mut body = vec![];

    body.push(format!("let wp = rb.get_rbatis().new_wrapper()"));

    let fkeys = ctx.get_relation_table_freginkeys(&tbl_name);

    let mut has_cond = false;
    for fcol in fkeys {
        log::info!("Fcol: {}", fcol.clone());
        let col = allcols.clone().into_iter().find(|f| f.column_name == Some(fcol.clone()));
        if col.is_some() && is_copied_data_type(&col.unwrap().data_type.unwrap_or_default()) {
            body.push(format!(
                "    .r#eq(\"{}\", cond.{})",
                fcol.clone(),
                fcol.clone()
            ));
        } else {
            body.push(format!(
                "    .r#eq(\"{}\", cond.{}.clone())",
                fcol.clone(),
                fcol.clone()
            ));
        }
        has_cond = true;
    }

    for col in pkcols.clone() {
        body.push(format!(
            "    .r#not_in(\"{}\", ids)",
            col.column_name.clone().unwrap_or_default()
        ));
    }
    if ctx.codegen_conf.multi_tenancy {
        let allcols = ctx.get_table_columns(&tbl_name.clone());
        for col in allcols.clone() {
            if col.column_name == Some("company_id".to_string())
                || col.column_name == Some("company_code".to_string())
            {
                body.push(format!(
                    "    .eq(\"{}\", cond.{}.clone())",
                    col.column_name.clone().unwrap_or_default(),
                    safe_struct_field_name(
                        &col.column_name
                            .clone()
                            .unwrap_or_default()
                            .to_string()
                            .to_lowercase()
                    )
                ));
                has_cond = true;
            }
        }
    }

    for col in pkcols.clone() {
        params.push((
            "ids".to_string(),
            format!(
                "&[{}]",
                parse_data_type_as_rust_type(
                    &col.data_type.clone().unwrap_or_default().to_lowercase()
                )
            ),
        ));
    }

    if has_cond {
        params.push(("cond".to_string(), format!("&{}", struct_name.clone())));
    } else {
        params.push(("_cond".to_string(), format!("&{}", struct_name.clone())));
    }

    let last = body.remove(body.len() - 1);
    body.push(last + ";");
    if tbconf.with_attachment {
        let mut savestr = String::new();
        savestr.push_str("match rb.fetch_list_by_wrapper::<Self>(wp.clone()).await {");
        body.push(savestr);
        body.push(format!("Ok(fss) => {{"));
        body.push(format!("for mut xss in fss.clone() {{"));
        body.push(format!("match xss.remove_attachments(rb).await {{"));
        body.push(format!("Ok(_) => {{}}"));
        body.push(format!("Err(_) => {{}}"));
        body.push(format!("}}"));
        body.push(format!("}}"));
        body.push(format!("}}"));
        body.push(format!("Err(_) => {{"));
        body.push(format!("}}"));
        body.push(format!("}}"));

        let mut savestr = String::new();
        savestr.push_str("rb.remove_by_wrapper::<Self>(wp).await");
        body.push(savestr);
    } else {
        let mut savestr = String::new();
        savestr.push_str("rb.remove_by_wrapper::<Self>(wp).await");
        body.push(savestr);
    }
    RustFunc {
        is_struct_fn: true,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: "remove_not_ids".to_string(),
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
    let struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&mut RBatisTxExecutor<'_>".to_string()));

    let mut body = vec![];

    body.push(format!("let wp = rb.get_rbatis().new_wrapper()"));
    for col in pkcols.clone() {
        body.push(format!(
            "    .r#in(\"{}\", ids)",
            col.column_name.clone().unwrap_or_default()
        ));
    }

    let mut has_cond = false;
    if ctx.codegen_conf.multi_tenancy {
        let allcols = ctx.get_table_columns(&tbl_name.clone());
        for col in allcols.clone() {
            if col.column_name == Some("company_id".to_string())
                || col.column_name == Some("company_code".to_string())
            {
                body.push(format!(
                    "    .eq(\"{}\", cond.{}.clone())",
                    col.column_name.clone().unwrap_or_default(),
                    safe_struct_field_name(
                        &col.column_name
                            .clone()
                            .unwrap_or_default()
                            .to_string()
                            .to_lowercase()
                    )
                ));
                has_cond = true;
            }
        }
    }

    for col in pkcols.clone() {
        params.push((
            "ids".to_string(),
            format!(
                "&[{}]",
                parse_data_type_as_rust_type(
                    &col.data_type.clone().unwrap_or_default().to_lowercase()
                )
            ),
        ));
    }

    if ctx.codegen_conf.multi_tenancy {
        if has_cond {
            params.push(("cond".to_string(), format!("&{}", struct_name.clone())));
        } else {
            params.push(("_cond".to_string(), format!("&{}", struct_name.clone())));
        }
    }

    let last = body.remove(body.len() - 1);
    body.push(last + ";");
    if tbconf.with_attachment {
        let mut savestr = String::new();
        savestr.push_str("match rb.fetch_list_by_wrapper::<Self>(wp.clone()).await {");
        body.push(savestr);
        body.push(format!("Ok(fss) => {{"));
        body.push(format!("for mut xss in fss.clone() {{"));
        body.push(format!("match xss.remove_attachments(rb).await {{"));
        body.push(format!("Ok(_) => {{}}"));
        body.push(format!("Err(_) => {{}}"));
        body.push(format!("}}"));
        body.push(format!("}}"));
        body.push(format!("}}"));
        body.push(format!("Err(_) => {{"));
        body.push(format!("}}"));
        body.push(format!("}}"));

        let mut savestr = String::new();
        savestr.push_str("rb.remove_by_wrapper::<Self>(wp).await");
        body.push(savestr);
    } else {
        let mut savestr = String::new();
        savestr.push_str("rb.remove_by_wrapper::<Self>(wp).await");
        body.push(savestr);
    }
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
 * 根据关联表来执行删除操作
 * 主要用于One-2-Many，One-2-One的删除
 * 在关联表中，需要实现主表删除后，关联表的数据也要跟着被删除
 */
pub fn generate_func_delete_rel_ids_for_struct(
    ctx: &GenerateContext,
    tbl: &TableInfo,
    rel: &RelationConfig,
) -> Option<RustFunc> {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    let struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };

    // 获得主表的配置信息
    let reltbc = ctx.get_table_conf(&rel.major_table.clone());
    if reltbc.is_none() {
        return None;
    }
    let reltbconf = reltbc.unwrap();

    let onemanyrelship = rel
        .one_to_many
        .clone()
        .into_iter()
        .filter(|f| f.table_name == tbl.table_name)
        .collect::<Vec<Relationship>>();
    let oneonerelship = rel
        .one_to_one
        .clone()
        .into_iter()
        .filter(|f| f.table_name == tbl.table_name)
        .collect::<Vec<Relationship>>();
    let relship = if onemanyrelship.is_empty() && oneonerelship.is_empty() {
        None
    } else if onemanyrelship.is_empty() {
        Some(oneonerelship[0].clone())
    } else if oneonerelship.is_empty() {
        Some(onemanyrelship[0].clone())
    } else {
        None
    };

    if relship.is_none() {
        return None;
    }

    let actrelship = relship.unwrap();
    let sxp = if actrelship.middle_table.is_some() {
        actrelship.major_field.unwrap_or_default()
    } else {
        actrelship.join_field.unwrap_or_default()
    };
    let reltable_name = if actrelship.middle_table.is_some() {
        actrelship.middle_table.unwrap_or_default()
    } else {
        actrelship.table_name.unwrap_or_default()
    };

    let multsxp = sxp
        .split(",")
        .into_iter()
        .map(|f| f.to_string())
        .collect::<Vec<String>>();

    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let pkcols = ctx
        .get_table_columns(&reltable_name.clone())
        .into_iter()
        .filter(|f| multsxp.contains(&f.column_name.clone().unwrap_or_default()))
        .collect::<Vec<ColumnInfo>>();

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&mut RBatisTxExecutor<'_>".to_string()));

    let mut body = vec![];
    let mut has_cond = false;

    body.push(format!("let wp = rb.get_rbatis().new_wrapper()"));
    for col in pkcols.clone() {
        body.push(format!(
            "    .r#in(\"{}\", ids)",
            col.column_name.clone().unwrap_or_default()
        ));
    }
    if ctx.codegen_conf.multi_tenancy {
        let allcols = ctx.get_table_columns(&tbl_name.clone());
        for col in allcols.clone() {
            if col.column_name == Some("company_id".to_string())
                || col.column_name == Some("company_code".to_string())
            {
                body.push(format!(
                    "    .eq(\"{}\", cond.{}.clone())",
                    col.column_name.clone().unwrap_or_default(),
                    safe_struct_field_name(
                        &col.column_name
                            .clone()
                            .unwrap_or_default()
                            .to_string()
                            .to_lowercase()
                    )
                ));
                has_cond = true;
            }
        }
    }

    let last = body.remove(body.len() - 1);
    body.push(last + ";");

    let mut savestr = String::new();
    savestr.push_str("rb.remove_by_wrapper::<Self>(wp).await");

    body.push(savestr);

    for col in pkcols.clone() {
        params.push((
            "ids".to_string(),
            format!(
                "&[{}]",
                parse_data_type_as_rust_type(
                    &col.data_type.clone().unwrap_or_default().to_lowercase()
                )
            ),
        ));
    }

    if has_cond {
        params.push(("cond".to_string(), format!("&{}", struct_name.clone())));
    } else {
        params.push(("_cond".to_string(), format!("&{}", struct_name.clone())));
    }

    let rfunc = RustFunc {
        is_struct_fn: true,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: format!("remove_{}_ids", reltbconf.api_handler_name.clone()),
        return_is_option: false,
        return_is_result: true,
        return_type: Some("u64".to_string()),
        params: params,
        bodylines: body,
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(tbconf.comment.clone()),
        api_method: None,
        api_pattern: None,
    };

    Some(rfunc)
}

/**
 * 执行Update Seletive操作
 */
pub fn generate_func_delete_batch_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let allcols = ctx.get_table_columns(&tbl_name.clone());

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&mut RBatisTxExecutor<'_>".to_string()));

    let mut body = vec![];

    body.push(format!("let wp = rb.get_rbatis().new_wrapper()"));
    process_col_generating(&tbconf, &mut body, &allcols);
    // body.remove(body.len() - 1);
    let last = body.remove(body.len() - 1);
    body.push(last + ";");

    if tbconf.with_attachment {
        body.push(format!(
            "match rb.fetch_list_by_wrapper::<Self>(wp.clone()).await {{"
        ));
        body.push(format!("Ok(ls) => {{"));
        body.push(format!("for mut it in ls.clone() {{"));
        body.push(format!("it.remove_attachments(rb).await?;"));
        body.push(format!("}}"));
        body.push(format!("}}"));
        body.push(format!("Err(err) => {{"));
        body.push(format!("return Err(err);"));
        body.push(format!("}}"));
        body.push(format!("}}"));
    }

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
 * 根据列来生成查询条件
 * 主要是处理基本的列的条件，以及：
 * 1. in_columns中指定列的条件
 * 2. full_text_columns
 * 3. datetime_between_columns
 */
fn process_col_generating(tbconf: &TableConfig, body: &mut Vec<String>, allcols: &Vec<ColumnInfo>) {
    let dtcols: Vec<String> = tbconf
        .datetime_between_columns
        .clone()
        .unwrap_or_default()
        .to_lowercase()
        .split(",")
        .map(|f| f.trim().to_string())
        .collect();
    let incols: Vec<String> = tbconf
        .in_columns
        .clone()
        .unwrap_or_default()
        .to_lowercase()
        .split(",")
        .map(|f| f.trim().to_string())
        .collect();
    let ftcols: Vec<String> = tbconf
        .full_text_columns
        .clone()
        .unwrap_or_default()
        .to_lowercase()
        .split(",")
        .map(|f| f.trim().to_string())
        .collect();

    for col in allcols.clone() {
        let safe_fdname = safe_struct_field_name(
            &col.column_name
                .clone()
                .unwrap_or_default()
                .to_string()
                .to_lowercase(),
        );
        if tbconf
            .tree_parent_field
            .clone()
            .unwrap_or_default()
            .to_lowercase()
            == safe_fdname.clone()
        {
            if !is_copied_data_type(&col.data_type.clone().unwrap_or_default()) {
                body.push(format!(
                    "         .r#if(self.{}.is_some(), |w| w.and().eq(\"{}\", self.{}.unwrap()))",
                    safe_fdname.clone(),
                    col.column_name.clone().unwrap_or_default(),
                    safe_fdname.clone()
                ));
            } else {
                body.push(format!("         .r#if(self.{}.is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
            }
            if tbconf
                .tree_root_value
                .clone()
                .unwrap_or_default()
                .to_lowercase()
                == "null".to_string()
            {
                body.push(format!(
                    "         .r#if(self.{}.is_none(), |w| w.and().is_null(\"{}\"))",
                    safe_fdname.clone(),
                    col.column_name.clone().unwrap_or_default()
                ));
            } else {
                body.push(format!(
                    "         .r#if(self.{}.is_none(), |w| w.and().eq(\"{}\", Some({})))",
                    safe_fdname.clone(),
                    col.column_name.clone().unwrap_or_default(),
                    tbconf
                        .tree_root_value
                        .clone()
                        .unwrap_or_default()
                        .to_lowercase()
                ));
            }
        } else {
            let col_name = col.column_name.clone().unwrap_or_default();
            let mut processed = false;
            if ftcols.len() > 0 {
                let ft = ftcols[0].clone();
                if ft == col_name {
                    let all_ft_cols = tbconf.full_text_columns.clone().unwrap_or_default();
                    let ft_sql = format!(" (match({}) against(?)) ", all_ft_cols);
                    body.push(format!("         .r#if(self.{}.clone().is_some(), |w| w.and().push_sql(\"{}\").push_arg(self.{}.clone().unwrap()))", safe_fdname.clone(), ft_sql, safe_fdname.clone()));
                    processed = true;
                }
            }

            if incols.len() > 0 {
                let inspliter = tbconf.in_spliter.clone().unwrap_or_default();
                for stcol in incols.clone() {
                    if stcol == col_name {
                        body.push(format!("         .r#if(self.{}.is_some() && self.{}.clone().unwrap().contains(\"{}\"), |w| w.and().r#in(\"{}\", &self.{}.clone().unwrap_or_default().split(\"{}\").map(|f| f.trim().to_string()).collect::<Vec<String>>().as_slice()))", safe_fdname.clone(), safe_fdname.clone(), inspliter, col_name.clone(), safe_fdname.clone(), inspliter));
                        processed = true;
                        break;
                    }
                }
            }

            if dtcols.len() == 2 {
                //只能是2个
                let dtfst = dtcols[0].clone();
                let dtsec = dtcols[1].clone();
                let fdfst = safe_struct_field_name(&dtfst);
                let fdsec = safe_struct_field_name(&dtsec);
                if dtfst == col_name {
                    if is_copied_data_type(&col.data_type.clone().unwrap_or_default()) {
                        body.push(format!("         .r#if(self.{}.is_some() && self.{}.is_some(), |w| w.and().between(\"{}\", self.{}.unwrap(), self.{}.unwrap()))", fdfst.clone(), fdsec.clone(), col_name.clone(), fdfst.clone(), fdsec.clone()));
                        body.push(format!("         .r#if(self.{}.is_some() && self.{}.is_none(), |w| w.and().eq(\"{}\", self.{}.unwrap()))", fdfst.clone(), fdsec.clone(), col_name.clone(), fdfst.clone()));
                    } else {
                        body.push(format!("         .r#if(self.{}.is_some() && self.{}.is_some(), |w| w.and().between(\"{}\", self.{}.clone().unwrap(), self.{}.clone().unwrap()))", fdfst.clone(), fdsec.clone(), col_name.clone(), fdfst.clone(), fdsec.clone()));
                        body.push(format!("         .r#if(self.{}.is_some() && self.{}.is_none(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", fdfst.clone(), fdsec.clone(), col_name.clone(), fdfst.clone()));
                    }
                    processed = true;
                } else if dtsec == col_name {
                    if is_copied_data_type(&col.data_type.clone().unwrap_or_default()) {
                        body.push(format!("         .r#if(self.{}.is_none() && self.{}.is_some(), |w| w.and().eq(\"{}\", self.{}.unwrap()))", fdfst.clone(), fdsec.clone(), col_name.clone(), fdfst.clone()));
                    } else {
                        body.push(format!("         .r#if(self.{}.is_none() && self.{}.is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", fdfst.clone(), fdsec.clone(), col_name.clone(), fdfst.clone()));
                    }
                    processed = true;
                }
            }

            if processed == false {
                if is_copied_data_type(&col.data_type.clone().unwrap_or_default()) {
                    body.push(format!("         .r#if(self.{}.is_some(), |w| w.and().eq(\"{}\", self.{}.unwrap()))", safe_fdname.clone(), col_name.clone(), safe_fdname.clone()));
                } else {
                    body.push(format!("         .r#if(self.{}.is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", safe_fdname.clone(), col_name.clone(), safe_fdname.clone()));
                }
            }
        }
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
    let allcols = ctx.get_table_columns(&tbl_name.clone());

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));
    params.push(("curr".to_string(), "u64".to_string()));
    params.push(("ps".to_string(), "u64".to_string()));

    let mut body = vec![];

    body.push(format!("let wp = rb.new_wrapper()"));
    process_col_generating(&tbconf, &mut body, &allcols);
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
 * 使用CommonSearch结构来处理
 */
pub fn generate_func_common_page_query_for_struct(
    ctx: &GenerateContext,
    tbl: &TableInfo,
) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let _allcols = ctx.get_table_columns(&tbl_name.clone());

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));
    params.push(("cs".to_string(), "CommonSearch".to_string()));
    params.push(("curr".to_string(), "u64".to_string()));
    params.push(("ps".to_string(), "u64".to_string()));

    let mut body = vec![];

    body.push(format!("let mut wp = rb.new_wrapper();"));
    // process_col_generating(&tbconf, &mut body, &allcols);
    body.push(format!("wp = cs.into_wrapper(wp);"));
    // body.remove(body.len() - 1);
    let mut savestr = String::new();
    savestr.push_str("rb.fetch_page_by_wrapper::<Self>(wp, &PageRequest::new(curr, ps)).await");

    body.push(savestr);
    RustFunc {
        is_struct_fn: true,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: "common_query_paged".to_string(),
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
    let allcols = ctx.get_table_columns(&tbl_name.clone());

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));

    let mut body = vec![];

    body.push(format!("let wp = rb.new_wrapper()"));
    process_col_generating(&tbconf, &mut body, &allcols);
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
pub fn generate_func_common_list_query_for_struct(
    ctx: &GenerateContext,
    tbl: &TableInfo,
) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    // let mut allcols = ctx.get_table_columns(&tbl_name.clone());

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));
    params.push(("cs".to_string(), "CommonSearch".to_string()));
    let mut body = vec![];

    body.push(format!("let mut wp = rb.new_wrapper();"));
    // process_col_generating(&tbconf, &mut body, &allcols);
    body.push(format!("wp = cs.into_wrapper(wp);"));
    // body.remove(body.len() - 1);

    let mut savestr = String::new();
    savestr.push_str("rb.fetch_list_by_wrapper::<Self>(wp).await");

    body.push(savestr);
    RustFunc {
        is_struct_fn: true,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: "common_query_list".to_string(),
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
    let _allcols = ctx.get_table_columns(&tbl_name.clone());

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
    let _allcols = ctx.get_table_columns(&tbl_name.clone());

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));

    let treecolopt = ctx.find_table_column(
        &tbl_name.clone(),
        &tbc.tree_parent_field.unwrap_or_default(),
    );
    let treecol = treecolopt.unwrap();

    params.push((
        "pid".to_string(),
        format!(
            "&Option<{}>",
            parse_data_type_as_rust_type(&treecol.data_type.clone().unwrap_or_default())
        ),
    ));

    let root_value = tbc.tree_root_value.unwrap_or_default();
    let mut body = vec![];

    body.push(format!("let wp = rb.new_wrapper()"));
    // for col in allcols.clone() {
    let safe_fdname = safe_struct_field_name(
        &treecol
            .column_name
            .clone()
            .unwrap_or_default()
            .to_string()
            .to_lowercase(),
    );
    body.push(format!(
        "         .r#if({}.clone().is_some(), |w| w.and().eq(\"{}\", {}.unwrap()))",
        safe_fdname.clone(),
        treecol.column_name.clone().unwrap_or_default(),
        safe_fdname.clone()
    ));

    if root_value == "null" {
        body.push(format!(
            "         .r#if({}.clone().is_none(), |w| w.and().is_null(\"{}\"))",
            safe_fdname.clone(),
            treecol.column_name.clone().unwrap_or_default()
        ));
    } else {
        body.push(format!(
            "         .r#if({}.clone().is_none(), |w| w.and().eq(\"{}\", {}))",
            safe_fdname.clone(),
            treecol.column_name.clone().unwrap_or_default(),
            root_value
        ));
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
pub fn generate_func_simple_func_for_struct(
    ctx: &GenerateContext,
    tbl: &TableInfo,
    simplefun: &SimpleFuncation,
) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let allcols = ctx.get_table_columns(&tbl_name.clone());

    let mut params = Vec::new();

    let sp = simplefun.condition.split(",");
    let mut condcols = vec![];
    for row in sp.into_iter() {
        for lc in allcols.clone() {
            if lc.column_name == Some(row.trim().to_string()) {
                condcols.push(lc.clone());
                break;
            }
        }
    }

    // log::info!("All cols: {}, Condition Cols: {}", allcols.len(), condcols.len());

    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));

    if !simplefun.is_self {
        // add the params
        for col in condcols.clone() {
            let safe_fdname = safe_struct_field_name(
                &col.column_name
                    .clone()
                    .unwrap_or_default()
                    .to_string()
                    .to_lowercase(),
            );
            let dt = parse_data_type_as_rust_type(
                &col.data_type.clone().unwrap_or_default().to_lowercase(),
            );
            if simplefun.param_optional {
                params.push((safe_fdname, format!("&Option<{}>", dt)));
            } else {
                params.push((safe_fdname, format!("&{}", dt)));
            }
        }
    }

    if simplefun.is_paged {
        params.push(("curr".to_string(), "u64".to_string()));
        params.push(("ps".to_string(), "u64".to_string()));
    }

    let mut body = vec![];

    body.push(format!("let wp = rb.new_wrapper()"));
    for col in condcols.clone() {
        let safe_fdname = safe_struct_field_name(
            &col.column_name
                .clone()
                .unwrap_or_default()
                .to_string()
                .to_lowercase(),
        );
        let safe_fdtype = col.data_type.unwrap_or_default();
        if simplefun.is_self {
            if simplefun.param_optional {
                if is_copied_data_type(&safe_fdtype) {
                    body.push(format!("         .r#if(self.{}.is_some(), |w| w.and().eq(\"{}\", self.{}.unwrap()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));
                } else {
                    body.push(format!("         .r#if(self.{}.is_some(), |w| w.and().eq(\"{}\", self.{}.clone().unwrap()))", safe_fdname.clone(), col.column_name.clone().unwrap_or_default(), safe_fdname.clone()));                    
                }
            } else {
                if is_copied_data_type(&safe_fdtype) {
                    body.push(format!(
                        "         .and().eq(\"{}\", self.{}.unwrap())",
                        col.column_name.clone().unwrap_or_default(),
                        safe_fdname.clone()
                    ));
                } else {
                    body.push(format!(
                        "         .and().eq(\"{}\", self.{}.clone().unwrap())",
                        col.column_name.clone().unwrap_or_default(),
                        safe_fdname.clone()
                    ));
                }
            }
        } else {
            if simplefun.param_optional {
                if is_copied_data_type(&safe_fdtype) {
                    body.push(format!(
                        "         .r#if({}.is_some(), |w| w.and().eq(\"{}\", {}.unwrap()))",
                        safe_fdname.clone(),
                        col.column_name.clone().unwrap_or_default(),
                        safe_fdname.clone()
                    ));
                } else {
                    body.push(format!(
                        "         .r#if({}.is_some(), |w| w.and().eq(\"{}\", {}.clone().unwrap()))",
                        safe_fdname.clone(),
                        col.column_name.clone().unwrap_or_default(),
                        safe_fdname.clone()
                    ));
                }
            } else {
                if is_copied_data_type(&safe_fdtype) {
                    body.push(format!(
                        "         .and().eq(\"{}\", {})",
                        col.column_name.clone().unwrap_or_default(),
                        safe_fdname.clone()
                    ));
                } else {
                    body.push(format!(
                        "         .and().eq(\"{}\", {}.clone())",
                        col.column_name.clone().unwrap_or_default(),
                        safe_fdname.clone()
                    ));
                }
            }
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

pub fn parse_table_as_value_object_struct(
    ctx: &GenerateContext,
    tbl: &TableInfo,
    cols: &Vec<ColumnInfo>,
    using_list: &mut Vec<String>,
) -> RustStruct {
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

    let mut fields = parse_column_list(ctx, &tbconf, cols, &mut columns, true, using_list);

    let mut annts = vec![];

    if ctx.codegen_conf.allow_bool_widecard {
        annts.push("#[serde(default)]".to_string());
        annts.push("#[serde(deserialize_with=\"bool_from_str\")]".to_string());
    }

    if tbconf.tree_parent_field.is_some() {
        let has_children = RustStructField {
            is_pub: true,
            schema_name: None,
            column_name: "has_children".to_string(),
            field_name: "has_children".to_string(),
            field_type: "bool".to_string(),
            orignal_field_name: None,
            comment: None,
            is_option: false,
            length: 0i64,
            annotations: vec![],
        };

        let leaf = RustStructField {
            is_pub: true,
            schema_name: None,
            column_name: "leaf".to_string(),
            field_name: "leaf".to_string(),
            field_type: "bool".to_string(),
            is_option: false,
            orignal_field_name: None,
            comment: None,
            length: 0i64,
            annotations: vec![],
        };

        let label = RustStructField {
            is_pub: true,
            schema_name: None,
            column_name: "label".to_string(),
            field_name: "label".to_string(),
            field_type: "String".to_string(),
            is_option: true,
            orignal_field_name: None,
            comment: None,
            length: 0i64,
            annotations: vec![],
        };

        let children = RustStructField {
            is_pub: true,
            schema_name: None,
            column_name: "children".to_string(),
            field_name: "children".to_string(),
            field_type: format!("Vec<{}>", valobjstruct_name),
            is_option: false,
            orignal_field_name: None,
            comment: None,
            length: 0i64,
            annotations: vec!["#[serde(default)]".to_string()],
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
        usings: using_list.clone(),
    }
}

fn guess_label_field(cols: &Vec<ColumnInfo>) -> String {
    let mut first_string = None;
    let mut lable_field = None;
    for col in cols.clone() {
        let dt = parse_data_type_as_rust_type(&col.data_type.clone().unwrap_or_default());
        let fd_name =
            safe_struct_field_name(&col.column_name.clone().unwrap_or_default().to_lowercase());
        if dt == "String" && first_string.is_none() {
            first_string = Some(fd_name.clone());
        }

        if fd_name == "label" {
            return fd_name;
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
                return String::new();
            } else {
                let fd_name = safe_struct_field_name(
                    &cols[0]
                        .column_name
                        .clone()
                        .unwrap_or_default()
                        .to_lowercase(),
                );
                return fd_name;
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

    let mut usings = vec![];
    let mut columns = String::new();
    let cols = ctx.get_table_columns(&tbl_name.clone());
    let parsed_fields = parse_column_list(ctx, &tbconf, &cols, &mut columns, true, &mut usings);
    for fd in parsed_fields {
        let fname = fd.field_name.clone();
        if fd.orignal_field_name.is_none() {
            body.push(format!(
                "{}: self.{}.clone(),",
                safe_struct_field_name(&fname),
                safe_struct_field_name(&fname)
            ));
        } else {
            body.push(format!(
                "{}: self.{}.clone(),",
                safe_struct_field_name(&fd.orignal_field_name.clone().unwrap_or_default()),
                safe_struct_field_name(&fname)
            ));
        }
    }

    body.push(format!("}}"));

    let params = Vec::new();
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

fn generate_func_value_object_from_entity(
    ctx: &GenerateContext,
    tbl: &TableInfo,
    simple: bool,
) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap();
    let tbc = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tbc.unwrap();
    let mut body = vec![];

    body.push(format!("Self {{"));

    let mut columns = String::new();
    let cols = ctx.get_table_columns(&tbl_name.clone());
    let mut usings = vec![];
    let parsed_fields = parse_column_list(ctx, &tbconf, &cols, &mut columns, true, &mut usings);
    for fd in parsed_fields {
        let fname = fd.field_name.clone();
        if fd.orignal_field_name.is_none() {
            body.push(format!(
                "{}: param.{}.clone(),",
                safe_struct_field_name(&fname),
                safe_struct_field_name(&fname)
            ));
        } else {
            body.push(format!(
                "{}: param.{}.clone(),",
                safe_struct_field_name(&fname),
                safe_struct_field_name(&fd.orignal_field_name.clone().unwrap_or_default())
            ));
        }
    }
    if simple {
        body.push(format!("has_children: false,"));
        body.push("leaf: false,".to_string());
        body.push(format!("children: vec![],"));
    } else {
        body.push(format!("has_children: haschild,"));
        body.push("leaf: haschild == false,".to_string());
        body.push(format!("children: children.to_vec(),"));
    }
    body.push(format!(
        "label: param.{}.clone(),",
        guess_label_field(&cols)
    ));

    body.push(format!("}}"));

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push((
        "param".to_string(),
        "&".to_owned() + tbconf.struct_name.clone().as_str(),
    ));

    if !simple {
        params.push(("haschild".to_string(), "bool".to_string()));
        params.push(("children".to_string(), format!("&[Self]")));
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
    params.push(("items".to_string(), "&[Self]".to_string()));

    body.push(format!("let mut tmptree = vec![];"));
    body.push(format!("for xip in items.iter.cloned() {{"));
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

fn generate_fun_build_tree_rec_for_value(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap();
    let tbc = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tbc.unwrap();
    let mut body = vec![];
    let mut params = vec![];
    params.push(("items".to_string(), "&[Self]".to_string()));
    params.push(("parent_item".to_string(), "&mut Self".to_string()));

    body.push(format!("for xip in items.iter.cloned() {{"));
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

/**
 * 自动根据注释中的关联信息来处理，来生成对应的复合表查询
 * 如表A，注释中标识，关联表B，则表B的基础字段与表A的全部字段组合成为一个新的Struct
 * 该新的Struct来说，将只进行查询（含分页查询）
 * 新的composite struct只是对指定表的直接关联，所以，实际上建业务表时，尽可能是平铺表，（flat table），
 * 对于一些更复杂的查询，则可以参考该实现手工去实现。
 * 这种类型可以满足大部分需求，且达到绝大多数形式。
 */
pub fn parse_composite_column_list(
    ctx: &GenerateContext,
    tbl: &TableConfig,
    cols: &Vec<ColumnInfo>,
    columns: &mut String,
    joinlist: &mut String,
    rename_id: bool,
    extended: bool,
    usings: &mut Vec<String>,
) -> Vec<RustStructFieldExtend> {
    let mut fields = vec![];
    let mut ti = 1i32;
    let mut ci = 1i32;
    let mut union_fields = vec![];
    for col in cols {
        let fd = parse_column_as_field(ctx, tbl, &col, rename_id,  usings);
        let mut rsfd = RustStructFieldExtend::parse(&fd);
        let colname = col.column_name.clone().unwrap_or_default();
        if !union_fields.contains(&colname) {
            union_fields.push(colname.clone());
            columns.push_str(format!("t0.{}", colname).as_str());
            columns.push(',');
            ci = ci + 1;
            if ci % 7 == 0 {
                columns.push_str("\n\t\t\t\t\t\t");
            }
            rsfd.alias = Some("t0".to_string());
            fields.push(rsfd.clone());
        }
        // log::info!("Relation: {}", rsfd.relation.clone().unwrap_or_default());

        if rsfd.relation.is_some() && extended {
            // 有关联表的情况

            let rel_table_name = rsfd.relation.unwrap_or_default();
            let reltblconf = ctx.get_table_conf(&rel_table_name.clone());
            if reltblconf.is_none() {
                continue;
            }
            let reltbconfig = reltblconf.unwrap();
            let mut pkcols = ctx.get_table_column_by_primary_key(&rel_table_name.clone());
            if pkcols.is_empty() {
                pkcols.append(&mut ctx.get_table_pkey_column(&rel_table_name.clone()));
            }
            let pkcol = pkcols[0].clone();
            let pkname = pkcol.column_name.clone().unwrap_or_default();

            ti = ti + 1;
            joinlist.push_str(
                format!(
                    " INNER JOIN {} t{} ON t{}.{} = t0.{} \n\t\t\t\t\t\t",
                    rel_table_name.clone(),
                    ti,
                    ti,
                    colname.clone(),
                    pkname
                )
                .as_str(),
            );

            let rel_cols = ctx.get_table_basic_columns(&rel_table_name.clone(), false);
            for rel_col in rel_cols.clone() {
                let rel_colname = rel_col.column_name.clone().unwrap_or_default();
                if !union_fields.contains(&rel_colname) {
                    union_fields.push(rel_colname.clone());
                    columns.push_str(format!("t{}.{}", ti, rel_colname.clone()).as_str()); // extnd
                    columns.push(',');
                    ci = ci + 1;
                    if ci % 7 == 0 {
                        columns.push_str("\n\t\t\t\t\t\t");
                    }
                    let rel_fd = parse_column_as_field(ctx, &reltbconfig, &rel_col, rename_id, usings);
                    let mut rel_fdet = RustStructFieldExtend::parse(&rel_fd);
                    rel_fdet.hidden = true;
                    rel_fdet.condition = false;
                    rel_fdet.alias = Some(format!("t{}", ti));
                    fields.push(rel_fdet);
                }
            }
        }
    }
    fields
}

pub fn parse_table_as_composite_struct(
    ctx: &GenerateContext,
    tbl: &TableInfo,
    cols: &Vec<ColumnInfo>,
) -> Option<RustStruct> {
    let mut columns = String::new();
    let mut joinstr = String::new();
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone());

    if tbc.is_none() {
        return None;
    }

    let mut usings = vec![];
    let tbconf = tbc.unwrap();

    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut fields =
        parse_composite_column_list(ctx, &tbconf, cols, &mut columns, &mut joinstr, false, true, &mut usings);
    if columns.ends_with(",") {
        columns = columns.substring(0, columns.len() - 1).to_string();
    }

    if joinstr.is_empty() {
        return None;
    }

    // refine the DateTime field for add then end field to receive date range condition
    let mut attach_fields = vec![];
    for fl in fields.clone() {
        if fl.field.field_type == "rbatis::DateTimeNative".to_string() {
            let mut cfl = fl.clone();
            cfl.field.field_name = fl.field.field_name.clone() + "_end";
            cfl.field.orignal_field_name = Some(fl.field.field_name.clone() + "_end");
            cfl.flag = 1i64;
            attach_fields.push(cfl);
        }
    }

    if !attach_fields.is_empty() {
        fields.append(&mut attach_fields);
    }

    let anno = vec!["#[derive(Debug, Clone, Default, Deserialize, Serialize)]".to_string()];

    let mut funclist = vec![];
    let query_func =
        generate_query_func_for_extend_struct(ctx, tbl, &fields, false, &columns, &joinstr);
    funclist.push(query_func);

    let page_query_func =
        generate_query_func_for_extend_struct(ctx, tbl, &fields, true, &columns, &joinstr);
    funclist.push(page_query_func);

    let rs = RustStruct {
        is_pub: true,
        has_paging: tbconf.page_query,
        struct_name: match ctx.get_struct_name(&tbl_name.clone()) {
            Some(t) => t + "Present",
            None => pascal_case(tbl_name.clone().as_str()) + "Present",
        },
        annotations: anno,
        fields: fields
            .into_iter()
            .map(|f| f.field)
            .collect::<Vec<RustStructField>>(),
        funclist: funclist,
        usings
    };

    Some(rs)
}

/**
 * 生成扩展Present结构的列表查询，以及分页查询
 * 根据字段来处理
 *
 */
pub fn generate_query_func_for_extend_struct(
    ctx: &GenerateContext,
    tbl: &TableInfo,
    fields: &Vec<RustStructFieldExtend>,
    paged: bool,
    coltext: &String,
    jointext: &String,
) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let _allcols = ctx.get_table_columns(&tbl_name.clone());

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));
    params.push((
        "param".to_string(),
        format!(" &{}Present", tbconf.struct_name.clone()),
    ));
    if paged {
        params.push(("curr".to_string(), "u64".to_string()));
        params.push(("ps".to_string(), "u64".to_string()));
    }

    let mut body = vec![];

    let sql = "select ".to_owned()
        + coltext.clone().as_str()
        + " from "
        + tbl_name.as_str()
        + " t0 "
        + jointext.as_str()
        + " where 1 = 1 ";

    body.push(format!("let mut sql = r#\"{}\"#.to_string();", sql));
    body.push(format!("let mut rb_args = vec![];"));
    for fd in fields.clone() {
        if fd.flag == 0i64 {
            if fd.field.field_type == "rbatis::DateTimeNative".to_string() {
                body.push(format!(
                    "if param.{}.is_some() && param.{}_end.is_some() {{",
                    fd.field.column_name.clone(),
                    fd.field.column_name.clone()
                ));
                body.push(format!(
                    "rb_args.push(rbson::to_bson(&param.{}).unwrap_or_default());",
                    fd.field.column_name.clone()
                ));
                body.push(format!(
                    "rb_args.push(rbson::to_bson(&param.{}_end).unwrap_or_default());",
                    fd.field.column_name.clone()
                ));
                if fd.alias.is_some() {
                    body.push(format!(
                        "sql.push_str(\" AND {}.{} BETWEEN ? AND ? \");",
                        fd.alias.clone().unwrap_or_default(),
                        fd.field.column_name.clone()
                    ));
                } else {
                    body.push(format!(
                        "sql.push_str(\" AND {} BETWEEN ? AND ? \");",
                        fd.field.column_name.clone()
                    ));
                }
                body.push(format!(
                    "}} else if param.{}.is_some() {{",
                    fd.field.column_name.clone()
                ));
                body.push(format!(
                    "rb_args.push(rbson::to_bson(&param.{}).unwrap_or_default());",
                    fd.field.column_name.clone()
                ));
                // body.push(format!("sql.push_str(\" AND {} = ? \");", fd.field.column_name.clone()));
                if fd.alias.is_some() {
                    body.push(format!(
                        "sql.push_str(\" AND {}.{} = ? \");",
                        fd.alias.clone().unwrap_or_default(),
                        fd.field.column_name.clone()
                    ));
                } else {
                    body.push(format!(
                        "sql.push_str(\" AND {} = ? \");",
                        fd.field.column_name.clone()
                    ));
                }
                body.push(format!("}}"));
            } else {
                body.push(format!(
                    "if param.{}.is_some() {{",
                    fd.field.column_name.clone()
                ));
                body.push(format!(
                    "rb_args.push(rbson::to_bson(&param.{}).unwrap_or_default());",
                    fd.field.column_name.clone()
                ));
                if fd.alias.is_some() {
                    body.push(format!(
                        "sql.push_str(\" AND {}.{} = ? \");",
                        fd.alias.clone().unwrap_or_default(),
                        fd.field.column_name.clone()
                    ));
                } else {
                    body.push(format!(
                        "sql.push_str(\" AND {} = ? \");",
                        fd.field.column_name.clone()
                    ));
                }

                body.push(format!("}}"));
            }
        }
    }

    if paged {
        body.push(format!(
            "rb.fetch_page(&sql, rb_args, &PageRequest::new(curr, ps)).await"
        ));
    } else {
        body.push(format!("rb.fetch(&sql, rb_args).await"));
    }

    RustFunc {
        is_struct_fn: true,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: if paged {
            "query_paged".to_string()
        } else {
            "query_list".to_string()
        },
        return_is_option: false,
        return_is_result: true,
        return_type: if paged {
            Some("Page<Self>".to_string())
        } else {
            Some("Vec<Self>".to_string())
        },
        params: params,
        bodylines: body,
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(tbconf.comment.clone()),
        api_method: None,
        api_pattern: None,
    }
}

pub fn generate_func_load_attachment(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }
    let pkcol = pkcols[0].clone();
    let safe_pkname =
        safe_struct_field_name(&pkcol.column_name.clone().unwrap_or_default().to_lowercase());

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));

    let mut body = vec![];

    body.push(format!(
        "match ChimesAttachmentInfo::find_attachments(rb, &\"{}\".to_string(), &self.{}).await {{",
        tbconf.api_handler_name.clone(),
        safe_pkname
    ));
    body.push(format!("Ok(ts) => {{"));
    body.push(format!("self.attachments = ts.clone();"));
    body.push(format!("Ok(ts.len() as u64)"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!("Err(err)"));
    body.push(format!("}}"));
    body.push(format!("}}"));

    RustFunc {
        is_struct_fn: true,
        is_self_fn: true,
        is_self_mut: true,
        is_pub: true,
        is_async: true,
        func_name: "load_attachment".to_string(),
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

pub fn generate_func_save_attachment(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }
    let pkcol = pkcols[0].clone();
    let safe_pkname =
        safe_struct_field_name(&pkcol.column_name.clone().unwrap_or_default().to_lowercase());
    let safe_pktype = pkcol.data_type.clone().unwrap_or_default();

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&mut RBatisTxExecutor<'_>".to_string()));

    let mut body = vec![];

    body.push(format!("for fp in self.attachments.clone() {{"));
    body.push(format!("let rel = ChimesAttachmentRefInfo {{"));
    body.push(format!("attachment_id: fp.attachment_id,"));
    body.push(format!(
        "business_name: Some(\"{}\".to_string()),",
        tbconf.api_handler_name.clone()
    ));
    if is_copied_data_type(&safe_pktype) {
        body.push(format!("business_id: self.{},", safe_pkname));
    } else {
        body.push(format!("business_id: self.{}.clone(),", safe_pkname));
    }
    body.push(format!("update_time: Some(rbatis::DateTimeNative::now()),"));
    body.push(format!("create_time: Some(rbatis::DateTimeNative::now()),"));
    body.push(format!("..Default::default()"));
    body.push(format!("}};"));
    body.push(format!("let _ = rel.remove_batch(rb).await.is_ok();"));
    body.push(format!("let _ = rel.save(rb).await.is_ok();"));
    body.push(format!("}}"));
    body.push(format!("Ok(0u64)"));

    RustFunc {
        is_struct_fn: true,
        is_self_fn: true,
        is_self_mut: true,
        is_pub: true,
        is_async: true,
        func_name: "save_attachment".to_string(),
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

pub fn generate_func_remove_attachment(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }
    let pkcol = pkcols[0].clone();
    let safe_pkname =
        safe_struct_field_name(&pkcol.column_name.clone().unwrap_or_default().to_lowercase());
    let safe_pktype = pkcol.data_type.clone().unwrap_or_default();

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&mut RBatisTxExecutor<'_>".to_string()));
    params.push(("attach_id".to_string(), "&i64".to_string()));

    let mut body = vec![];

    body.push(format!("let rel = ChimesAttachmentRefInfo {{"));
    body.push(format!("attachment_id: Some(attach_id),"));
    body.push(format!(
        "business_name: Some(\"{}\".to_string()),",
        tbconf.api_handler_name
    ));
    if is_copied_data_type(&safe_pktype) {
        body.push(format!("business_id: self.{},", safe_pkname));
    } else {
        body.push(format!("business_id: self.{}.clone(),", safe_pkname));
    }
    body.push(format!("..Default::default()"));
    body.push(format!("}};"));

    body.push(format!("match rel.remove_batch(rb).await {{"));
    body.push(format!("Ok(t) => {{"));
    body.push(format!("Ok(t)"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!("Err(err)"));
    body.push(format!("}}"));
    body.push(format!("}}"));

    RustFunc {
        is_struct_fn: true,
        is_self_fn: true,
        is_self_mut: true,
        is_pub: true,
        is_async: true,
        func_name: "remove_attachment".to_string(),
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

pub fn generate_func_remove_attachments(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();

    let mut params = Vec::new();
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }
    let pkcol = pkcols[0].clone();
    let safe_pkname =
        safe_struct_field_name(&pkcol.column_name.clone().unwrap_or_default().to_lowercase());
    let safe_pktype = pkcol.data_type.clone().unwrap_or_default();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&mut RBatisTxExecutor<'_>".to_string()));

    let mut body = vec![];

    body.push(format!("let rel = ChimesAttachmentRefInfo {{"));
    body.push(format!(
        "business_name: Some(\"{}\".to_string()),",
        tbconf.api_handler_name
    ));
    if is_copied_data_type(&safe_pktype) {
        body.push(format!("business_id: self.{},", safe_pkname));
    } else {
        body.push(format!("business_id: self.{}.clone(),", safe_pkname));
    }
    body.push(format!("..Default::default()"));
    body.push(format!("}};"));

    body.push(format!("match rel.remove_batch(rb).await {{"));
    body.push(format!("Ok(t) => {{"));
    body.push(format!("Ok(t)"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!("Err(err)"));
    body.push(format!("}}"));
    body.push(format!("}}"));

    RustFunc {
        is_struct_fn: true,
        is_self_fn: true,
        is_self_mut: true,
        is_pub: true,
        is_async: true,
        func_name: "remove_attachments".to_string(),
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
