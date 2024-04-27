use super::{parse_column_list, RustStruct};
use crate::codegen::{parse_data_type_as_rust_type, GenerateContext, RustFunc};
use crate::schema::TableInfo;
use change_case::pascal_case;

/**
 * 根据ctx.codegenconf.multi_tenancy来决定是否生成su: SystemUser<ChimesUserInfo>
 */
pub fn generate_actix_handler_for_table(
    ctx: &mut GenerateContext,
    tbl: &TableInfo,
    usinglist: &mut Vec<String>,
    comps: &Option<RustStruct>,
) -> Vec<RustFunc> {
    let mut funclist = vec![];
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbl_struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };

    let tbc = ctx.get_table_conf(&tbl_name.clone()).unwrap();

    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let save_handler = generate_handler_save_for_struct(ctx, tbl);
    funclist.push(save_handler);

    let update_handler = generate_handler_update_for_struct(ctx, tbl);
    funclist.push(update_handler);
    let delete_handler = generate_handler_delete_for_struct(ctx, tbl);
    funclist.push(delete_handler);

    if pkcols.len() == 1 {
        let delete_ids_handler = generate_handler_delete_ids_for_struct(ctx, tbl);
        funclist.push(delete_ids_handler);
    }

    let list_handler = generate_handler_query_list_for_struct(ctx, tbl);
    funclist.push(list_handler);
    if tbc.page_query {
        let page_handler = generate_handler_query_page_for_struct(ctx, tbl);
        funclist.push(page_handler);
        usinglist.push(format!("rbatis::Page"));
    }
    let get_handler = generate_handler_get_for_struct(ctx, tbl);
    funclist.push(get_handler);

    if comps.is_some() {
        let comps_list_handler = generate_handler_query_list_for_composite(ctx, tbl, comps);
        funclist.push(comps_list_handler);
        if tbc.page_query {
            let comps_page_handler = generate_handler_query_page_for_composite(ctx, tbl, comps);
            funclist.push(comps_page_handler);
            let rbimpl = format!("rbatis::Page");
            if !usinglist.contains(&rbimpl) {
                usinglist.push(rbimpl);
            }
        }
    }

    if tbc.tree_parent_field.is_some() {
        let tree_handler = generate_handler_query_tree_for_struct(ctx, tbl);
        funclist.push(tree_handler);
        usinglist.push(format!("actix_web::HttpRequest"));
        if tbc.generate_param_struct {
            usinglist.push(
                format!(
                    "crate::entity::{{{}, {}Value, {}Query}}",
                    tbl_struct_name.clone(),
                    tbl_struct_name.clone(),
                    tbl_struct_name.clone()
                )
                .to_string(),
            );
        } else {
            usinglist.push(
                format!(
                    "crate::entity::{{{}, {}Value}}",
                    tbl_struct_name.clone(),
                    tbl_struct_name.clone()
                )
                .to_string(),
            );
        }
    } else {
        if tbc.generate_param_struct {
            usinglist.push(
                format!(
                    "crate::entity::{{{}, {}Query}}",
                    tbl_struct_name.clone(),
                    tbl_struct_name.clone()
                )
                .to_string(),
            );
        } else {
            usinglist.push(format!("crate::entity::{}", tbl_struct_name.clone()).to_string());
        }
    }

    if comps.is_some() {
        usinglist.push(format!("crate::entity::{}Present", tbl_struct_name.clone()).to_string());
    }

    ctx.add_permission(tbl, &funclist);

    funclist
}

/**
 * 针对实体对象
 * 对其结构进行初始化操作
 * 1、初始化company_id, company_code
 * 2、初始化modify_by, modify_user_id, modify_user_name, create_by, create_user_id, create_user_name
 * 3、初始化create_time, update_time
 */
pub fn process_common_fields(
    ctx: &GenerateContext,
    body: &mut Vec<String>,
    tbl: &TableInfo,
    operation: i64,
) -> (bool, bool) {
    let mut columns = String::new();
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone());
    if tbc.is_none() {
        return (false, false);
    }
    let tbconf = tbc.unwrap();

    let mut usings = vec![];
    let cols: Vec<crate::schema::ColumnInfo> = ctx.get_table_columns(&tbl_name);
    let fields = parse_column_list(ctx, &tbconf, &cols, &mut columns, false, &mut usings);

    let mut has_su = false;
    let mut has_newcond = false;
    // 处理company_id或company_code
    for fl in fields.clone() {
        if ctx.codegen_conf.multi_tenancy {
            if fl.field_name == "company_id".to_string() {
                body.push(format!("val.company_id = su.user.company_id.clone();"));
                has_su = true;
                has_newcond = true;
            }
            if fl.field_name == "company_code".to_string() {
                body.push(format!("val.company_code = su.user.company_code.clone();"));
                has_su = true;
                has_newcond = true;
            }
        }
        if operation == 1 {
            // for Create
            if fl.field_name == "create_by" {
                if fl.field_type == "String".to_string() {
                    body.push(format!("val.create_by = su.user.username.clone();"));
                    has_su = true;
                    has_newcond = true;
                } else {
                    body.push(format!("val.create_by = su.user.user_id.clone();"));
                    has_su = true;
                    has_newcond = true;
                }
            } else if fl.field_name == "modify_by" {
                if fl.field_type == "String".to_string() {
                    body.push(format!("val.modify_by = su.user.username.clone();"));
                    has_su = true;
                    has_newcond = true;
                } else {
                    body.push(format!("val.modify_by = su.user.user_id.clone();"));
                    has_su = true;
                    has_newcond = true;
                }
            } else if fl.field_name == "create_userid".to_string() {
                body.push(format!("val.create_userid = su.user.user_id.clone();"));
                has_su = true;
                has_newcond = true;
            } else if fl.field_name == "create_user_id".to_string() {
                body.push(format!("val.create_user_id = su.user.user_id.clone();"));
                has_su = true;
                has_newcond = true;
            } else if fl.field_name == "modify_user_id".to_string() {
                body.push(format!("val.modify_user_id = su.user.user_id.clone();"));
                has_su = true;
                has_newcond = true;
            } else if fl.field_name == "modify_userid".to_string() {
                body.push(format!("val.modify_userid = su.user.user_id.clone();"));
                has_su = true;
                has_newcond = true;
            } else if fl.field_name == "modify_username".to_string() {
                body.push(format!("val.modify_username = su.user.nick_name.clone();"));
                has_su = true;
                has_newcond = true;
            } else if fl.field_name == "modify_user_name".to_string() {
                body.push(format!("val.modify_user_name = su.user.nick_name.clone();"));
                has_su = true;
                has_newcond = true;
            } else if fl.field_name == "create_username".to_string() {
                body.push(format!("val.create_username = su.user.nick_name.clone();"));
                has_su = true;
                has_newcond = true;
            } else if fl.field_name == "create_user_name".to_string() {
                body.push(format!("val.create_user_name = su.user.nick_name.clone();"));
                has_su = true;
                has_newcond = true;
            } else if fl.field_name == "create_time".to_string() {
                body.push(format!(
                    "val.create_time = Some(rbatis::DateTimeNative::now());"
                ));
                has_newcond = true;
            } else if fl.field_name == "modify_time".to_string() {
                body.push(format!(
                    "val.modify_time = Some(rbatis::DateTimeNative::now());"
                ));
                has_newcond = true;
            } else if fl.field_name == "update_time".to_string() {
                body.push(format!(
                    "val.update_time = Some(rbatis::DateTimeNative::now());"
                ));
                has_newcond = true;
            } else if fl.field_name == "create_date".to_string() {
                body.push(format!(
                    "val.create_date = Some(rbatis::DateTimeNative::now());"
                ));
                has_newcond = true;
            } else if fl.field_name == "update_date".to_string() {
                body.push(format!(
                    "val.update_date = Some(rbatis::DateTimeNative::now());"
                ));
                has_newcond = true;
            } else if fl.field_name == "modify_date".to_string() {
                body.push(format!(
                    "val.modify_date = Some(rbatis::DateTimeNative::now());"
                ));
                has_newcond = true;
            }
        } else if operation == 2 {
            // for update or delete
            if fl.field_name == "modify_by" {
                if fl.field_type == "String".to_string() {
                    body.push(format!("val.modify_by = su.user.username.clone();"));
                    has_su = true;
                    has_newcond = true;
                } else {
                    body.push(format!("val.modify_by = su.user.user_id.clone();"));
                    has_su = true;
                    has_newcond = true;
                }
            } else if fl.field_name == "modify_user_id".to_string() {
                body.push(format!("val.modify_user_id = su.user.user_id.clone();"));
                has_su = true;
                has_newcond = true;
            } else if fl.field_name == "modify_userid".to_string() {
                body.push(format!("val.modify_userid = su.user.user_id.clone();"));
                has_su = true;
                has_newcond = true;
            } else if fl.field_name == "modify_username".to_string() {
                body.push(format!("val.modify_username = su.user.nick_name.clone();"));
                has_su = true;
                has_newcond = true;
            } else if fl.field_name == "modify_user_name".to_string() {
                body.push(format!("val.modify_user_name = su.user.nick_name.clone();"));
                has_su = true;
                has_newcond = true;
            } else if fl.field_name == "modify_time".to_string() {
                body.push(format!(
                    "val.modify_time = Some(rbatis::DateTimeNative::now());"
                ));
                has_newcond = true;
            } else if fl.field_name == "update_time".to_string() {
                body.push(format!(
                    "val.update_time = Some(rbatis::DateTimeNative::now());"
                ));
                has_newcond = true;
            } else if fl.field_name == "update_date".to_string() {
                body.push(format!(
                    "val.update_date = Some(rbatis::DateTimeNative::now());"
                ));
                has_newcond = true;
            } else if fl.field_name == "modify_date".to_string() {
                body.push(format!(
                    "val.modify_date = Some(rbatis::DateTimeNative::now());"
                ));
                has_newcond = true;
            }
        } else if operation == 3 {
            // for query
            if fl.field_name == "create_by" {
                if fl.field_type == "String".to_string() {
                    body.push(format!("val.create_by = su.user.username.clone();"));
                    has_su = true;
                    has_newcond = true;
                } else {
                    body.push(format!("val.create_by = su.user.user_id.clone();"));
                    has_su = true;
                    has_newcond = true;
                }
            } else if fl.field_name == "create_userid".to_string() {
                body.push(format!("val.create_userid = su.user.user_id.clone();"));
                has_su = true;
                has_newcond = true;
            } else if fl.field_name == "create_user_id".to_string() {
                body.push(format!("val.create_user_id = su.user.user_id.clone();"));
                has_su = true;
                has_newcond = true;
            } else if fl.field_name == "create_username".to_string() {
                body.push(format!("val.create_username = su.user.nick_name.clone();"));
                has_su = true;
                has_newcond = true;
            } else if fl.field_name == "create_user_name".to_string() {
                body.push(format!("val.create_user_name = su.user.nick_name.clone();"));
                has_su = true;
                has_newcond = true;
            }
        }
    }
    return (has_su, has_newcond);
}

pub fn process_enity_common_fields_v2(
    ctx: &GenerateContext,
    body: &mut Vec<String>,
    tbl: &TableInfo,
    operation: i64,
) -> bool {
    let mut columns = String::new();
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone());
    if tbc.is_none() {
        return false;
    }
    let tbconf = tbc.unwrap();
    let mut usings = vec![];
    let cols = ctx.get_table_columns(&tbl_name);
    let fields = parse_column_list(ctx, &tbconf, &cols, &mut columns, false, &mut usings);

    let mut has_su = false;
    // 处理company_id或company_code
    for fl in fields.clone() {
        if ctx.codegen_conf.multi_tenancy {
            if fl.field_name == "company_id".to_string() {
                body.push(format!("company_id: su.user.company_id.clone(),"));
                has_su = true;
            }
            if fl.field_name == "company_code".to_string() {
                body.push(format!("company_code: su.user.company_code.clone(),"));
                has_su = true;
            }
        }
        if operation == 1 {
            // for Create
            if fl.field_name == "create_by" {
                if fl.field_type == "String".to_string() {
                    body.push(format!("create_by: su.user.username.clone(),"));
                    has_su = true;
                } else {
                    body.push(format!("create_by: su.user.user_id,"));
                    has_su = true;
                }
            } else if fl.field_name == "modify_by" {
                if fl.field_type == "String".to_string() {
                    body.push(format!("modify_by: su.user.username.clone(),"));
                    has_su = true;
                } else {
                    body.push(format!("modify_by: su.user.user_id,"));
                    has_su = true;
                }
            } else if fl.field_name == "create_userid".to_string() {
                body.push(format!("create_userid: su.user.user_id,"));
                has_su = true;
            } else if fl.field_name == "create_user_id".to_string() {
                body.push(format!("create_user_id: su.user.user_id,"));
                has_su = true;
            } else if fl.field_name == "modify_user_id".to_string() {
                body.push(format!("modify_user_id: su.user.user_id,"));
                has_su = true;
            } else if fl.field_name == "modify_userid".to_string() {
                body.push(format!("modify_userid: su.user.user_id,"));
                has_su = true;
            } else if fl.field_name == "modify_username".to_string() {
                body.push(format!("modify_username: su.user.nick_name.clone(),"));
                has_su = true;
            } else if fl.field_name == "modify_user_name".to_string() {
                body.push(format!("modify_user_name: su.user.nick_name.clone(),"));
                has_su = true;
            } else if fl.field_name == "create_username".to_string() {
                body.push(format!("create_username: su.user.nick_name.clone(),"));
                has_su = true;
            } else if fl.field_name == "create_user_name".to_string() {
                body.push(format!("create_user_name: su.user.nick_name.clone(),"));
                has_su = true;
            } else if fl.field_name == "create_time".to_string() {
                body.push(format!("create_time: Some(rbatis::DateTimeNative::now()),"));
            } else if fl.field_name == "modify_time".to_string() {
                body.push(format!("modify_time: Some(rbatis::DateTimeNative::now()),"));
            } else if fl.field_name == "update_time".to_string() {
                body.push(format!("update_time: Some(rbatis::DateTimeNative::now()),"));
            } else if fl.field_name == "create_date".to_string() {
                body.push(format!("create_date: Some(rbatis::DateTimeNative::now()),"));
            } else if fl.field_name == "update_date".to_string() {
                body.push(format!("update_date: Some(rbatis::DateTimeNative::now()),"));
            } else if fl.field_name == "modify_date".to_string() {
                body.push(format!("modify_date: Some(rbatis::DateTimeNative::now()),"));
            }
        } else if operation == 2 {
            // for update or delete
            if fl.field_name == "modify_by" {
                if fl.field_type == "String".to_string() {
                    body.push(format!("modify_by: su.user.username.clone(),"));
                    has_su = true;
                } else {
                    body.push(format!("modify_by: su.user.user_id,"));
                    has_su = true;
                }
            } else if fl.field_name == "modify_user_id".to_string() {
                body.push(format!("modify_user_id: su.user.user_id,"));
                has_su = true;
            } else if fl.field_name == "modify_userid".to_string() {
                body.push(format!("modify_userid: su.user.user_id,"));
                has_su = true;
            } else if fl.field_name == "modify_username".to_string() {
                body.push(format!("modify_username: su.user.nick_name.clone(),"));
                has_su = true;
            } else if fl.field_name == "modify_user_name".to_string() {
                body.push(format!("modify_user_name = su.user.nick_name.clone(),"));
                has_su = true;
            } else if fl.field_name == "modify_time".to_string() {
                body.push(format!("modify_time: Some(rbatis::DateTimeNative::now()),"));
            } else if fl.field_name == "update_time".to_string() {
                body.push(format!("update_time: Some(rbatis::DateTimeNative::now()),"));
            } else if fl.field_name == "update_date".to_string() {
                body.push(format!("update_date: Some(rbatis::DateTimeNative::now()),"));
            } else if fl.field_name == "modify_date".to_string() {
                body.push(format!("modify_date: Some(rbatis::DateTimeNative::now()),"));
            }
        } else if operation == 3 {
            // for query
            if fl.field_name == "create_by" {
                if fl.field_type == "String".to_string() {
                    body.push(format!("create_by: su.user.username.clone(),"));
                    has_su = true;
                } else {
                    body.push(format!("create_by: su.user.user_id,"));
                    has_su = true;
                }
            } else if fl.field_name == "create_userid".to_string() {
                body.push(format!("create_userid: su.user.user_id,"));
                has_su = true;
            } else if fl.field_name == "create_user_id".to_string() {
                body.push(format!("create_user_id: su.user.user_id,"));
                has_su = true;
            } else if fl.field_name == "create_username".to_string() {
                body.push(format!("create_username: su.user.nick_name.clone(),"));
                has_su = true;
            } else if fl.field_name == "create_user_name".to_string() {
                body.push(format!("create_user_name: su.user.nick_name.clone(),"));
                has_su = true;
            }
        }
    }
    body.push(format!("..Default::default()"));
    body.push("};".to_string());
    return has_su;
}

pub fn process_tenancy_fields(
    ctx: &GenerateContext,
    body: &mut Vec<String>,
    tbl: &TableInfo,
    _operation: i64,
) -> bool {
    let mut columns = String::new();
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone());
    if tbc.is_none() {
        return false;
    }
    let tbconf = tbc.unwrap();

    let tbl_struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };
    let mut usings = vec![];

    let cols = ctx.get_table_columns(&tbl_name);
    let fields = parse_column_list(ctx, &tbconf, &cols, &mut columns, false, &mut usings);
    // 处理company_id或company_code
    let mut has_su = false;
    for fl in fields.clone() {
        if ctx.codegen_conf.multi_tenancy {
            if fl.field_name == "company_id".to_string() {
                body.push(format!("if tv.company_id != su.user.company_id.clone() {{"));
                body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5040, &\"Not-Found\".to_string()));", tbl_struct_name.clone()));
                body.push(format!("return Ok(HttpResponse::Ok().json(ret));"));
                body.push(format!("}}"));
                has_su = true;
                break;
            }
            if fl.field_name == "company_code".to_string() {
                body.push(format!(
                    "if tv.company_code != su.user.company_code.clone() {{"
                ));
                body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5040, &\"Not-Found\".to_string()));", tbl_struct_name.clone()));
                body.push(format!("return Ok(HttpResponse::Ok().json(ret));"));
                body.push(format!("}}"));
                has_su = true;
                break;
            }
        }
    }
    
    return has_su;
}

/**
 * 生成handler：Update操作
 *
 */
pub fn generate_handler_update_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone()).unwrap();

    let tbl_struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);

    let mut body = vec![];

    body.push(format!("let rb = get_rbatis();"));

    let mut somebody = vec![];

    let (has_su, has_cond) = process_common_fields(ctx, &mut somebody, tbl, 2);
    if has_su {
        params.push(("su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));
        params.push((
            "req".to_string(),
            format!("web::Json<{}>", tbl_struct_name.clone()),
        ));
    } else {
        params.push(("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));
        params.push((
            "req".to_string(),
            format!("web::Json<{}>", tbl_struct_name.clone()),
        ));
    }
    if has_cond {
        body.push(format!("let mut val = req.to_owned();"));
    } else {
        body.push(format!("let val = req.to_owned();"));
    }

    body.append(&mut somebody);

    body.push(format!("match rb.acquire_begin().await {{"));
    body.push(format!("Ok(mut tx) => {{"));
    if tbc.update_seletive {
        body.push(format!("match val.update_selective(&mut tx).await {{"));
    } else {
        body.push(format!("match val.update(&mut tx).await {{"));
    }
    body.push(format!("Ok(_st) => {{"));
    body.push(format!("match tx.commit().await {{"));
    body.push(format!("Ok(_) => {{"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::ok(val));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push("}".to_string());
    body.push("}".to_string());
    body.push("Err(err) => {".to_string());
    body.push(format!("let _ = tx.rollback().await.is_ok();"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("}".to_string());
    body.push("}".to_string());
    body.push("Err(err) => {".to_string());
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("}".to_string());
    let func_name = tbc.api_handler_name.clone() + "_update";

    let url_pattern = format!(
        "{}/{}/update",
        ctx.codegen_conf.api_handler_prefix.clone(),
        tbc.api_handler_name.clone()
    );
    let postmacro = format!("#[post(\"{}\")]", url_pattern.clone());
    RustFunc {
        is_struct_fn: false,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: false,
        is_async: true,
        func_name: func_name,
        return_is_option: false,
        return_is_result: false,
        return_type: Some("Result<HttpResponse>".to_string()),
        params: params,
        bodylines: body,
        macros: vec![postmacro],
        comment: Some(format!("{}更新", tbc.comment.clone())),
        api_method: Some("POST".to_string()),
        api_pattern: Some(url_pattern.clone()),
    }
}

/**
 * 生成handler：Save操作
 *
 * Save操作调用save方法，实际为insert
 */
pub fn generate_handler_save_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone()).unwrap();

    let tbl_struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    let mut body = vec![];

    body.push(format!("let rb = get_rbatis();"));

    let mut somebody = vec![];
    let (has_su, has_cond) = process_common_fields(ctx, &mut somebody, tbl, 1);
    if has_su {
        params.push(("su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));
        params.push((
            "req".to_string(),
            format!("web::Json<{}>", tbl_struct_name.clone()),
        ));
    } else {
        params.push(("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));
        params.push((
            "req".to_string(),
            format!("web::Json<{}>", tbl_struct_name.clone()),
        ));
    }

    if has_cond {
        body.push(format!("let mut val = req.to_owned();"));
    } else {
        body.push(format!("let val = req.to_owned();"));
    }

    body.append(&mut somebody);

    body.push(format!("match rb.acquire_begin().await {{"));
    body.push(format!("Ok(mut tx) => {{"));
    body.push(format!("match val.save(&mut tx).await {{"));
    body.push(format!("Ok(_st) => {{"));
    body.push(format!("match tx.commit().await {{"));
    body.push(format!("Ok(_) => {{"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::ok(val));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));

    body.push("}".to_string());
    body.push("Err(err) => {".to_string());
    body.push(format!("let _ = tx.rollback().await.is_ok();"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("}".to_string());
    body.push("}".to_string());
    body.push("Err(err) => {".to_string());
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("}".to_string());
    let func_name = tbc.api_handler_name.clone() + "_save";

    let url_pattern = format!(
        "{}/{}/create",
        ctx.codegen_conf.api_handler_prefix.clone(),
        tbc.api_handler_name.clone()
    );
    let postmacro = format!("#[post(\"{}\")]", url_pattern.clone());
    RustFunc {
        is_struct_fn: false,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: func_name,
        return_is_option: false,
        return_is_result: false,
        return_type: Some("Result<HttpResponse>".to_string()),
        params: params,
        bodylines: body,
        macros: vec![postmacro],
        comment: Some(format!("{}保存", tbc.comment.clone())),
        api_method: Some("POST".to_string()),
        api_pattern: Some(url_pattern.clone()),
    }
}

/**
 * 生成handler：delete操作
 *
 *
 */
pub fn generate_handler_delete_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone()).unwrap();

    let tbl_struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);

    let mut body = vec![];

    body.push(format!("let rb = get_rbatis();"));

    let mut somebody = vec![];

    let (has_su, has_cond) = process_common_fields(ctx, &mut somebody, tbl, 2);
    if has_su {
        params.push(("su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));
        params.push((
            "req".to_string(),
            format!("web::Json<{}>", tbl_struct_name.clone()),
        ));
    } else {
        params.push(("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));
        params.push((
            "req".to_string(),
            format!("web::Json<{}>", tbl_struct_name.clone()),
        ));
    }

    if has_cond {
        body.push(format!("let mut val = req.to_owned();"));
    } else {
        body.push(format!("let val = req.to_owned();"));
    }

    body.append(&mut somebody);

    body.push(format!("match rb.acquire_begin().await {{"));
    body.push(format!("Ok(mut tx) => {{"));
    body.push(format!("match val.remove(&mut tx).await {{"));

    body.push(format!("Ok(_st) => {{"));
    body.push(format!("match tx.commit().await {{"));
    body.push(format!("Ok(_) => {{"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::ok(val));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push("}".to_string());
    body.push("Err(err) => {".to_string());
    body.push(format!("let _ = tx.rollback().await.is_ok();"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("}".to_string());
    body.push("}".to_string());
    body.push("Err(err) => {".to_string());
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("}".to_string());
    let func_name = tbc.api_handler_name.clone() + "_delete";

    let url_pattern = format!(
        "{}/{}/delete",
        ctx.codegen_conf.api_handler_prefix.clone(),
        tbc.api_handler_name.clone()
    );
    let postmacro = format!("#[post(\"{}\")]", url_pattern.clone());

    RustFunc {
        is_struct_fn: false,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: func_name,
        return_is_option: false,
        return_is_result: false,
        return_type: Some("Result<HttpResponse>".to_string()),
        params: params,
        bodylines: body,
        macros: vec![postmacro],
        comment: Some(format!("{}删除", tbc.comment.clone())),
        api_method: Some("POST".to_string()),
        api_pattern: Some(url_pattern.clone()),
    }
}

/**
 * 生成handler：delete操作
 *
 *
 */
pub fn generate_handler_delete_ids_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone()).unwrap();

    let tbl_struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };

    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    // params.push(("req".to_string(), format!("web::Json<{}>", tbl_struct_name.clone())));
    for col in pkcols.clone() {
        params.push((
            "req".to_string(),
            format!(
                "web::Json<Vec<{}>>",
                parse_data_type_as_rust_type(
                    &col.data_type.clone().unwrap_or_default().to_lowercase()
                )
            ),
        ));
    }

    let mut body = vec![];

    body.push(format!("let rb = get_rbatis();"));
    body.push(format!("let ids = req.as_slice();"));

    body.push(format!("let val = {} {{", tbl_struct_name.clone()));
    let has_su = process_enity_common_fields_v2(ctx, &mut body, tbl, 2);

    if ctx.codegen_conf.multi_tenancy {
        if has_su {
            params.insert(
                0,
                ("su".to_string(), "SystemUser<ChimesUserInfo>".to_string()),
            );
        } else {
            params.insert(
                0,
                ("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()),
            );
        }
    }

    body.push(format!("match rb.acquire_begin().await {{"));
    body.push(format!("Ok(mut tx) => {{"));
    if ctx.codegen_conf.multi_tenancy {
        body.push(format!(
            "match {}::remove_ids(&mut tx, ids, &val).await {{",
            tbl_struct_name.clone()
        ));
    } else {
        body.push(format!(
            "match {}::remove_ids(&mut tx, ids).await {{",
            tbl_struct_name.clone()
        ));
    }

    body.push(format!("Ok(st) => {{"));
    body.push(format!("match tx.commit().await {{"));
    body.push(format!("Ok(_) => {{"));
    body.push(format!(
        "let ret: web::Json<ApiResult<u64>> = web::Json(ApiResult::ok(st));"
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));

    body.push("}".to_string());
    body.push("Err(err) => {".to_string());
    body.push(format!("let _ = tx.rollback().await.is_ok();"));
    body.push(format!(
        "let ret: web::Json<ApiResult<u64>> = web::Json(ApiResult::error(5010, &err.to_string()));"
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("}".to_string());
    body.push("}".to_string());
    body.push("Err(err) => {".to_string());
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("}".to_string());
    let func_name = tbc.api_handler_name.clone() + "_delete_ids";

    let url_pattern = format!(
        "{}/{}/delete_ids",
        ctx.codegen_conf.api_handler_prefix.clone(),
        tbc.api_handler_name.clone()
    );
    let postmacro = format!("#[post(\"{}\")]", url_pattern.clone());
    RustFunc {
        is_struct_fn: false,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: func_name,
        return_is_option: false,
        return_is_result: false,
        return_type: Some("Result<HttpResponse>".to_string()),
        params: params,
        bodylines: body,
        macros: vec![postmacro],
        comment: Some(format!("{}批量删除", tbc.comment.clone())),
        api_method: Some("POST".to_string()),
        api_pattern: Some(url_pattern.clone()),
    }
}

/**
 * 生成Query List查询
 *
 *
 */
pub fn generate_handler_query_list_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone()).unwrap();

    let tbl_struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);

    if tbc.using_common_search {
        params.push(("req".to_string(), format!("web::Json<CommonSearch>")));
    } else {
        if tbc.generate_param_struct {
            params.push((
                "req".to_string(),
                format!("web::Json<{}Query>", tbl_struct_name.clone()),
            ));
        } else {
            params.push((
                "req".to_string(),
                format!("web::Json<{}>", tbl_struct_name.clone()),
            ));
        }
    }

    let mut body = vec![];

    body.push(format!("let rb = get_rbatis();"));

    let mut somebody = vec![];

    let (has_su, has_cond) = process_common_fields(ctx, &mut somebody, tbl, 4);
    if has_su {
        params.insert(
            0,
            ("su".to_string(), "SystemUser<ChimesUserInfo>".to_string()),
        );
    } else {
        params.insert(
            0,
            ("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()),
        );
    }

    if !tbc.using_common_search {
        if has_cond {
            body.push(format!("let mut val = req.to_owned();"));
        } else {
            body.push(format!("let val = req.to_owned();"));
        }
    }

    body.append(&mut somebody);

    if tbc.using_common_search {
        body.push(format!(
            "match {}::common_query_list(rb, req.to_owned()).await {{",
            tbl_struct_name.clone()
        ));
    } else {
        body.push(format!("match val.query_list(rb).await {{"));
    }
    body.push(format!("Ok(st) => {{"));
    if tbc.tree_parent_field.is_some() {
        body.push(format!("let mtts:Vec<{}Value> = st.into_iter().map(|f| {}Value::from_entity_with(&f, true, &vec![])).collect();", tbl_struct_name.clone(), tbl_struct_name.clone()));
        body.push(format!(
            "let ret: web::Json<ApiResult<Vec<{}Value>>> = web::Json(ApiResult::ok(mtts));",
            tbl_struct_name.clone()
        ));
    } else {
        body.push(format!(
            "let ret: web::Json<ApiResult<Vec<{}>>> = web::Json(ApiResult::ok(st));",
            tbl_struct_name.clone()
        ));
    }
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));

    body.push("}".to_string());
    body.push("Err(err) => {".to_string());
    if tbc.tree_parent_field.is_some() {
        body.push(format!("let ret: web::Json<ApiResult<Vec<{}Value>>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_struct_name.clone()));
    } else {
        body.push(format!("let ret: web::Json<ApiResult<Vec<{}>>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_struct_name.clone()));
    }
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("}".to_string());
    let func_name = tbc.api_handler_name.clone() + "_search";

    let url_pattern = format!(
        "{}/{}/search",
        ctx.codegen_conf.api_handler_prefix.clone(),
        tbc.api_handler_name.clone()
    );
    let postmacro = format!("#[post(\"{}\")]", url_pattern.clone());
    RustFunc {
        is_struct_fn: false,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: func_name,
        return_is_option: false,
        return_is_result: false,
        return_type: Some("Result<HttpResponse>".to_string()),
        params: params,
        bodylines: body,
        macros: vec![postmacro],
        comment: Some(format!("{}查询", tbc.comment.clone())),
        api_method: Some("POST".to_string()),
        api_pattern: Some(url_pattern.clone()),
    }
}

/**
 * 生成Query Paged查询
 *
 *
 */
pub fn generate_handler_query_page_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone()).unwrap();

    let tbl_struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);

    if tbc.using_common_search {
        params.push(("req".to_string(), format!("web::Json<CommonSearch>")));
    } else {
        if tbc.generate_param_struct {
            params.push((
                "req".to_string(),
                format!("web::Json<{}Query>", tbl_struct_name.clone()),
            ));
        } else {
            params.push((
                "req".to_string(),
                format!("web::Json<{}>", tbl_struct_name.clone()),
            ));
        }
    }

    params.push(("path_param".to_string(), format!("web::Path<(u64, u64)>")));

    let mut body = vec![];

    body.push(format!("let rb = get_rbatis();"));

    let mut somebody = vec![];

    let (has_su, has_cond) = process_common_fields(ctx, &mut somebody, tbl, 4);
    if has_su {
        params.insert(
            0,
            ("su".to_string(), "SystemUser<ChimesUserInfo>".to_string()),
        );
    } else {
        params.insert(
            0,
            ("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()),
        );
    }

    if !tbc.using_common_search {
        if has_cond {
            body.push(format!("let mut val = req.to_owned();"));
        } else {
            body.push(format!("let val = req.to_owned();"));
        }
    }
    body.push(format!("let (current, size) = path_param.into_inner();"));

    body.append(&mut somebody);

    if tbc.using_common_search {
        body.push(format!(
            "match {}::common_query_paged(rb, req.to_owned(), current, size).await {{",
            tbl_struct_name.clone()
        ));
    } else {
        body.push(format!("match val.query_paged(rb, current, size).await {{"));
    }
    body.push(format!("Ok(st) => {{"));
    if tbc.tree_parent_field.is_some() {
        body.push(format!("let mtts:Vec<{}Value> = st.records.into_iter().map(|f| {}Value::from_entity_with(&f, true, &vec![])).collect();", tbl_struct_name.clone(), tbl_struct_name.clone()));
        body.push(format!(
            "let mut newpage = Page::new_total(st.page_no, st.page_size, st.total);"
        ));
        body.push(format!("newpage.records = mtts;"));
        body.push(format!(
            "let ret: web::Json<ApiResult<Page<{}Value>>> = web::Json(ApiResult::ok(newpage));",
            tbl_struct_name.clone()
        ));
    } else {
        body.push(format!(
            "let ret: web::Json<ApiResult<Page<{}>>> = web::Json(ApiResult::ok(st));",
            tbl_struct_name.clone()
        ));
    }
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("Err(err) => {".to_string());
    if tbc.tree_parent_field.is_some() {
        body.push(format!("let ret: web::Json<ApiResult<Page<{}Value>>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_struct_name.clone()));
    } else {
        body.push(format!("let ret: web::Json<ApiResult<Page<{}>>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_struct_name.clone()));
    }
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("}".to_string());
    let func_name = tbc.api_handler_name.clone() + "_paged";

    let url_pattern = format!(
        "{}/{}/paged/{{current}}/{{size}}",
        ctx.codegen_conf.api_handler_prefix.clone(),
        tbc.api_handler_name.clone()
    );
    let postmacro = format!("#[post(\"{}\")]", url_pattern.clone());

    RustFunc {
        is_struct_fn: false,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: func_name,
        return_is_option: false,
        return_is_result: false,
        return_type: Some("Result<HttpResponse>".to_string()),
        params: params,
        bodylines: body,
        macros: vec![postmacro],
        comment: Some(format!("{}分页查询", tbc.comment.clone())),
        api_method: Some("POST".to_string()),
        api_pattern: Some(url_pattern.clone()),
    }
}

/**
 * 生成Query List查询
 *
 *
 */
pub fn generate_handler_query_list_for_composite(
    ctx: &GenerateContext,
    tbl: &TableInfo,
    comps: &Option<RustStruct>,
) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone()).unwrap();

    let tbl_struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    // params.push(("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));

    if comps.is_some() {
        params.push((
            "req".to_string(),
            format!("web::Json<{}Present>", tbl_struct_name.clone()),
        ));
    }

    let mut body = vec![];

    body.push(format!("let rb = get_rbatis();"));

    let mut somebody = vec![];

    let (has_su, has_cond) = process_common_fields(ctx, &mut somebody, tbl, 4);
    if has_su {
        params.insert(
            0,
            ("su".to_string(), "SystemUser<ChimesUserInfo>".to_string()),
        );
    } else {
        params.insert(
            0,
            ("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()),
        );
    }

    if has_cond {
        body.push(format!("let mut val = req.to_owned();"));
    } else {
        body.push(format!("let val = req.to_owned();"));
    }

    body.append(&mut somebody);

    body.push(format!(
        "match {}Present::query_list(rb, &val).await {{",
        tbl_struct_name.clone()
    ));

    body.push(format!("Ok(st) => {{"));
    if tbc.tree_parent_field.is_some() {
        body.push(format!("let mtts:Vec<{}Value> = st.into_iter().map(|f| {}Value::from_entity_with(&f, true, &vec![])).collect();", tbl_struct_name.clone(), tbl_struct_name.clone()));
        body.push(format!(
            "let ret: web::Json<ApiResult<Vec<{}Value>>> = web::Json(ApiResult::ok(mtts));",
            tbl_struct_name.clone()
        ));
    } else {
        body.push(format!(
            "let ret: web::Json<ApiResult<Vec<{}Present>>> = web::Json(ApiResult::ok(st));",
            tbl_struct_name.clone()
        ));
    }
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));

    body.push("}".to_string());
    body.push("Err(err) => {".to_string());
    if tbc.tree_parent_field.is_some() {
        body.push(format!("let ret: web::Json<ApiResult<Vec<{}Value>>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_struct_name.clone()));
    } else {
        body.push(format!("let ret: web::Json<ApiResult<Vec<{}Present>>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_struct_name.clone()));
    }
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("}".to_string());
    let func_name = tbc.api_handler_name.clone() + "_search_present";

    let url_pattern = format!(
        "{}/{}/present/search",
        ctx.codegen_conf.api_handler_prefix.clone(),
        tbc.api_handler_name.clone()
    );
    let postmacro = format!("#[post(\"{}\")]", url_pattern.clone());
    RustFunc {
        is_struct_fn: false,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: func_name,
        return_is_option: false,
        return_is_result: false,
        return_type: Some("Result<HttpResponse>".to_string()),
        params: params,
        bodylines: body,
        macros: vec![postmacro],
        comment: Some(format!("{}查询", tbc.comment.clone())),
        api_method: Some("POST".to_string()),
        api_pattern: Some(url_pattern.clone()),
    }
}

/**
 * 生成Query Paged查询
 *
 *
 */
pub fn generate_handler_query_page_for_composite(
    ctx: &GenerateContext,
    tbl: &TableInfo,
    comps: &Option<RustStruct>,
) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone()).unwrap();

    let tbl_struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    // params.push(("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));

    if comps.is_some() {
        params.push((
            "req".to_string(),
            format!("web::Json<{}Present>", tbl_struct_name.clone()),
        ));
    }

    params.push(("path_param".to_string(), format!("web::Path<(u64, u64)>")));

    let mut body = vec![];

    body.push(format!("let rb = get_rbatis();"));

    let mut somebody = vec![];

    let (has_su, has_cond) = process_common_fields(ctx, &mut somebody, tbl, 4);
    if has_su {
        params.insert(
            0,
            ("su".to_string(), "SystemUser<ChimesUserInfo>".to_string()),
        );
    } else {
        params.insert(
            0,
            ("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()),
        );
    }

    if has_cond {
        body.push(format!("let mut val = req.to_owned();"));
    } else {
        body.push(format!("let val = req.to_owned();"));
    }

    body.append(&mut somebody);

    body.push(format!("let (current, size) = path_param.into_inner();"));

    body.push(format!(
        "match {}Present::query_paged(rb, &val, current, size).await {{",
        tbl_struct_name.clone()
    ));

    body.push(format!("Ok(st) => {{"));
    if tbc.tree_parent_field.is_some() {
        body.push(format!("let mtts:Vec<{}Value> = st.records.into_iter().map(|f| {}Value::from_entity_with(&f, true, &vec![])).collect();", tbl_struct_name.clone(), tbl_struct_name.clone()));
        body.push(format!(
            "let mut newpage = Page::new_total(st.page_no, st.page_size, st.total);"
        ));
        body.push(format!("newpage.records = mtts;"));
        body.push(format!(
            "let ret: web::Json<ApiResult<Page<{}Value>>> = web::Json(ApiResult::ok(newpage));",
            tbl_struct_name.clone()
        ));
    } else {
        body.push(format!(
            "let ret: web::Json<ApiResult<Page<{}Present>>> = web::Json(ApiResult::ok(st));",
            tbl_struct_name.clone()
        ));
    }
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("Err(err) => {".to_string());
    if tbc.tree_parent_field.is_some() {
        body.push(format!("let ret: web::Json<ApiResult<Page<{}Value>>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_struct_name.clone()));
    } else {
        body.push(format!("let ret: web::Json<ApiResult<Page<{}Present>>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_struct_name.clone()));
    }
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("}".to_string());
    let func_name = tbc.api_handler_name.clone() + "_paged_present";

    let url_pattern = format!(
        "{}/{}/present/paged/{{current}}/{{size}}",
        ctx.codegen_conf.api_handler_prefix.clone(),
        tbc.api_handler_name.clone()
    );
    let postmacro = format!("#[post(\"{}\")]", url_pattern.clone());

    RustFunc {
        is_struct_fn: false,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: func_name,
        return_is_option: false,
        return_is_result: false,
        return_type: Some("Result<HttpResponse>".to_string()),
        params: params,
        bodylines: body,
        macros: vec![postmacro],
        comment: Some(format!("{}分页查询", tbc.comment.clone())),
        api_method: Some("POST".to_string()),
        api_pattern: Some(url_pattern.clone()),
    }
}

/**
 * 生成Tree方法的查询List查询
 */
pub fn generate_handler_query_tree_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone()).unwrap();

    let tbl_struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };

    let treecol = ctx
        .find_table_column(&tbl_name.clone(), &tbc.tree_parent_field.unwrap())
        .unwrap();
    let treecol_type = parse_data_type_as_rust_type(&treecol.data_type.unwrap_or_default());

    let mut params = Vec::new();

    params.push(("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("req".to_string(), format!("HttpRequest")));

    let mut body = vec![];

    body.push(format!("let rb = get_rbatis();"));

    body.push(format!("let query = req.query_string();"));
    body.push(format!("let dic = crate::utils::parse_query(query);"));
    body.push(format!(
        "let val = crate::utils::get_hash_value(&dic, \"pid\");"
    ));

    if treecol_type == "String" {
        body.push(format!("let valopt = if val.is_empty() {{"));
        body.push(format!("None"));
        body.push(format!("}} else {{"));
        body.push(format!("Some(val)"));
        body.push(format!("}};"));
    } else {
        // actuall the should be i64, u64 etc some number type
        body.push(format!(
            "let valopt = match val.parse::<{}>() {{",
            treecol_type
        ));
        body.push(format!("Ok(tv) => Some(tv),"));
        body.push(format!("Err(_) => None"));
        body.push(format!("}};"));
    }

    body.push(format!(
        "match {}::query_tree(rb, &valopt).await {{",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(st) => {{"));
    body.push(format!("let mtts:Vec<{}Value> = st.into_iter().map(|f| {}Value::from_entity_with(&f, true, &vec![])).collect();", tbl_struct_name.clone(), tbl_struct_name.clone()));
    body.push(format!(
        "let ret: web::Json<ApiResult<Vec<{}Value>>> = web::Json(ApiResult::ok(mtts));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("Err(err) => {".to_string());
    body.push(format!("let ret: web::Json<ApiResult<Vec<{}Value>>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_struct_name.clone()));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("}".to_string());
    let func_name = tbc.api_handler_name.clone() + "_tree";

    let url_pattern = format!(
        "{}/{}/tree",
        ctx.codegen_conf.api_handler_prefix.clone(),
        tbc.api_handler_name.clone()
    );
    let postmacro = format!("#[get(\"{}\")]", url_pattern.clone());

    RustFunc {
        is_struct_fn: false,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: func_name,
        return_is_option: false,
        return_is_result: false,
        return_type: Some("Result<HttpResponse>".to_string()),
        params: params,
        bodylines: body,
        macros: vec![postmacro],
        comment: Some(format!("{}树形查询", tbc.comment.clone())),
        api_method: Some("GET".to_string()),
        api_pattern: Some(url_pattern.clone()),
    }
}

/**
 * 生成handler：delete操作
 *
 *
 */
pub fn generate_handler_get_for_struct(ctx: &GenerateContext, tbl: &TableInfo) -> RustFunc {
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone()).unwrap();

    let tbl_struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };

    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut param_text = String::new();
    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    // params.push(("req".to_string(), format!("web::Path<{}>", tbl_struct_name.clone())));

    // params.push(("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));

    for col in pkcols.clone() {
        let dt = parse_data_type_as_rust_type(&col.data_type.unwrap_or_default());
        let colname = col.column_name.unwrap_or_default().to_lowercase();
        params.push((
            format!("{}_req", colname.clone()),
            format!("web::Path<{}>", dt),
        ));
        param_text.push_str(format!(", &{}", colname.as_str()).as_str());
    }
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);

    let mut body = vec![];

    body.push(format!("let rb = get_rbatis();"));
    for col in pkcols.clone() {
        let colname = col.column_name.unwrap_or_default().to_lowercase();
        body.push(format!(
            "let {} = {}_req.to_owned();",
            colname.clone(),
            colname.clone()
        ));
    }

    body.push(format!(
        "match {}::from_id(rb{}).await {{",
        tbl_struct_name.clone(),
        param_text.clone()
    ));

    body.push(format!("Ok(st) => {{"));
    body.push(format!("match st {{"));
    body.push(format!("Some(tv) => {{"));

    let has_su = if ctx.codegen_conf.multi_tenancy {
        process_tenancy_fields(ctx, &mut body, tbl, 0)
    } else {
        false
    };
    if has_su {
        params.insert(
            0,
            ("su".to_string(), "SystemUser<ChimesUserInfo>".to_string()),
        );
    } else {
        params.insert(
            0,
            ("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()),
        );
    }

    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::ok(tv));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push(format!("None => {{"));
    body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5040, &\"Not-Found\".to_string()));", tbl_struct_name.clone()));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("}".to_string());
    body.push("}".to_string());
    body.push("Err(err) => {".to_string());
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));",
        tbl_struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("}".to_string());
    let func_name = tbc.api_handler_name.clone() + "_get";

    let url_pattern = format!(
        "{}/{}/get/{{id}}",
        ctx.codegen_conf.api_handler_prefix.clone(),
        tbc.api_handler_name.clone()
    );

    let postmacro = format!("#[get(\"{}\")]", url_pattern.clone());
    RustFunc {
        is_struct_fn: false,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: func_name,
        return_is_option: false,
        return_is_result: false,
        return_type: Some("Result<HttpResponse>".to_string()),
        params: params,
        bodylines: body,
        macros: vec![postmacro],
        comment: Some(format!("{}获取", tbc.comment.clone())),
        api_method: Some("GET".to_string()),
        api_pattern: Some(url_pattern.clone()),
    }
}
