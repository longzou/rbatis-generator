use change_case::{pascal_case, snake_case};
use std::collections::{HashMap, HashSet};

use super::{
    is_copied_type, parse_composite_column_list, CodeGenerator, RelationForm, RelationTable, RustFileImpl
};
use crate::codegen::{
    parse_column_list, parse_data_type_as_rust_type, GenerateContext, RustFunc, RustStruct,
    RustStructField,
};
use crate::config::{safe_struct_field_name, RelationConfig, TEMPLATES};
use crate::schema::TableInfo;
use serde_json::{json, Value};
use substring::Substring;
use tera::Context;

/**
 * 针对实体对象
 * 对其结构进行初始化操作
 * 1、初始化company_id, company_code
 * 2、初始化modify_by, modify_user_id, modify_user_name, create_by, create_user_id, create_user_name
 * 3、初始化create_time, update_time
 */
pub fn process_detail_common_fields(
    ctx: &GenerateContext,
    vbn: &str,
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
    let mut usings = vec![];
    let tbconf = tbc.unwrap();
    let cols = ctx.get_table_columns(&tbl_name);
    let fields = parse_column_list(ctx, &tbconf, &cols, &mut columns, false, &mut usings);

    let mut has_su = false;
    // 处理company_id或company_code
    for fl in fields.clone() {
        if ctx.codegen_conf.multi_tenancy {
            if operation != 5 {
                if fl.field_name == "company_id".to_string() {
                    body.push(format!("{}.company_id = su.user.company_id.clone();", vbn));
                    has_su = true;
                }
                if fl.field_name == "company_code".to_string() {
                    body.push(format!(
                        "{}.company_code = su.user.company_code.clone();",
                        vbn
                    ));
                    has_su = true;
                }
            }
        }
        if operation == 1 {
            // for Create
            if fl.field_name == "create_by" {
                if fl.field_type == "String".to_string() {
                    body.push(format!("{}.create_by = su.user.username.clone();", vbn));
                } else {
                    body.push(format!("{}.create_by = su.user.user_id.clone();", vbn));
                }
                has_su = true;
            } else if fl.field_name == "modify_by" {
                if fl.field_type == "String".to_string() {
                    body.push(format!("{}.modify_by = su.user.username.clone();", vbn));
                } else {
                    body.push(format!("{}.modify_by = su.user.user_id.clone();", vbn));
                }
                has_su = true;
            } else if fl.field_name == "create_userid".to_string() {
                body.push(format!("{}.create_userid = su.user.user_id.clone();", vbn));
                has_su = true;
            } else if fl.field_name == "create_user_id".to_string() {
                body.push(format!("{}.create_user_id = su.user.user_id.clone();", vbn));
                has_su = true;
            } else if fl.field_name == "modify_user_id".to_string() {
                body.push(format!("{}.modify_user_id = su.user.user_id.clone();", vbn));
                has_su = true;
            } else if fl.field_name == "modify_userid".to_string() {
                body.push(format!("{}.modify_userid = su.user.user_id.clone();", vbn));
                has_su = true;
            } else if fl.field_name == "modify_username".to_string() {
                body.push(format!(
                    "{}.modify_username = su.user.nick_name.clone();",
                    vbn
                ));
                has_su = true;
            } else if fl.field_name == "modify_user_name".to_string() {
                body.push(format!(
                    "{}.modify_user_name = su.user.nick_name.clone();",
                    vbn
                ));
                has_su = true;
            } else if fl.field_name == "create_username".to_string() {
                body.push(format!(
                    "{}.create_username = su.user.nick_name.clone();",
                    vbn
                ));
                has_su = true;
            } else if fl.field_name == "create_user_name".to_string() {
                body.push(format!(
                    "{}.create_user_name = su.user.nick_name.clone();",
                    vbn
                ));
                has_su = true;
            } else if fl.field_name == "create_time".to_string() {
                body.push(format!(
                    "{}.create_time = Some(rbatis::DateTimeNative::now());",
                    vbn
                ));
            } else if fl.field_name == "modify_time".to_string() {
                body.push(format!(
                    "{}.modify_time = Some(rbatis::DateTimeNative::now());",
                    vbn
                ));
            } else if fl.field_name == "update_time".to_string() {
                body.push(format!(
                    "{}.update_time = Some(rbatis::DateTimeNative::now());",
                    vbn
                ));
            } else if fl.field_name == "create_date".to_string() {
                body.push(format!(
                    "{}.create_date = Some(rbatis::DateTimeNative::now());",
                    vbn
                ));
            } else if fl.field_name == "update_date".to_string() {
                body.push(format!(
                    "{}.update_date = Some(rbatis::DateTimeNative::now());",
                    vbn
                ));
            } else if fl.field_name == "modify_date".to_string() {
                body.push(format!(
                    "{}.modify_date = Some(rbatis::DateTimeNative::now());",
                    vbn
                ));
            }
        } else if operation == 2 || operation == 5 {
            // for update or delete
            if fl.field_name == "modify_by" {
                if fl.field_type == "String".to_string() {
                    body.push(format!("{}.modify_by = su.user.username.clone();", vbn));
                    has_su = true;
                } else {
                    body.push(format!("{}.modify_by = su.user.user_id.clone();", vbn));
                    has_su = true;
                }
            } else if fl.field_name == "modify_user_id".to_string() {
                body.push(format!("{}.modify_user_id = su.user.user_id.clone();", vbn));
                has_su = true;
            } else if fl.field_name == "modify_userid".to_string() {
                body.push(format!("{}.modify_userid = su.user.user_id.clone();", vbn));
                has_su = true;
            } else if fl.field_name == "modify_username".to_string() {
                body.push(format!(
                    "{}.modify_username = su.user.nick_name.clone();",
                    vbn
                ));
                has_su = true;
            } else if fl.field_name == "modify_user_name".to_string() {
                body.push(format!(
                    "{}.modify_user_name = su.user.nick_name.clone();",
                    vbn
                ));
                has_su = true;
            } else if fl.field_name == "modify_time".to_string() {
                body.push(format!(
                    "{}.modify_time = Some(rbatis::DateTimeNative::now());",
                    vbn
                ));
            } else if fl.field_name == "update_time".to_string() {
                body.push(format!(
                    "{}.update_time = Some(rbatis::DateTimeNative::now());",
                    vbn
                ));
            } else if fl.field_name == "update_date".to_string() {
                body.push(format!(
                    "{}.update_date = Some(rbatis::DateTimeNative::now());",
                    vbn
                ));
            } else if fl.field_name == "modify_date".to_string() {
                body.push(format!(
                    "{}.modify_date = Some(rbatis::DateTimeNative::now());",
                    vbn
                ));
            }
        } else if operation == 3 {
            // for query
            if fl.field_name == "create_by" {
                if fl.field_type == "String".to_string() {
                    body.push(format!("{}.create_by = su.user.username.clone();", vbn));
                    has_su = true;
                } else {
                    body.push(format!("{}.create_by = su.user.user_id.clone();", vbn));
                    has_su = true;
                }
            } else if fl.field_name == "create_userid".to_string() {
                body.push(format!("{}.create_userid = su.user.user_id.clone();", vbn));
                has_su = true;
            } else if fl.field_name == "create_user_id".to_string() {
                body.push(format!("{}.create_user_id = su.user.user_id.clone();", vbn));
                has_su = true;
            } else if fl.field_name == "create_username".to_string() {
                body.push(format!(
                    "{}.create_username = su.user.nick_name.clone();",
                    vbn
                ));
                has_su = true;
            } else if fl.field_name == "create_user_name".to_string() {
                body.push(format!(
                    "{}.create_user_name = su.user.nick_name.clone();",
                    vbn
                ));
                has_su = true;
            }
        }
    }
    has_su
}

pub fn process_detail_common_fields_v2(
    ctx: &GenerateContext,
    _vbn: &str,
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
            if operation != 5 {
                if fl.field_name == "company_id".to_string() {
                    body.push(format!("company_id: su.user.company_id.clone(),"));
                    has_su = true;
                }
                if fl.field_name == "company_code".to_string() {
                    body.push(format!("company_code: su.user.company_code.clone(),"));
                    has_su = true;
                }
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
                body.push(format!("create_userid: su.user.user_id.clone(),"));
                has_su = true;
            } else if fl.field_name == "create_user_id".to_string() {
                body.push(format!("create_user_id: su.user.user_id.clone(),"));
                has_su = true;
            } else if fl.field_name == "modify_user_id".to_string() {
                body.push(format!("modify_user_id: su.user.user_id.clone(),"));
                has_su = true;
            } else if fl.field_name == "modify_userid".to_string() {
                body.push(format!("modify_userid: su.user.user_id.clone(),"));
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
                // has_su = false;
            } else if fl.field_name == "modify_time".to_string() {
                body.push(format!("modify_time: Some(rbatis::DateTimeNative::now()),"));
                // has_su = false;
            } else if fl.field_name == "update_time".to_string() {
                body.push(format!("update_time: Some(rbatis::DateTimeNative::now()),"));
            } else if fl.field_name == "create_date".to_string() {
                body.push(format!("create_date: Some(rbatis::DateTimeNative::now()),"));
            } else if fl.field_name == "update_date".to_string() {
                body.push(format!(
                    "update_date: = Some(rbatis::DateTimeNative::now()),"
                ));
            } else if fl.field_name == "modify_date".to_string() {
                body.push(format!("modify_date: Some(rbatis::DateTimeNative::now()),"));
            }
        } else if operation == 2 || operation == 5 {
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
                body.push(format!("modify_user_name: su.user.nick_name.clone(),"));
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
            } else if fl.field_name == "create_user_id".to_string() {
                body.push(format!("create_user_id: su.user.user_id,"));
            } else if fl.field_name == "create_username".to_string() {
                body.push(format!("create_username: su.user.nick_name.clone(),"));
                has_su = true;
            } else if fl.field_name == "create_user_name".to_string() {
                body.push(format!("create_user_name: su.user.nick_name.clone(),"));
                has_su = true;
            }
        }
    }
    return has_su;
}

pub fn process_detail_tenancy_fields(
    ctx: &GenerateContext,
    body: &mut Vec<String>,
    tbl: &TableInfo,
    struct_name: &String,
    _operation: i64,
) -> bool {
    let mut columns = String::new();
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    let tbc = ctx.get_table_conf(&tbl_name.clone());
    if tbc.is_none() {
        return false;
    }
    let tbconf = tbc.unwrap();
    let mut has_su = false;

    let _tbl_struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
        Some(t) => t,
        None => pascal_case(tbl_name.clone().as_str()),
    };

    let mut usings = vec![];
    let cols = ctx.get_table_columns(&tbl_name);
    let fields = parse_column_list(ctx, &tbconf, &cols, &mut columns, false, &mut usings);
    // 处理company_id或company_code
    for fl in fields.clone() {
        if ctx.codegen_conf.multi_tenancy {
            if fl.field_name == "company_id".to_string() {
                body.push(format!("if tv.company_id != su.user.company_id.clone() {{"));
                body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5040, &\"Not-Found\".to_string()));", struct_name.clone()));
                body.push(format!("return Ok(HttpResponse::Ok().json(ret));"));
                body.push(format!("}}"));
                has_su = true;
                break;
            }
            if fl.field_name == "company_code".to_string() {
                body.push(format!(
                    "if tv.company_code != su.user.company_code.clone() {{"
                ));
                body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5040, &\"Not-Found\".to_string()));", struct_name.clone()));
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
 * 解析关系并生成文件
 */
pub fn parse_relation_as_file(ctx: &GenerateContext, rel: &RelationConfig) -> Option<RustFileImpl> {
    let st = parse_relation_as_struct(ctx, rel);
    let tbc = ctx.get_table_conf(&rel.major_table.clone());
    match tbc {
        Some(tbconf) => {
            let mut usinglist = CodeGenerator::get_default_entity_using(
                ctx,
                false,
                tbconf.with_attachment,
                super::CodeModelType::Relation
            );
            usinglist.push(format!("chimes_rust::{{SystemUser, ChimesUserInfo}}"));
            usinglist.push(format!("crate::entity::{}", tbconf.struct_name));

            for rl in rel.one_to_one.clone() {
                match ctx.get_table_conf(&rl.table_name.clone().unwrap_or_default()) {
                    Some(mt) => {
                        usinglist.push(format!("crate::entity::{}", mt.struct_name));
                    }
                    None => {}
                }
            }

            for rl in rel.one_to_many.clone() {
                match ctx.get_table_conf(&rl.table_name.clone().unwrap_or_default()) {
                    Some(mt) => {
                        usinglist.push(format!("crate::entity::{}", mt.struct_name));
                    }
                    None => {}
                }
                if rl.middle_table.is_some() {
                    match ctx.get_table_conf(&rl.middle_table.clone().unwrap_or_default()) {
                        Some(mt) => {
                            usinglist.push(format!("crate::entity::{}", mt.struct_name));
                        }
                        None => {}
                    }
                }
            }

            let rfi = RustFileImpl {
                file_name: snake_case(rel.struct_name.clone().as_str()) + ".rs",
                mod_name: "entity".to_string(),
                caretlist: vec![],
                usinglist: usinglist,
                structlist: vec![st],
                funclist: vec![],
            };
            Some(rfi)
        }
        None => None,
    }
}

/**
 * 解析关系并生成文件
 */
pub fn parse_relation_handlers_as_file(
    ctx: &mut GenerateContext,
    rel: &RelationConfig,
) -> Option<RustFileImpl> {
    let st = parse_relation_as_struct(ctx, rel);
    let tbc = ctx.get_table_conf(&rel.major_table.clone());

    match tbc {
        Some(tbconf) => {
            let mut usinglist = CodeGenerator::get_default_handler_using(ctx, tbconf.page_query, tbconf.using_common_search);
            usinglist.push(format!("crate::entity::{}", st.struct_name));
            // usinglist.push(format!("crate::entity::{}", tbconf.struct_name));
            if tbconf.with_attachment {
                usinglist.push(format!(
                    "chimes_rust::{{ChimesAttachmentInfo, ChimesAttachmentRefInfo}}"
                ));
            }
            for rl in rel.one_to_one.clone() {
                match ctx.get_table_conf(&rl.table_name.clone().unwrap_or_default()) {
                    Some(mt) => {
                        // usinglist.push(format!("crate::entity::{}", mt.struct_name));
                        if mt.with_attachment {
                            usinglist.push(format!(
                                "chimes_rust::{{ChimesAttachmentInfo, ChimesAttachmentRefInfo}}"
                            ));
                        }
                    }
                    None => {}
                }
            }

            for rl in rel.one_to_many.clone() {
                match ctx.get_table_conf(&rl.table_name.clone().unwrap_or_default()) {
                    Some(mt) => {
                        // usinglist.push(format!("crate::entity::{}", mt.struct_name));
                        if mt.with_attachment {
                            usinglist.push(format!(
                                "chimes_rust::{{ChimesAttachmentInfo, ChimesAttachmentRefInfo}}"
                            ));
                        }
                    }
                    None => {}
                }
                if rl.middle_table.is_some() {
                    match ctx.get_table_conf(&rl.middle_table.clone().unwrap_or_default()) {
                        Some(mt) => {
                            // usinglist.push(format!("crate::entity::{}", mt.struct_name));
                            if mt.with_attachment {
                                usinglist.push(format!("chimes_rust::{{ChimesAttachmentInfo, ChimesAttachmentRefInfo}}"));
                            }
                        }
                        None => {}
                    }
                }
            }

            let mut funclist = vec![];
            if rel.generate_select {
                let funcsel = generate_handler_load_for_relation(ctx, rel);
                funclist.push(funcsel);
            }

            if rel.generate_delete {
                let funcdel = generate_handler_remove_for_relation(ctx, rel);
                funclist.push(funcdel);

                let funcdel_multi = generate_handler_remove_multi_for_relation(ctx, rel);
                funclist.push(funcdel_multi);
            }

            if rel.generate_save {
                let funcsave = generate_handler_save_for_relation(ctx, rel);
                funclist.push(funcsave);
            }

            ctx.add_permission_for_relation(rel, &funclist);

            let rfi = RustFileImpl {
                file_name: snake_case(rel.struct_name.clone().as_str()) + ".rs",
                mod_name: "handler".to_string(),
                caretlist: vec![],
                usinglist: usinglist,
                structlist: vec![],
                funclist: funclist,
            };
            Some(rfi)
        }
        None => None,
    }
}

/// 解析关系的描述内容，并生成相应的结构体
/// 关系的结构体生成，依靠Ctx中现有的TableInfo来进行
pub fn parse_relation_as_struct(ctx: &GenerateContext, rel: &RelationConfig) -> RustStruct {
    let mut columns = String::new();
    let tbl_name = rel.major_table.clone();
    let tbc = ctx.get_table_conf(&tbl_name.clone());

    if tbc.is_none() {
        return RustStruct::default();
    }

    let tbconf = tbc.unwrap();
    let cols = ctx.get_table_columns(&tbl_name.clone());
    let mut usings = vec![];

    let mut fields = if rel.extend_major {
        let mut parsed_fields = parse_column_list(ctx, &tbconf, &cols, &mut columns, false, &mut usings);
        // if columns.ends_with(",") {
        //    columns = columns.substring(0, columns.len() - 1).to_string();
        // }
        if tbconf.with_attachment {
            parsed_fields.push(RustStructField {
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
            });
        }
        parsed_fields
    } else {
        let fname = tbconf.api_handler_name.clone();
        vec![RustStructField {
            is_pub: true,
            schema_name: None,
            column_name: String::new(),
            field_name: safe_struct_field_name(&fname),
            field_type: tbconf.struct_name.clone(),
            is_option: true,
            orignal_field_name: None,
            comment: None,
            length: 0i64,
            annotations: vec![],
        }]
    };

    for rl in rel.one_to_one.clone() {
        let rltbc = ctx.get_table_conf(&rl.table_name.clone().unwrap_or_default());
        if rltbc.is_some() {
            let rltcnf = rltbc.unwrap();
            let fdname = safe_struct_field_name(&rltcnf.api_handler_name);
            let rlfd = RustStructField {
                is_pub: true,
                schema_name: rl.table_name.clone(),
                column_name: rl.join_field.clone().unwrap_or_default(),
                field_name: fdname.clone(),
                field_type: rltcnf.struct_name.clone(),
                is_option: true,
                orignal_field_name: Some(fdname.clone()),
                comment: None,
                length: 0i64,
                annotations: vec![],
            };
            fields.push(rlfd);
        }
    }

    for rl in rel.one_to_many.clone() {
        let rltbc = ctx.get_table_conf(&rl.table_name.clone().unwrap_or_default());
        if rltbc.is_some() {
            let rltcnf = rltbc.unwrap();
            let fdname = format!("{}s", safe_struct_field_name(&rltcnf.api_handler_name));
            let rlfd = RustStructField {
                is_pub: true,
                schema_name: rl.table_name.clone(),
                column_name: rl.join_field.clone().unwrap_or_default(),
                field_name: fdname.clone(),
                field_type: format!("Vec<{}>", rltcnf.struct_name.clone()),
                is_option: false,
                orignal_field_name: Some(fdname.clone()),
                comment: None,
                length: 0i64,
                annotations: vec!["#[serde(default)]".to_string()],
            };
            fields.push(rlfd);
            let rlfd_deleted = RustStructField {
                is_pub: true,
                schema_name: rl.table_name.clone(),
                column_name: rl.join_field.clone().unwrap_or_default(),
                field_name: fdname.clone() + "_deleted",
                field_type: format!("Vec<{}>", rltcnf.struct_name.clone()),
                is_option: false,
                orignal_field_name: Some(fdname.clone()),
                comment: None,
                length: 0i64,
                annotations: vec![],
            };
            fields.push(rlfd_deleted);
        }
    }

    // let crudtbl = format!("#[crud_table(table_name:\"{}\"|table_columns:\"{}\")]", tbl_name.clone(), columns);
    let anno = vec!["#[derive(Debug, Clone, Default, Deserialize, Serialize)]".to_string()];

    let mut funclist = vec![];

    let from_func = generate_func_from_major_table(ctx, rel);
    let to_func = generate_func_to_major_table(ctx, rel);
    let refine_func = generate_func_refine(ctx, rel);

    funclist.push(from_func);
    funclist.push(to_func);
    funclist.push(refine_func);

    if rel.generate_select {
        let from_id = generate_func_from_pkey_for_relation(ctx, rel);
        funclist.push(from_id);
    }

    if rel.generate_save {
        let update_func = generate_func_update_for_relation(ctx, rel);
        funclist.push(update_func);

        let delete_more = generate_func_delete_ids_for_relation(ctx, rel);
        funclist.push(delete_more);
    }

    // if rel.generate_update {
    // let update_slct_func = generate_func_update_selective_for_relation(ctx, rel);
    // funclist.push(update_slct_func);
    // }

    if rel.generate_delete {
        let delete_func = generate_func_delete_for_relation(ctx, rel);
        funclist.push(delete_func);
    }

    RustStruct {
        is_pub: true,
        has_paging: tbconf.page_query,
        struct_name: rel.struct_name.clone(),
        annotations: anno,
        fields: fields,
        funclist: funclist,
        usings,
    }
}

fn generate_func_from_major_table(ctx: &GenerateContext, rel: &RelationConfig) -> RustFunc {
    let tbl_name = rel.major_table.clone();
    let tbc = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tbc.unwrap();
    let mut body = vec![];
    let mut usings = vec![];

    body.push(format!("{} {{", rel.struct_name.clone()));
    if rel.extend_major {
        let mut columns = String::new();
        let cols = ctx.get_table_columns(&tbl_name.clone());
        let parsed_fields = parse_column_list(ctx, &tbconf, &cols, &mut columns, false, &mut usings);
        for fd in parsed_fields {
            let fname = fd.field_name.clone();
            if is_copied_type(&fd.field_type.clone()) {
                body.push(format!(
                    "{}: param.{},",
                    safe_struct_field_name(&fname),
                    safe_struct_field_name(&fname)
                ));
            } else {
                body.push(format!(
                    "{}: param.{}.clone(),",
                    safe_struct_field_name(&fname),
                    safe_struct_field_name(&fname)
                ));
            }
        }
        if tbconf.with_attachment {
            body.push(format!("attachments: param.attachments.clone(),"));
        }
    } else {
        let fname = tbconf.api_handler_name.clone();
        body.push(format!(
            "{}: Some(param.clone()),",
            safe_struct_field_name(&fname)
        ));
    }

    for rl in rel.one_to_one.clone() {
        let rltbc = ctx.get_table_conf(&rl.table_name.unwrap_or_default());
        if rltbc.is_some() {
            let rltcnf = rltbc.unwrap();
            body.push(format!(
                "{}: None,",
                safe_struct_field_name(&rltcnf.api_handler_name)
            ));
        }
    }

    for rl in rel.one_to_many.clone() {
        let rltbc = ctx.get_table_conf(&rl.table_name.unwrap_or_default());
        if rltbc.is_some() {
            let rltcnf = rltbc.unwrap();
            body.push(format!(
                "{}: vec![],",
                format!("{}s", safe_struct_field_name(&rltcnf.api_handler_name))
            ));
            body.push(format!(
                "{}_deleted: vec![],",
                format!("{}s", safe_struct_field_name(&rltcnf.api_handler_name))
            ));
        }
    }

    body.push(format!("}}"));

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push((
        "param".to_string(),
        "&".to_owned() + tbconf.struct_name.clone().as_str(),
    ));

    RustFunc {
        is_struct_fn: true,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: false,
        func_name: format!("from_{}", tbconf.api_handler_name.clone()),
        return_is_option: false,
        return_is_result: false,
        return_type: Some("Self".to_string()),
        params: params,
        bodylines: body,
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(format!("实体转{}", rel.comment.clone())),
        api_method: None,
        api_pattern: None,
    }
}

fn generate_func_to_major_table(ctx: &GenerateContext, rel: &RelationConfig) -> RustFunc {
    let tbl_name = rel.major_table.clone();
    let tbc = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tbc.unwrap();
    let mut body = vec![];
    let mut usings = vec![];

    if rel.extend_major {
        let mut columns = String::new();
        let cols = ctx.get_table_columns(&tbl_name.clone());
        let parsed_fields = parse_column_list(ctx, &tbconf, &cols, &mut columns, false, &mut usings);
        for otp in rel.one_to_one.clone() {
            let tpconf = ctx.get_table_conf(&otp.table_name.clone().unwrap_or_default());
            if tpconf.is_some() {
                let tpc = tpconf.unwrap();
                let fdname = safe_struct_field_name(&otp.join_field.clone().unwrap_or_default());
                let mjname = safe_struct_field_name(&otp.major_field.clone().unwrap_or_default());
                body.push(format!(
                    "let self_{} = match self.{}.clone() {{",
                    fdname.clone(),
                    tpc.api_handler_name.clone()
                ));
                body.push(format!("Some(np) => np.{}.clone(),", fdname.clone()));
                body.push(format!("None => self.{}.clone(),", mjname.clone()));
                body.push(format!("}};"));
            }
        }
        body.push(format!("{} {{", tbconf.struct_name.clone()));
        for fd in parsed_fields {
            let fname = fd.field_name.clone();
            let mut found = None;
            for otp in rel.one_to_one.clone() {
                let tpconf = ctx.get_table_conf(&otp.table_name.clone().unwrap_or_default());
                if tpconf.is_some() {
                    let _tpc = tpconf.unwrap();
                    let fdname =
                        safe_struct_field_name(&otp.join_field.clone().unwrap_or_default());
                    // body.push(format!("let self_{} = self.{}.{}.clone();", fdname.clone(),  tpc.api_handler_name.clone(), fdname.clone()));
                    let sname = format!("self_{}", fdname.clone());
                    if Some(fd.column_name.clone()) == otp.join_field {
                        found = Some(fdname.clone());
                        body.push(format!(
                            "{}: {},",
                            safe_struct_field_name(&fname),
                            sname.clone()
                        ));
                    }
                }
            }
            if found.is_none() {
                if is_copied_type(&fd.field_type) {
                    body.push(format!(
                        "{}: self.{},",
                        safe_struct_field_name(&fname),
                        safe_struct_field_name(&fname)
                    ));
                } else {
                    body.push(format!(
                        "{}: self.{}.clone(),",
                        safe_struct_field_name(&fname),
                        safe_struct_field_name(&fname)
                    ));
                }
            }
        }
        if tbconf.with_attachment {
            body.push(format!("attachments: self.attachments.clone(),"));
        }
        body.push(format!("}}"));
    } else {
        let fname = tbconf.api_handler_name.clone();
        body.push(format!(
            "match self.{}.clone() {{",
            safe_struct_field_name(&fname)
        ));
        body.push(format!("Some(st) => st,"));
        body.push(format!("None => {}::default()", tbconf.struct_name.clone()));
        body.push(format!("}}"));
    }

    let params = Vec::new();

    RustFunc {
        is_struct_fn: true,
        is_self_fn: true,
        is_self_mut: false,
        is_pub: true,
        is_async: false,
        func_name: format!("to_{}", tbconf.api_handler_name.clone()),
        return_is_option: false,
        return_is_result: false,
        return_type: Some(tbconf.struct_name.clone()),
        params: params,
        bodylines: body,
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(format!("{}转实体对象", rel.comment.clone())),
        api_method: None,
        api_pattern: None,
    }
}

fn generate_func_refine(ctx: &GenerateContext, rel: &RelationConfig) -> RustFunc {
    let tbl_name = rel.major_table.clone();
    let tbc = ctx.get_table_conf(&tbl_name.clone());
    let _tbconf = tbc.unwrap();
    let tbl = ctx.get_table_info(&tbl_name.clone());
    let tblinfo = tbl.unwrap();
    let mut body = vec![];
    let mut has_su = false;
    let pkcols = ctx.get_table_column_by_primary_key(&tbl_name);
    for col in pkcols.clone() {
        body.push(format!(
            "if self.{}.is_none() {{",
            safe_struct_field_name(&col.column_name.clone().unwrap_or_default())
        ));
        has_su |= process_detail_common_fields(ctx, "self", &mut body, &tblinfo, 1);
        body.push(format!("}} else {{"));
        has_su |= process_detail_common_fields(ctx, "self", &mut body, &tblinfo, 5);
        body.push(format!("}}"));
    }

    for rl in rel.one_to_one.clone() {
        let one_tbl_name = rl.table_name.clone().unwrap_or_default();
        let onetbc = ctx.get_table_conf(&one_tbl_name.clone());
        let onetbconf = onetbc.unwrap();
        let onetbl = ctx.get_table_info(&one_tbl_name.clone());
        let onetblinfo = onetbl.unwrap();
        body.push(format!(
            "self.{} = match self.{}.clone() {{",
            onetbconf.api_handler_name.clone(),
            onetbconf.api_handler_name.clone()
        ));
        body.push(format!("Some(valtp) => {{"));
        body.push(format!("let mut mval = valtp.clone();"));
        body.push(format!(
            "if mval.{}.is_none() {{",
            onetbconf.primary_key.clone()
        ));
        has_su |= process_detail_common_fields(ctx, "mval", &mut body, &onetblinfo, 1);
        body.push(format!("}} else {{"));
        has_su |= process_detail_common_fields(ctx, "mval", &mut body, &onetblinfo, 2);
        body.push(format!("}}"));
        body.push(format!("Some(mval)"));
        body.push(format!("}},"));
        body.push(format!("None => None"));
        body.push(format!("}};"));
    }

    for rl in rel.one_to_many.clone() {
        let many_tbl_name = rl.table_name.clone().unwrap_or_default();
        let manytbc = ctx.get_table_conf(&many_tbl_name.clone());
        let manytbconf = manytbc.unwrap();
        let manytbl = ctx.get_table_info(&many_tbl_name.clone());
        let manytblinfo = manytbl.unwrap();
        body.push(format!(
            "self.{}s = self.{}s.clone().into_iter().map(|f| {{",
            manytbconf.api_handler_name.clone(),
            manytbconf.api_handler_name.clone()
        ));
        body.push(format!("let mut fm = f.clone();"));
        body.push(format!(
            "if fm.{}.is_none() || fm.{} == Some(0i64) {{",
            manytbconf.primary_key.clone(),
            manytbconf.primary_key.clone()
        ));
        has_su |= process_detail_common_fields(ctx, "fm", &mut body, &manytblinfo, 1);
        body.push(format!("}} else {{"));
        has_su |= process_detail_common_fields(ctx, "fm", &mut body, &manytblinfo, 2);
        body.push(format!("}};"));
        body.push(format!("fm"));
        body.push(format!(
            "}}).collect::<Vec<{}>>();",
            manytbconf.struct_name.clone()
        ));
    }

    let mut params = Vec::new();
    if has_su {
        params.push(("su".to_string(), "&SystemUser<ChimesUserInfo>".to_string()));
    } else {
        params.push(("_su".to_string(), "&SystemUser<ChimesUserInfo>".to_string()));
    }
    RustFunc {
        is_struct_fn: true,
        is_self_fn: true,
        is_self_mut: true,
        is_pub: true,
        is_async: false,
        func_name: format!("refine"),
        return_is_option: false,
        return_is_result: false,
        return_type: None,
        params: params,
        bodylines: body,
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(format!("为{}精化数据", rel.comment.clone())),
        api_method: None,
        api_pattern: None,
    }
}

fn generate_func_from_pkey_for_relation(ctx: &GenerateContext, tbl: &RelationConfig) -> RustFunc {
    let tbl_name = tbl.major_table.clone();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    let mut params_text = String::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));
    for col in pkcols.clone() {
        let dt = parse_data_type_as_rust_type(&col.data_type.unwrap_or_default());
        params.push((
            col.column_name.clone().unwrap_or_default().to_lowercase(),
            "&".to_string() + dt.as_str(),
        ));
        params_text.push_str(
            col.column_name
                .clone()
                .unwrap_or_default()
                .to_lowercase()
                .as_str(),
        );
        params_text.push_str(",");
    }

    if params_text.ends_with(",") {
        params_text = params_text.substring(0, params_text.len() - 1).to_string();
    }

    let tbc = tblinfo.unwrap();

    let mut body = vec![];
    body.push(format!(
        "match {}::from_id(rb, {}).await {{",
        tbc.struct_name.clone(),
        params_text
    ));
    body.push(format!("Ok(ts) => {{"));
    body.push(format!("match ts {{"));
    body.push(format!("Some(mp) => {{"));
    body.push(format!(
        "let mut selfmp = Self::from_{}(&mp);",
        tbc.api_handler_name.clone()
    ));
    // Above is right
    for otp in tbl.one_to_one.clone() {
        let tpconf = ctx.get_table_conf(&otp.table_name.clone().unwrap_or_default());
        if tpconf.is_some() {
            let tpc = tpconf.unwrap();

            let mut optpkcols =
                ctx.get_table_column_by_primary_key(&otp.table_name.clone().unwrap_or_default());
            if optpkcols.is_empty() {
                optpkcols.append(
                    &mut ctx.get_table_pkey_column(&&otp.table_name.clone().unwrap_or_default()),
                );
            }

            // 关系型的表，目前代码生成中，支持一个主键，没有主键也不行
            let optpkcol = optpkcols.get(0).unwrap();
            let _optpkcolname = safe_struct_field_name(
                &optpkcol
                    .column_name
                    .clone()
                    .unwrap_or_default()
                    .to_lowercase(),
            );

            body.push(format!(
                "let tmp_{} = {} {{",
                tpc.api_handler_name.clone(),
                tpc.struct_name.clone()
            ));

            if tbl.extend_major {
                if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &otp.major_field.clone().unwrap_or_default()) {
                    body.push(format!(
                        "{}: selfmp.{},",
                        otp.join_field.clone().unwrap_or_default().to_lowercase(),
                        otp.major_field.clone().unwrap_or_default().to_lowercase()
                    ));
                } else {
                    body.push(format!(
                        "{}: selfmp.{}.clone(),",
                        otp.join_field.clone().unwrap_or_default().to_lowercase(),
                        otp.major_field.clone().unwrap_or_default().to_lowercase()
                    ));
                }
            } else {
                if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &otp.major_field.clone().unwrap_or_default()) {
                    body.push(format!(
                        "{}: selfmp.{}.clone().unwrap().{},",
                        otp.join_field.clone().unwrap_or_default().to_lowercase(),
                        tbc.api_handler_name.clone(),
                        otp.major_field.clone().unwrap_or_default().to_lowercase()
                    ));
                } else {
                    body.push(format!(
                        "{}: selfmp.{}.clone().unwrap().{}.clone(),",
                        otp.join_field.clone().unwrap_or_default().to_lowercase(),
                        tbc.api_handler_name.clone(),
                        otp.major_field.clone().unwrap_or_default().to_lowercase()
                    ));
                }
            }
            body.push(format!("..Default::default()"));
            body.push(format!("}};"));

            body.push(format!(
                "selfmp.{} = match tmp_{}.query_list(rb).await {{",
                tpc.api_handler_name.clone(),
                tpc.api_handler_name.clone()
            ));
            body.push(format!("Ok(lst) => {{"));
            body.push(format!("if lst.len() > 0 {{"));
            body.push(format!("Some(lst[0].clone())"));
            body.push(format!("}} else {{"));
            body.push(format!("None"));
            body.push(format!("}}"));
            body.push(format!("}}"));
            body.push(format!("Err(_) => {{"));
            body.push(format!("None"));
            body.push(format!("}}"));
            body.push(format!("}};"));
        }
    }

    for otp in tbl.one_to_many.clone() {
        let tpconf = ctx.get_table_conf(&otp.table_name.clone().unwrap_or_default());
        if tpconf.is_some() {
            let tpc = tpconf.unwrap();

            let mut optpkcols =
                ctx.get_table_column_by_primary_key(&otp.table_name.clone().unwrap_or_default());
            if optpkcols.is_empty() {
                optpkcols.append(
                    &mut ctx.get_table_pkey_column(&&otp.table_name.clone().unwrap_or_default()),
                );
            }

            let many_many = otp.middle_table.is_some();

            // 关系型的表，目前代码生成中，支持一个主键，没有主键也不行
            let optpkcol = optpkcols.get(0).unwrap();
            let _optpkcolname = safe_struct_field_name(
                &optpkcol
                    .column_name
                    .clone()
                    .unwrap_or_default()
                    .to_lowercase(),
            );
            if many_many {
                let joinfd = otp.join_field.clone().unwrap_or_default();
                let majorfd = otp.major_field.clone().unwrap_or_default();

                let sql = format!(
                    "SELECT tp.* FROM {} tp INNER JOIN {} mt ON tp.{} = mt.{} WHERE mt.{} = ?",
                    otp.table_name.clone().unwrap_or_default(),
                    otp.middle_table.clone().unwrap_or_default(),
                    joinfd.clone(),
                    joinfd.clone(),
                    majorfd.clone()
                );
                body.push(format!("let mut rb_args = vec![];"));
                body.push(format!(
                    "let sql_{} = \"{}\";",
                    tpc.api_handler_name.clone(),
                    sql
                ));
                if tbl.extend_major {
                    body.push(format!("rb_args.push(rbson::to_bson(&selfmp.{}.clone().unwrap_or_default()).unwrap_or_default());", majorfd.clone().to_lowercase()));
                } else {
                    body.push(format!("rb_args.push(rbson::to_bson(&selfmp.{}.clone().unwrap().{}.clone().unwrap_or_default()).unwrap_or_default());", 
                                                tbc.api_handler_name.clone(), majorfd.clone().to_lowercase()));
                }

                body.push(format!(
                    "selfmp.{}s = match rb.fetch(sql_{}, rb_args).await {{",
                    tpc.api_handler_name.clone(),
                    tpc.api_handler_name.clone()
                ));
                body.push(format!("Ok(lst) => {{"));
                body.push(format!("lst"));
                body.push(format!("}}"));
                body.push(format!("Err(_) => {{"));
                body.push(format!("vec![]"));
                body.push(format!("}}"));
                body.push(format!("}};"));
            } else {
                body.push(format!(
                    "let tmp_{} = {} {{",
                    tpc.api_handler_name.clone(),
                    tpc.struct_name.clone()
                ));
                if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &otp.major_field.clone().unwrap_or_default()) {
                    body.push(format!(
                        "{}: selfmp.{},",
                        otp.join_field.clone().unwrap_or_default().to_lowercase(),
                        otp.major_field.clone().unwrap_or_default().to_lowercase()
                    ));
                } else {
                    body.push(format!(
                        "{}: selfmp.{}.clone(),",
                        otp.join_field.clone().unwrap_or_default().to_lowercase(),
                        otp.major_field.clone().unwrap_or_default().to_lowercase()
                    ));
                }
                body.push(format!("..Default::default()"));
                body.push(format!("}};"));
                body.push(format!(
                    "selfmp.{}s = match tmp_{}.query_list(rb).await {{",
                    tpc.api_handler_name.clone(),
                    tpc.api_handler_name.clone()
                ));
                body.push(format!("Ok(lst) => {{"));
                body.push(format!("lst"));
                body.push(format!("}}"));
                body.push(format!("Err(_) => {{"));
                body.push(format!("vec![]"));
                body.push(format!("}}"));
                body.push(format!("}};"));
            }
        }
    }
    // Below is right
    body.push(format!("Ok(Some(selfmp))"));
    body.push(format!("}}"));
    body.push(format!("None => {{"));
    body.push(format!("Ok(None)"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!("Err(err)"));
    body.push(format!("}}"));
    body.push(format!("}}"));

    RustFunc {
        is_struct_fn: true,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: "load".to_string(),
        return_is_option: true,
        return_is_result: true,
        return_type: Some("Self".to_string()),
        params: params,
        bodylines: body,
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(format!("{}按ID加载", tbl.comment.clone())),
        api_method: None,
        api_pattern: None,
    }
}

/**
 * 执行Update、Save操作
 * 如果主键是空值，且为自动生成的值，则会
 */
fn generate_func_update_for_relation(ctx: &GenerateContext, tbl: &RelationConfig) -> RustFunc {
    let tbl_name = tbl.major_table.clone();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    // 关系型的表，目前代码生成中，支持一个主键，没有主键也不行
    let pkcol = pkcols.get(0).unwrap();
    let pkcolname =
        safe_struct_field_name(&pkcol.column_name.clone().unwrap_or_default().to_lowercase());

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&mut RBatisTxExecutor<'_>".to_string()));

    let mut body = vec![];

    body.push(format!("let mut ret: Option<Error>;"));
    // Save the major table first
    body.push(format!(
        "let mut self_{} = self.to_{}();",
        tbconf.api_handler_name.clone(),
        tbconf.api_handler_name.clone()
    ));
    body.push(format!(
        "if self_{}.{}.is_none() {{",
        tbconf.api_handler_name.clone(),
        pkcolname.clone()
    ));
    body.push(format!(
        "ret = match self_{}.save(rb).await {{",
        tbconf.api_handler_name.clone()
    ));
    body.push(format!("Ok(_rs) => {{")); //  begin of Ok
    body.push(format!("None"));
    body.push(format!("}}")); //  end of Ok
    body.push(format!("Err(err) => {{")); //  begin of none
    body.push(format!(
        "log::info!(\"Save {} occurred an error {{}}\", err);",
        tbconf.api_handler_name.clone()
    ));
    body.push(format!("Some(err)"));
    body.push(format!("}}")); //  end of error
    body.push(format!("}}")); // end of if
    body.push(format!("}}")); // end of if
    body.push(format!("else {{"));
    body.push(format!(
        "ret = match self_{}.update_selective(rb).await {{",
        tbconf.api_handler_name.clone()
    ));
    body.push(format!("Ok(_rs) => {{")); //  begin of Ok
    body.push(format!("None"));
    body.push(format!("}}")); //  end of Ok
    body.push(format!("Err(err) => {{")); //  begin of none
    body.push(format!(
        "log::info!(\"Update {} occurred an error {{}}\", err);",
        tbconf.api_handler_name.clone()
    ));
    body.push(format!("Some(err)"));
    body.push(format!("}}")); //  end of error
    body.push(format!("}}")); // end of if
    body.push(format!("}}"));

    for otp in tbl.one_to_one.clone() {
        let tpconf = ctx.get_table_conf(&otp.table_name.clone().unwrap_or_default());
        if !otp.readonly && tpconf.is_some() {
            let tpc = tpconf.unwrap();
            let _tpctblinfo = ctx.get_table_info(&otp.table_name.clone().unwrap_or_default());

            let mut optpkcols =
                ctx.get_table_column_by_primary_key(&otp.table_name.clone().unwrap_or_default());
            if optpkcols.is_empty() {
                optpkcols.append(
                    &mut ctx.get_table_pkey_column(&&otp.table_name.clone().unwrap_or_default()),
                );
            }

            // 关系型的表，目前代码生成中，支持一个主键，没有主键也不行
            let optpkcol = optpkcols.get(0).unwrap();
            let optpkcolname = safe_struct_field_name(
                &optpkcol
                    .column_name
                    .clone()
                    .unwrap_or_default()
                    .to_lowercase(),
            );

            body.push(format!("if ret.is_none() {{"));
            body.push(format!(
                "ret = match self.{}.clone() {{",
                tpc.api_handler_name.clone()
            ));
            body.push(format!("Some(tp) => {{")); // begin of Some
            body.push(format!("let mut mtp = tp.clone();"));
            let mjid = if otp.major_field.is_none() {
                pkcolname.clone()
            } else {
                otp.major_field.clone().unwrap()
            };
            if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &mjid) {
                body.push(format!(
                    "mtp.{} = self_{}.{};",
                    safe_struct_field_name(&otp.join_field.clone().unwrap_or_default().to_lowercase()),
                    tbconf.api_handler_name.clone(),
                    mjid.clone()
                ));
            } else {
                body.push(format!(
                    "mtp.{} = self_{}.{}.clone();",
                    safe_struct_field_name(&otp.join_field.clone().unwrap_or_default().to_lowercase()),
                    tbconf.api_handler_name.clone(),
                    mjid.clone()
                ));
            }
            body.push(format!("if mtp.{}.is_none() {{", optpkcolname.clone()));
            body.push(format!("match mtp.save(rb).await {{"));
            body.push(format!("Ok(_mtpsave) => {{")); //  begin of Ok
            body.push(format!("None"));
            body.push(format!("}}")); //  end of Ok
            body.push(format!("Err(err) => {{")); //  begin of none
            body.push(format!(
                "log::info!(\"Save {} occurred an error {{}}\", err);",
                tpc.api_handler_name.clone()
            ));
            body.push(format!("Some(err)"));
            body.push(format!("}}")); //  end of error
            body.push(format!("}}")); //  end of mtpsave

            body.push(format!("}} else {{")); //  end of Some(tp)
            body.push(format!("match mtp.update(rb).await {{"));
            body.push(format!("Ok(_mtpsave) => {{")); //  begin of Ok
            body.push(format!("None"));
            body.push(format!("}}")); //  end of Ok
            body.push(format!("Err(err) => {{")); //  begin of none
            body.push(format!(
                "log::info!(\"Save {} occurred an error {{}}\", err);",
                tpc.api_handler_name.clone()
            ));
            body.push(format!("Some(err)"));
            body.push(format!("}}")); //  end of error
            body.push(format!("}}")); //  end of mtpsave

            body.push(format!("}}")); //  end of Some(tp)
            body.push(format!("}}")); //  end of Some(tp)
            body.push(format!("None => {{")); //  begin of none
            body.push(format!("None"));
            body.push(format!("}}")); //  end of None
            body.push(format!("}};")); // end of match option
            body.push(format!("}}")); // end of if
        }
    }

    for otp in tbl.one_to_many.clone() {
        let tpconf = if otp.middle_table.clone().is_none() {
            ctx.get_table_conf(&otp.table_name.clone().unwrap_or_default())
        } else {
            ctx.get_table_conf(&otp.middle_table.clone().unwrap_or_default())
        };

        let _tpctblinfo = ctx.get_table_info(&otp.table_name.clone().unwrap_or_default());

        let targtbl = ctx.get_table_conf(&otp.table_name.clone().unwrap_or_default());

        let many_many = if otp.middle_table.clone().is_none() {
            false
        } else {
            true
        };

        let targtblconf = targtbl.unwrap();

        let mut optpkcols =
            ctx.get_table_column_by_primary_key(&otp.table_name.clone().unwrap_or_default());
        if optpkcols.is_empty() {
            optpkcols.append(
                &mut ctx.get_table_pkey_column(&&otp.table_name.clone().unwrap_or_default()),
            );
        }

        // 关系型的表，目前代码生成中，支持一个主键，没有主键也不行
        let optpkcol = optpkcols.get(0).unwrap();
        let optpkcolname = safe_struct_field_name(
            &optpkcol
                .column_name
                .clone()
                .unwrap_or_default()
                .to_lowercase(),
        );

        if !otp.readonly && tpconf.is_some() {
            let tpc = tpconf.unwrap();
            body.push(format!("// remove batch for {}.", tpc.struct_name.clone()));

            body.push(format!("if ret.is_none() {{"));
            if many_many {
                body.push(format!(
                    "let mut rm_{} = {} {{",
                    tpc.api_handler_name.clone(),
                    tpc.struct_name.clone()
                ));
                if tbl.extend_major {
                    if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &otp.major_field.clone().unwrap_or_default()) {
                        body.push(format!(
                            "{}: self.{},",
                            otp.major_field.clone().unwrap_or_default(),
                            otp.major_field.clone().unwrap_or_default()
                        ));
                    } else {
                        body.push(format!(
                            "{}: self.{}.clone(),",
                            otp.major_field.clone().unwrap_or_default(),
                            otp.major_field.clone().unwrap_or_default()
                        ));
                    }
                } else {
                    if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &otp.major_field.clone().unwrap_or_default()) {
                        body.push(format!(
                            "{}: self.{}.clone().unwrap().{},",
                            otp.major_field.clone().unwrap_or_default(),
                            tbconf.api_handler_name.clone(),
                            otp.major_field.clone().unwrap_or_default()
                        ));
                    } else {
                        body.push(format!(
                            "{}: self.{}.clone().unwrap().{}.clone(),",
                            otp.major_field.clone().unwrap_or_default(),
                            tbconf.api_handler_name.clone(),
                            otp.major_field.clone().unwrap_or_default()
                        ));
                    }
                }
                body.push(format!("..Default::default()"));
                body.push(format!("}};"));
                //}
                body.push(format!(
                    "ret = match rm_{}.remove_batch(rb).await {{",
                    tpc.api_handler_name.clone()
                ));
                body.push(format!("Ok(_) => {{")); //  begin of Ok
                body.push(format!("None"));
                body.push(format!("}}")); //  end of Ok
                body.push(format!("Err(err) => {{")); //  begin of none
                body.push(format!(
                    "log::info!(\"Remove {} occurred an error {{}}\", err);",
                    tpc.api_handler_name.clone()
                ));
                body.push(format!("Some(err)"));
                body.push(format!("}}")); //  end of error
                body.push(format!("}};")); // end of rm_{}
                body.push(format!("}}")); // end of if

                body.push(format!(
                    "for row in self.{}s.clone() {{",
                    targtblconf.api_handler_name
                ));
                body.push(format!(
                    "let svrow_{} = {} {{",
                    tpc.api_handler_name.clone(),
                    tpc.struct_name.clone()
                ));
                if tbl.extend_major {
                    if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &pkcolname) {
                        body.push(format!(
                            "{}: self.{},",
                            otp.join_field.clone().unwrap_or_default(),
                            pkcolname.clone()
                        ));
                    } else {
                        body.push(format!(
                            "{}: self.{}.clone(),",
                            otp.join_field.clone().unwrap_or_default(),
                            pkcolname.clone()
                        ));
                    }
                } else {
                    if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &pkcolname) {
                        body.push(format!(
                            "{}: self.{}.clone().unwrap().{},",
                            otp.major_field.clone().unwrap_or_default(),
                            tbconf.api_handler_name.clone(),
                            pkcolname.clone()
                        ));
                    } else {
                        body.push(format!(
                            "{}: self.{}.clone().unwrap().{}.clone(),",
                            otp.major_field.clone().unwrap_or_default(),
                            tbconf.api_handler_name.clone(),
                            pkcolname.clone()
                        ));
                    }
                }
                body.push(format!("..Default::default()"));
                body.push(format!("}};"));

                if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &otp.major_field.clone().unwrap_or_default()) {
                    body.push(format!(
                        "svrow_{}.{} = row.{};",
                        tpc.api_handler_name.clone(),
                        otp.join_field.clone().unwrap_or_default(),
                        otp.major_field.clone().unwrap_or_default()
                    ));
                } else {
                    body.push(format!(
                        "svrow_{}.{} = row.{}.clone();",
                        tpc.api_handler_name.clone(),
                        otp.join_field.clone().unwrap_or_default(),
                        otp.major_field.clone().unwrap_or_default()
                    ));
                }

                body.push(format!(
                    "ret = match svrow_{}.save(rb).await {{",
                    tpc.api_handler_name.clone()
                ));
                body.push(format!("Ok(_) => {{")); //  begin of Ok
                body.push(format!("None"));
                body.push(format!("}}")); //  end of Ok
                body.push(format!("Err(err) => {{")); //  begin of none
                body.push(format!(
                    "log::info!(\"Save {} occurred an error {{}}\", err);",
                    tpc.api_handler_name.clone()
                ));
                body.push(format!("Some(err)"));
                body.push(format!("}}")); //  end of error
                body.push(format!("}};")); // end of rm_{}
                body.push(format!("}}")); // end of for
            } else {
                // if otp.middle_table.clone().is_some() {
                if !tbl.deleted_by_relation {
                    body.push(format!(
                        "for row in self.{}s_deleted.clone() {{",
                        targtblconf.api_handler_name.clone()
                    ));
                    body.push(format!(
                        "let mut rm_{} = row.clone();",
                        tpc.api_handler_name.clone()
                    ));
                    body.push(format!(
                        "if rm_{}.{}.is_some() {{",
                        tpc.api_handler_name.clone(),
                        optpkcolname.clone()
                    ));
                    body.push(format!(
                        "match rm_{}.remove(rb).await {{",
                        tpc.api_handler_name.clone()
                    ));
                    body.push(format!("Ok(_) => {{}}"));
                    body.push(format!("Err(err) => {{"));
                    body.push(format!(
                        "log::info!(\"Remove {} occurred an error {{}}\", err);",
                        tpc.api_handler_name.clone()
                    ));
                    body.push(format!("}}"));
                    body.push(format!("}};"));
                    body.push(format!("}}"));
                    body.push(format!("}}"));
                } else {
                    body.push(format!(
                        "if !self.{}s_deleted.is_empty() {{",
                        targtblconf.api_handler_name.clone()
                    ));
                    body.push(format!("let delete_ids: Vec<i64> = self.{}s_deleted.clone().into_iter().filter(|f| f.{}.is_some()).map(|f| f.{}.unwrap_or_default()).collect();", targtblconf.api_handler_name, targtblconf.primary_key.clone(), targtblconf.primary_key.clone()));
                    body.push(format!("let cond = {} {{", targtblconf.struct_name.clone()));
                    if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &pkcolname) {
                        body.push(format!(
                            "{}: self_{}.{},",
                            otp.join_field.clone().unwrap_or_default(),
                            tbconf.api_handler_name.clone(),
                            pkcolname.clone()
                        ));
                    } else {
                        body.push(format!(
                            "{}: self_{}.{}.clone(),",
                            otp.join_field.clone().unwrap_or_default(),
                            tbconf.api_handler_name.clone(),
                            pkcolname.clone()
                        ));
                    }
                    body.push(format!("..Default::default()"));
                    body.push(format!("}};"));

                    body.push(format!(
                        "match {}::remove_ids(rb, &delete_ids, &cond).await {{",
                        targtblconf.struct_name.clone()
                    ));
                    body.push(format!("Ok(_) => {{}},"));
                    body.push(format!("Err(err) => {{"));
                    body.push(format!(
                        "log::info!(\"Remove {} occurred an error {{}}\", err);",
                        targtblconf.api_handler_name
                    ));
                    body.push(format!("}}"));
                    body.push(format!("}}"));
                    body.push(format!("}} else {{"));
                    body.push(format!("let delete_not_ids: Vec<i64> = self.{}s.clone().into_iter().filter(|f| f.{}.is_some()).map(|f| f.{}.unwrap_or_default()).collect();", targtblconf.api_handler_name, targtblconf.primary_key.clone(), targtblconf.primary_key.clone()));
                    body.push(format!("if delete_not_ids.len() > 0 {{"));
                    body.push(format!("let cond = {} {{", targtblconf.struct_name.clone()));
                    if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &pkcolname) {
                        body.push(format!(
                            "{}: self_{}.{},",
                            otp.join_field.clone().unwrap_or_default(),
                            tbconf.api_handler_name.clone(),
                            pkcolname.clone()
                        ));
                    } else {
                        body.push(format!(
                            "{}: self_{}.{}.clone(),",
                            otp.join_field.clone().unwrap_or_default(),
                            tbconf.api_handler_name.clone(),
                            pkcolname.clone()
                        ));
                    }
                    body.push(format!("..Default::default()"));
                    body.push(format!("}};"));

                    body.push(format!(
                        "match {}::remove_not_ids(rb, &delete_not_ids, &cond).await {{",
                        targtblconf.struct_name.clone()
                    ));
                    body.push(format!("Ok(_) => {{}},"));
                    body.push(format!("Err(err) => {{"));
                    body.push(format!(
                        "log::info!(\"Remove {} occurred an error {{}}\", err);",
                        targtblconf.api_handler_name.clone()
                    ));
                    body.push(format!("}}"));
                    body.push(format!("}}"));
                    body.push(format!("}}"));
                    body.push(format!("}}"));
                }

                body.push(format!(
                    "for row in self.{}s.clone() {{",
                    targtblconf.api_handler_name
                ));
                body.push(format!(
                    "let mut rm_{} = row.clone();",
                    tpc.api_handler_name.clone()
                ));

                if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &pkcolname) {
                    body.push(format!(
                        "rm_{}.{} = self_{}.{};",
                        tpc.api_handler_name.clone(),
                        otp.join_field.unwrap_or_default(),
                        tbconf.api_handler_name.clone(),
                        pkcolname.clone()
                    ));
                } else {
                    body.push(format!(
                        "rm_{}.{} = self_{}.{}.clone();",
                        tpc.api_handler_name.clone(),
                        otp.join_field.unwrap_or_default(),
                        tbconf.api_handler_name.clone(),
                        pkcolname.clone()
                    ));
                }

                body.push(format!(
                    "if rm_{}.{}.is_none() {{",
                    tpc.api_handler_name.clone(),
                    optpkcolname.clone()
                ));
                body.push(format!(
                    "ret = match rm_{}.save(rb).await {{",
                    tpc.api_handler_name.clone()
                ));
                body.push(format!("Ok(_) => {{")); //  begin of Ok
                body.push(format!("None"));
                body.push(format!("}}")); //  end of Ok
                body.push(format!("Err(err) => {{")); //  begin of none
                body.push(format!(
                    "log::info!(\"Remove {} occurred an error {{}}\", err);",
                    tpc.api_handler_name.clone()
                ));
                body.push(format!("Some(err)"));
                body.push(format!("}}")); //  end of error
                body.push(format!("}};")); // end of rm_{}
                                           // body.push(format!("}}")); // end of if
                body.push(format!("}} else {{"));
                body.push(format!(
                    "ret = match rm_{}.update(rb).await {{",
                    tpc.api_handler_name.clone()
                ));
                body.push(format!("Ok(_) => {{")); //  begin of Ok
                body.push(format!("None"));
                body.push(format!("}}")); //  end of Ok
                body.push(format!("Err(err) => {{")); //  begin of none
                body.push(format!(
                    "log::info!(\"Remove {} occurred an error {{}}\", err);",
                    tpc.api_handler_name.clone()
                ));
                body.push(format!("Some(err)"));
                body.push(format!("}}")); //  end of error
                body.push(format!("}};")); // end of rm_{}
                body.push(format!("}}")); // end of if
                body.push(format!("}}")); // end of for
                body.push(format!("}}"));
            }
        }
    }

    body.push(format!("match ret {{"));
    body.push(format!("Some(err) => {{"));
    body.push(format!("Err(err)"));
    body.push(format!("}}"));
    body.push(format!("None => {{"));
    body.push(format!("Ok(true)"));
    body.push(format!("}}"));
    body.push(format!("}}"));

    RustFunc {
        is_struct_fn: true,
        is_self_fn: true,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: "save".to_string(),
        return_is_option: false,
        return_is_result: true,
        return_type: Some("bool".to_string()),
        params: params,
        bodylines: body,
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(format!("{}保存", tbl.comment.clone())),
        api_method: None,
        api_pattern: None,
    }
}

/**
 * 执行Delete操作
 */
fn generate_func_delete_for_relation(ctx: &GenerateContext, tbl: &RelationConfig) -> RustFunc {
    let tbl_name = tbl.major_table.clone();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&mut RBatisTxExecutor<'_>".to_string()));

    let mut body = vec![];

    body.push(format!("let mut ret: Option<Error> = None;"));
    for otp in tbl.one_to_one.clone() {
        let tpconf = ctx.get_table_conf(&otp.table_name.unwrap_or_default());
        if !otp.readonly && tpconf.is_some() {
            let tpc = tpconf.unwrap();
            body.push(format!("if ret.is_none() {{"));
            body.push(format!(
                "ret = match self.{}.clone() {{",
                tpc.api_handler_name.clone()
            ));
            body.push(format!("Some(tp) => {{")); // begin of Some
            body.push(format!("let mut mtp = tp.clone();"));
            body.push(format!("match mtp.remove(rb).await {{"));
            body.push(format!("Ok(_rtremove) => {{")); //  begin of Ok
            body.push(format!("None"));
            body.push(format!("}}")); //  end of Ok
            body.push(format!("Err(err) => {{")); //  begin of none
            body.push(format!(
                "log::info!(\"Remove {} occurred an error {{}}\", err);",
                tpc.api_handler_name.clone()
            ));
            body.push(format!("Some(err)"));
            body.push(format!("}}")); //  end of error
            body.push(format!("}}")); //  end of Some(tp)
            body.push(format!("}}")); //  end of Some(tp)
            body.push(format!("None => {{")); //  begin of none
            body.push(format!("None"));
            body.push(format!("}}")); //  end of None
            body.push(format!("}};")); // end of match option
            body.push(format!("}}")); // end of if
        }
    }

    for otp in tbl.one_to_many.clone() {
        let tpconf = if otp.middle_table.clone().is_none() {
            ctx.get_table_conf(&otp.table_name.clone().unwrap_or_default())
        } else {
            ctx.get_table_conf(&otp.middle_table.clone().unwrap_or_default())
        };
        let majtblconf = ctx.get_table_conf(&otp.table_name.clone().unwrap_or_default());

        if !otp.readonly && tpconf.is_some() {
            let _mtpc = majtblconf.unwrap();
            let tpc = tpconf.unwrap();
            body.push(format!("// remove batch for {}.", tpc.struct_name.clone()));

            body.push(format!("if ret.is_none() {{"));
            // if otp.middle_table.clone().is_some() {
            if tbl.extend_major {
                body.push(format!(
                    "let rm_{} = {} {{",
                    tpc.api_handler_name.clone(),
                    tpc.struct_name.clone()
                ));
                if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &otp.major_field.clone().unwrap_or_default().to_lowercase()) {
                    body.push(format!(
                        "{}: self.{},",
                        otp.major_field.clone().unwrap_or_default().to_lowercase(),
                        otp.major_field.clone().unwrap_or_default().to_lowercase()
                    ));
                } else {
                    body.push(format!(
                        "{}: self.{}.clone(),",
                        otp.major_field.clone().unwrap_or_default().to_lowercase(),
                        otp.major_field.clone().unwrap_or_default().to_lowercase()
                    ));
                }
                body.push(format!("..Default::default()"));
                body.push(format!("}};"));
            } else {
                body.push(format!(
                    "let rm_{} = {} {{",
                    tpc.api_handler_name.clone(),
                    tpc.struct_name.clone()
                ));
                if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &otp.major_field.clone().unwrap_or_default().to_lowercase()) {
                    body.push(format!(
                        "{}: self.{}.clone().unwrap().{},",
                        otp.join_field.unwrap_or_default().to_lowercase(),
                        tbconf.api_handler_name.clone(),
                        otp.major_field.unwrap_or_default().to_lowercase()
                    ));
                } else {
                    body.push(format!(
                        "{}: self.{}.clone().unwrap().{}.clone(),",
                        otp.join_field.unwrap_or_default().to_lowercase(),
                        tbconf.api_handler_name.clone(),
                        otp.major_field.unwrap_or_default().to_lowercase()
                    ));
                }
                body.push(format!("..Default::default()"));
                body.push(format!("}};"));
            }
            //}
            body.push(format!(
                "ret = match rm_{}.remove_batch(rb).await {{",
                tpc.api_handler_name.clone()
            ));
            body.push(format!("Ok(_rtremove) => {{")); //  begin of Ok
            body.push(format!("None"));
            body.push(format!("}}")); //  end of Ok
            body.push(format!("Err(err) => {{")); //  begin of none
            body.push(format!(
                "log::info!(\"Remove {} occurred an error {{}}\", err);",
                tpc.api_handler_name.clone()
            ));
            body.push(format!("Some(err)"));
            body.push(format!("}}")); //  end of error
            body.push(format!("}};")); // end of rm_{}
            body.push(format!("}}")); // end of if
        }
    }

    body.push(format!("if let Some(ret) = ret {{"));
    body.push(format!("Err(ret)"));
    body.push(format!("}} else {{")); // end of if
    body.push(format!(
        "match self.to_{}().remove(rb).await {{",
        tbconf.api_handler_name.clone()
    ));
    body.push(format!("Ok(_rs) => {{")); //  begin of Ok
    body.push(format!("Ok(true)"));
    body.push(format!("}}")); //  end of Ok
    body.push(format!("Err(err) => {{")); //  begin of none
    body.push(format!(
        "log::info!(\"Remove {} occurred an error {{}}\", err);",
        tbconf.api_handler_name.clone()
    ));
    body.push(format!("Err(err)"));
    body.push(format!("}}")); //  end of error
    body.push(format!("}}")); // end of if
    body.push(format!("}}")); // end of if

    RustFunc {
        is_struct_fn: true,
        is_self_fn: true,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: "remove".to_string(),
        return_is_option: false,
        return_is_result: true,
        return_type: Some("bool".to_string()),
        params: params,
        bodylines: body,
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(format!("{}删除", tbl.comment.clone())),
        api_method: None,
        api_pattern: None,
    }
}

/**
 * 执行Delete操作 (同时删除多条记录)
 */
fn generate_func_delete_ids_for_relation(ctx: &GenerateContext, tbl: &RelationConfig) -> RustFunc {
    let tbl_name = tbl.major_table.clone();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tblinfo.unwrap();
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&mut RBatisTxExecutor<'_>".to_string()));

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

    // if ctx.codegen_conf.multi_tenancy {
    params.push(("cond".to_string(), format!("&Self")));
    //}

    let mut body = vec![];

    body.push(format!("let mut ret: Option<Error> = None;"));
    if ctx.codegen_conf.multi_tenancy {
        body.push(format!(
            "let mines = {}::load_ids(rb.get_rbatis(), ids, &cond.to_{}()).await?;",
            tbconf.struct_name.clone(),
            tbconf.api_handler_name.clone()
        ));
    } else {
        body.push(format!(
            "let mines = {}::load_ids(rb.get_rbatis(), ids).await?;",
            tbconf.struct_name.clone()
        ));
    }
    body.push(format!("let my_ids_list = mines.into_iter().map(|f| f.{}.unwrap_or_default()).collect::<Vec<i64>>();", tbconf.primary_key.clone()));
    body.push(format!("let my_ids = my_ids_list.as_slice();"));

    for otp in tbl.one_to_one.clone() {
        let stpconf = ctx.get_table_conf(&otp.table_name.clone().unwrap_or_default());
        if !otp.readonly && stpconf.is_some() {
            let tpc = stpconf.unwrap();
            body.push(format!("// remove batch for {}.", tpc.struct_name.clone()));

            body.push(format!("if ret.is_none() {{"));
            // if otp.middle_table.clone().is_some() {
            if tbl.extend_major {
                body.push(format!(
                    "let rm_{} = {} {{",
                    tpc.api_handler_name.clone(),
                    tpc.struct_name.clone()
                ));
                if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &otp.major_field.clone().unwrap_or_default().to_lowercase()) {
                    body.push(format!(
                        "{}: cond.{},",
                        otp.major_field.clone().unwrap_or_default().to_lowercase(),
                        otp.major_field.clone().unwrap_or_default().to_lowercase()
                    ));
                } else {
                    body.push(format!(
                        "{}: cond.{}.clone(),",
                        otp.major_field.clone().unwrap_or_default().to_lowercase(),
                        otp.major_field.clone().unwrap_or_default().to_lowercase()
                    ));
                }
                body.push(format!("..Default::default()"));
                body.push(format!("}};"));
            } else {
                body.push(format!(
                    "let rm_{} = {} {{",
                    tpc.api_handler_name.clone(),
                    tpc.struct_name.clone()
                ));
                if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &otp.major_field.clone().unwrap_or_default().to_lowercase()) {
                    body.push(format!(
                        "{}: cond.{}.clone().unwrap().{},",
                        otp.join_field.unwrap_or_default().to_lowercase(),
                        tbconf.api_handler_name.clone(),
                        otp.major_field.unwrap_or_default().to_lowercase()
                    ));
                } else {
                    body.push(format!(
                        "{}: cond.{}.clone().unwrap().{}.clone(),",
                        otp.join_field.unwrap_or_default().to_lowercase(),
                        tbconf.api_handler_name.clone(),
                        otp.major_field.unwrap_or_default().to_lowercase()
                    ));
                }
                body.push(format!("..Default::default()"));
                body.push(format!("}};"));
            }
            //}

            body.push(format!(
                "ret = match {}::remove_{}_ids(rb, my_ids, &rm_{}).await {{",
                tpc.struct_name.clone(),
                tbconf.api_handler_name.clone(),
                tpc.api_handler_name.clone()
            ));

            body.push(format!("Ok(_rtremove) => {{")); //  begin of Ok
            body.push(format!("None"));
            body.push(format!("}}")); //  end of Ok
            body.push(format!("Err(err) => {{")); //  begin of none
            body.push(format!(
                "log::info!(\"Remove {} occurred an error {{}}\", err);",
                tpc.api_handler_name.clone()
            ));
            body.push(format!("Some(err)"));
            body.push(format!("}}")); //  end of error
            body.push(format!("}};")); // end of rm_{}
            body.push(format!("}}")); // end of if
        }
    }

    for otp in tbl.one_to_many.clone() {
        let tpconf = if otp.middle_table.clone().is_none() {
            ctx.get_table_conf(&otp.table_name.clone().unwrap_or_default())
        } else {
            ctx.get_table_conf(&otp.middle_table.clone().unwrap_or_default())
        };
        let majtblconf = ctx.get_table_conf(&otp.table_name.clone().unwrap_or_default());

        if !otp.readonly && tpconf.is_some() {
            let _mtpc = majtblconf.unwrap();
            let tpc = tpconf.unwrap();
            body.push(format!("// remove batch for {}.", tpc.struct_name.clone()));

            body.push(format!("if ret.is_none() {{"));
            // if otp.middle_table.clone().is_some() {
            if tbl.extend_major {
                body.push(format!(
                    "let rm_{} = {} {{",
                    tpc.api_handler_name.clone(),
                    tpc.struct_name.clone()
                ));
                if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &otp.major_field.clone().unwrap_or_default().to_lowercase()) {
                    body.push(format!(
                        "{}: cond.{},",
                        otp.major_field.clone().unwrap_or_default().to_lowercase(),
                        otp.major_field.clone().unwrap_or_default().to_lowercase()
                    ));
                } else {
                    body.push(format!(
                        "{}: cond.{}.clone(),",
                        otp.major_field.clone().unwrap_or_default().to_lowercase(),
                        otp.major_field.clone().unwrap_or_default().to_lowercase()
                    ));                    
                }
                body.push(format!("..Default::default()"));
                body.push(format!("}};"));
            } else {
                body.push(format!(
                    "let rm_{} = {} {{",
                    tpc.api_handler_name.clone(),
                    tpc.struct_name.clone()
                ));
                if ctx.is_copied_data_type(&otp.table_name.clone().unwrap_or_default(), &otp.major_field.clone().unwrap_or_default().to_lowercase()) {
                    body.push(format!(
                        "{}: cond.{}.clone().unwrap().{},",
                        otp.join_field.unwrap_or_default().to_lowercase(),
                        tbconf.api_handler_name.clone(),
                        otp.major_field.unwrap_or_default().to_lowercase()
                    ));
                } else {
                    body.push(format!(
                        "{}: cond.{}.clone().unwrap().{}.clone(),",
                        otp.join_field.unwrap_or_default().to_lowercase(),
                        tbconf.api_handler_name.clone(),
                        otp.major_field.unwrap_or_default().to_lowercase()
                    ));
                }
                body.push(format!("..Default::default()"));
                body.push(format!("}};"));
            }
            //}

            body.push(format!(
                "ret = match {}::remove_{}_ids(rb, my_ids, &rm_{}).await {{",
                tpc.struct_name.clone(),
                tbconf.api_handler_name.clone(),
                tpc.api_handler_name.clone()
            ));

            body.push(format!("Ok(_rtremove) => {{")); //  begin of Ok
            body.push(format!("None"));
            body.push(format!("}}")); //  end of Ok
            body.push(format!("Err(err) => {{")); //  begin of none
            body.push(format!(
                "log::info!(\"Remove {} occurred an error {{}}\", err);",
                tpc.api_handler_name.clone()
            ));
            body.push(format!("Some(err)"));
            body.push(format!("}}")); //  end of error
            body.push(format!("}};")); // end of rm_{}
            body.push(format!("}}")); // end of if
        }
    }

    body.push(format!("if let Some(ret) = ret {{"));
    body.push(format!("Err(ret)"));
    body.push(format!("}} else {{"));
    if ctx.codegen_conf.multi_tenancy {
        body.push(format!(
            "match {}::remove_ids(rb, my_ids, &cond.to_{}()).await {{",
            tbconf.struct_name.clone(),
            tbconf.api_handler_name.clone()
        ));
    } else {
        body.push(format!(
            "match {}::remove_ids(rb, my_ids).await {{",
            tbconf.struct_name.clone()
        ));
    }
    body.push(format!("Ok(_rs) => {{")); //  begin of Ok
    body.push(format!("Ok(true)"));
    body.push(format!("}}")); //  end of Ok
    body.push(format!("Err(err) => {{")); //  begin of none
    body.push(format!(
        "log::info!(\"Remove {} occurred an error {{}}\", err);",
        tbconf.api_handler_name.clone()
    ));
    body.push(format!("Err(err)"));
    body.push(format!("}}")); //  end of error
    body.push(format!("}}")); // end of if
    body.push(format!("}}")); // end of if

    RustFunc {
        is_struct_fn: true,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: "remove_rel_ids".to_string(),
        return_is_option: false,
        return_is_result: true,
        return_type: Some("bool".to_string()),
        params: params,
        bodylines: body,
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(format!("{}删除", tbl.comment.clone())),
        api_method: None,
        api_pattern: None,
    }
}

/**
 * 生成Relation加载load 的Handler
 */
pub fn generate_handler_load_for_relation(ctx: &GenerateContext, tbl: &RelationConfig) -> RustFunc {
    let tbl_name = tbl.major_table.clone();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let table_info = ctx.get_table_info(&tbl_name.clone());
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    let mut params_text = String::new();
    let mut macrotext = String::new();
    let mut somebody = vec![];

    let has_su = process_detail_tenancy_fields(
        ctx,
        &mut somebody,
        &table_info.unwrap(),
        &tbl.struct_name.clone(),
        0,
    );

    if ctx.codegen_conf.multi_tenancy {
        if has_su {
            params.push(("su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));
        } else {
            params.push(("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));
        }
    }

    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    for col in pkcols.clone() {
        let dt = parse_data_type_as_rust_type(&col.data_type.unwrap_or_default());
        params.push((
            col.column_name.clone().unwrap_or_default().to_lowercase(),
            format!("web::Path<{}>", dt.clone()),
        ));
        params_text.push_str(
            format!(
                "&{}",
                col.column_name.clone().unwrap_or_default().to_lowercase()
            )
            .as_str(),
        );
        params_text.push_str(",");
        macrotext.push_str("/");
        macrotext.push_str(
            format!(
                "{{{}}}",
                col.column_name.clone().unwrap_or_default().to_lowercase()
            )
            .as_str(),
        );
    }

    if params_text.ends_with(",") {
        params_text = params_text.substring(0, params_text.len() - 1).to_string();
    }

    let _tbc = tblinfo.unwrap();

    let mut body = vec![];

    body.push(format!("let rb = get_rbatis();"));
    body.push(format!(
        "match {}::load(rb, {}).await {{",
        tbl.struct_name.clone(),
        params_text
    ));
    body.push(format!("Ok(st) => {{"));
    if has_su { // TODO: May be some logics for multi_tenancy
        body.push(format!("if let Some(mut tv) = st.clone() {{"));
        body.append(&mut somebody);
        body.push(format!("log::debug!(\"avoid warning: {{}}\", tv);"));
        body.push(format!("}}"));
    }
    body.push(format!(
        "let ret: web::Json<ApiResult<Option<{}>>> = web::Json(ApiResult::ok(st));",
        tbl.struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!("let ret: web::Json<ApiResult<Option<{}>>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl.struct_name.clone()));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));

    let url_pattern = format!(
        "{}/{}/load{}",
        ctx.codegen_conf.api_handler_prefix.clone(),
        tbl.api_handler_name.clone().unwrap_or_default(),
        macrotext
    );
    let postmacro = format!("#[get(\"{}\")]", url_pattern.clone());

    let func_name = format!(
        "{}_rel_load",
        tbl.api_handler_name.clone().unwrap_or_default()
    );

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
        comment: Some(format!("{}加载", tbl.comment.clone())),
        api_method: Some("GET".to_string()),
        api_pattern: Some(url_pattern.clone()),
    }
}

/**
 * 生成Relation Delete 的Handler
 */
pub fn generate_handler_remove_for_relation(
    ctx: &GenerateContext,
    tbl: &RelationConfig,
) -> RustFunc {
    let tbl_name = tbl.major_table.clone();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let table_info = ctx.get_table_info(&tbl_name.clone());
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    let mut params_text = String::new();
    let mut macrotext = String::new();
    let mut somebody = vec![];

    let has_su = process_detail_tenancy_fields(
        ctx,
        &mut somebody,
        &table_info.unwrap(),
        &tbl.struct_name.clone(),
        0,
    );

    if has_su {
        params.push(("su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));
    } else {
        params.push(("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));
    }

    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    for col in pkcols.clone() {
        let dt = parse_data_type_as_rust_type(&col.data_type.unwrap_or_default());
        params.push((
            col.column_name.clone().unwrap_or_default().to_lowercase(),
            format!("web::Path<{}>", dt.clone()),
        ));
        params_text.push_str(
            format!(
                "&{}",
                col.column_name.clone().unwrap_or_default().to_lowercase()
            )
            .as_str(),
        );
        params_text.push_str(",");
        macrotext.push_str("/");
        macrotext.push_str(
            format!(
                "{{{}}}",
                col.column_name.clone().unwrap_or_default().to_lowercase()
            )
            .as_str(),
        );
    }

    if params_text.ends_with(",") {
        params_text = params_text.substring(0, params_text.len() - 1).to_string();
    }

    let _tbc = tblinfo.unwrap();

    let mut body = vec![];

    body.push(format!("let rb = get_rbatis();"));
    body.push(format!(
        "match {}::load(rb, {}).await {{",
        tbl.struct_name.clone(),
        params_text
    ));
    body.push(format!("Ok(st) => {{"));
    body.push(format!("match st {{"));
    body.push(format!("Some(cst) => {{"));
    if has_su {  // TODO: Maybe there are some logic for tenancy
        body.push(format!("let mut tv = cst.clone();"));
        body.append(&mut somebody);
        body.push(format!("log::debug!(\"avoid warning: {{}}\", tv);"));
    }
    body.push(format!("match rb.acquire_begin().await {{"));
    body.push(format!("Ok(mut tx) => {{"));
    body.push(format!("match cst.remove(&mut tx).await {{"));
    body.push(format!("Ok(_) => {{"));
    body.push(format!("match tx.commit().await {{"));
    body.push(format!("Ok(_) => {{"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::ok(cst));",
        tbl.struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5011, &err.to_string()));",
        tbl.struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!("let _ = tx.rollback().await.is_ok();"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5011, &err.to_string()));",
        tbl.struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));",
        tbl.struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("None => {{"));
    body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5040, &\"Not-Found\".to_string()));", tbl.struct_name.clone()));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));",
        tbl.struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    let url_pattern = format!(
        "{}/{}/remove{}",
        ctx.codegen_conf.api_handler_prefix.clone(),
        tbl.api_handler_name.clone().unwrap_or_default(),
        macrotext
    );
    let postmacro = format!("#[post(\"{}\")]", url_pattern.clone());

    let func_name = format!(
        "{}_rel_remove",
        tbl.api_handler_name.clone().unwrap_or_default()
    );

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
        comment: Some(format!("{}删除", tbl.comment.clone())),
        api_method: Some("POST".to_string()),
        api_pattern: Some(url_pattern.clone()),
    }
}

/**
 * 生成Relation Delete 的Handler
 */
pub fn generate_handler_remove_multi_for_relation(
    ctx: &GenerateContext,
    tbl: &RelationConfig,
) -> RustFunc {
    let tbl_name = tbl.major_table.clone();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let table_info = ctx.get_table_info(&tbl_name.clone());
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    let mut params_text = String::new();
    let _macrotext = String::new();

    let mut somebody = vec![];
    let has_su = process_detail_common_fields_v2(
        ctx,
        "cond",
        &mut somebody,
        &table_info.clone().unwrap_or_default(),
        0,
    );

    if has_su {
        params.push(("su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));
    } else {
        params.push(("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));
    }

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

    if params_text.ends_with(",") {
        params_text = params_text.substring(0, params_text.len() - 1).to_string();
    }

    log::debug!("Avoid warning: {}", params_text.clone());

    let _tbc = tblinfo.unwrap();

    let mut body = vec![];

    body.push(format!("let rb = get_rbatis();"));
    body.push(format!("let ids = req.as_slice();"));
    body.push(format!("let cond = {} {{", tbl.struct_name.clone()));

    body.append(&mut somebody);
    body.push(format!("..Default::default()"));
    body.push(format!("}};"));

    body.push(format!("match rb.acquire_begin().await {{"));
    body.push(format!("Ok(mut tx) => {{"));
    body.push(format!(
        "match {}::remove_rel_ids(&mut tx, ids, &cond).await {{",
        tbl.struct_name.clone()
    ));
    body.push(format!("Ok(_st) => {{"));
    body.push(format!("match tx.commit().await {{"));
    body.push(format!("Ok(_) => {{"));
    body.push(format!("let ret: web::Json<ApiResult<String>> = web::Json(ApiResult::ok(\"SUCCESS\".to_string()));"));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5011, &err.to_string()));",
        tbl.struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!("let _ = tx.rollback().await.is_ok();"));
    body.push(format!("let ret: web::Json<ApiResult<String>> = web::Json(ApiResult::error(5010, &err.to_string()));"));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!("let ret: web::Json<ApiResult<String>> = web::Json(ApiResult::error(5010, &err.to_string()));"));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));

    let url_pattern = format!(
        "{}/{}/multi/remove",
        ctx.codegen_conf.api_handler_prefix.clone(),
        tbl.api_handler_name.clone().unwrap_or_default()
    );
    let postmacro = format!("#[post(\"{}\")]", url_pattern.clone());

    let func_name = format!(
        "{}_rel_remove_multi",
        tbl.api_handler_name.clone().unwrap_or_default()
    );

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
        comment: Some(format!("{}删除", tbl.comment.clone())),
        api_method: Some("POST".to_string()),
        api_pattern: Some(url_pattern.clone()),
    }
}

/**
 * 生成Relation保存的Handler
 */
pub fn generate_handler_save_for_relation(ctx: &GenerateContext, tbl: &RelationConfig) -> RustFunc {
    let tbl_name = tbl.major_table.clone();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    let _table_info = ctx.get_table_info(&tbl_name.clone());
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let xccols = ctx.get_table_columns(&tbl_name.clone());

    let mut params = Vec::new();

    params.push(("su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));

    params.push((
        "req".to_string(),
        format!("web::Json<{}>", tbl.struct_name.clone()),
    ));

    let _tbc = tblinfo.unwrap();

    let mut body = vec![];

    body.push(format!("let rb = get_rbatis();"));
    body.push(format!("let mut val = req.to_owned();"));
    body.push(format!("val.refine(&su);"));
    for xc in xccols.clone() {
        let xcname = xc.column_name.unwrap_or_default();
        if xcname == "company_id".to_string() || xcname == "company_code".to_string() {
            body.push(format!(
                "if val.{} != su.user.{} {{",
                xcname.clone(),
                xcname.clone()
            ));
            body.push(format!("let ret: web::Json<ApiResult<String>> = web::Json(ApiResult::error(5403, &\"非法处理\".to_string()));"));
            body.push(format!("return Ok(HttpResponse::Ok().json(ret));"));
            body.push(format!("}}"));
        }
    }
    body.push(format!("match rb.acquire_begin().await {{"));
    body.push(format!("Ok(mut tx) => {{"));
    body.push(format!("match val.save(&mut tx).await {{"));
    body.push(format!("Ok(_st) => {{"));
    body.push(format!("match tx.commit().await {{"));
    body.push(format!("Ok(_) => {{"));
    body.push(format!("let ret: web::Json<ApiResult<String>> = web::Json(ApiResult::ok(\"SUCCESS\".to_string()));"));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!(
        "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5011, &err.to_string()));",
        tbl.struct_name.clone()
    ));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!("let _ = tx.rollback().await.is_ok();"));
    body.push(format!("let ret: web::Json<ApiResult<String>> = web::Json(ApiResult::error(5010, &err.to_string()));"));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!("let ret: web::Json<ApiResult<String>> = web::Json(ApiResult::error(5010, &err.to_string()));"));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    let url_pattern = format!(
        "{}/{}/rel/save",
        ctx.codegen_conf.api_handler_prefix.clone(),
        tbl.api_handler_name.clone().unwrap_or_default()
    );
    let postmacro = format!("#[post(\"{}\")]", url_pattern.clone());

    let func_name = format!(
        "{}_rel_save",
        tbl.api_handler_name.clone().unwrap_or_default()
    );

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
        comment: Some(format!("{}保存", tbl.comment.clone())),
        api_method: Some("POST".to_string()),
        api_pattern: Some(url_pattern.clone()),
    }
}

pub fn tera_pascal(param: &HashMap<String, serde_json::Value>) -> Result<Value, tera::Error> {
    //for (lx, mp) in param.clone() {
    // log::info!("Key: {} Value: {}", lx, mp);
    // }
    let value = param.get(&"str".to_string());
    if value.is_some() {
        let text = value.unwrap().as_str().unwrap_or_default();
        Ok(json!(pascal_case(text)))
    } else {
        Ok(json!(""))
    }
}

pub fn generate_relation_form(ctx: &mut GenerateContext, rel: &RelationConfig) -> String {
    let mut tera = TEMPLATES.clone();

    tera.register_function("pascal", tera_pascal);

    let tbl = ctx.get_table_info(&rel.major_table);
    let tbc = ctx.get_table_conf(&rel.major_table);

    let cols = ctx.get_table_columns(&rel.major_table);

    let mut relform = RelationForm::default();
    relform.codegen = ctx.codegen_conf.clone();
    relform.table_info = tbl.clone();
    relform.table_conf = tbc.clone();
    relform.relation_conf = Some(rel.clone());
    let mut columns = String::new();
    let mut joinlist = String::new();
    let mut usings = vec![];
    relform.fields = parse_composite_column_list(
        ctx,
        &tbc.unwrap(),
        &cols,
        &mut columns,
        &mut joinlist,
        true,
        true,
        &mut usings,
    );
    for cp in relform.fields.clone() {
        if cp.relation.is_some() {
            let relation_table_name = cp.relation.unwrap();
            let relconf = ctx.get_table_conf(&relation_table_name);
            if relconf.is_some() {
                relform
                    .relation_map
                    .insert(relation_table_name.clone(), relconf.unwrap());
            }
        }
        if cp.dict.is_some() {
            if cp.dict == Some("area".to_string()) {
                relform.has_area = true;
            } else {
                relform.dict_list.push(cp.dict.clone().unwrap_or_default());
            }
        }
    }

    for cl in rel.one_to_one.clone() {
        let rel_table_name = cl.table_name.unwrap_or_default();
        let reltbl = ctx.get_table_info(&rel_table_name);
        let reltbc = ctx.get_table_conf(&rel_table_name);
        let mut relcolumns = String::new();
        let mut reljoinlist = String::new();
        let rel_cols = ctx.get_table_columns(&rel_table_name);
        let relfields = parse_composite_column_list(
            ctx,
            &reltbc.clone().unwrap(),
            &rel_cols,
            &mut relcolumns,
            &mut reljoinlist,
            true,
            false,
            &mut usings,
        );
        let reltable = RelationTable {
            table_conf: reltbc.clone(),
            table_info: reltbl.clone(),
            major_field_name: Some(format!(
                "{}",
                safe_struct_field_name(&reltbc.clone().unwrap().api_handler_name)
            )),
            one_many: false,
            dialog_form: false,
            fields: relfields.clone(),
        };
        relform.relations.push(reltable.clone());

        for cp in reltable.fields.clone() {
            if cp.relation.is_some() {
                let relation_table_name = cp.relation.unwrap();
                let relconf = ctx.get_table_conf(&relation_table_name);
                if relconf.is_some() {
                    relform
                        .relation_map
                        .insert(relation_table_name.clone(), relconf.unwrap());
                }
            }
            if cp.dict.is_some() {
                if cp.dict == Some("area".to_string()) {
                    relform.has_area = true;
                } else {
                    relform.dict_list.push(cp.dict.clone().unwrap_or_default());
                }
            }
        }
    }

    for cl in rel.one_to_many.clone() {
        let rel_table_name = cl.table_name.unwrap_or_default();
        let reltbl = ctx.get_table_info(&rel_table_name);
        let reltbc = ctx.get_table_conf(&rel_table_name);
        let mut relcolumns = String::new();
        let mut reljoinlist = String::new();
        let rel_cols = ctx.get_table_columns(&rel_table_name);
        let relfields = parse_composite_column_list(
            ctx,
            &reltbc.clone().unwrap(),
            &rel_cols,
            &mut relcolumns,
            &mut reljoinlist,
            true,
            false,
            &mut usings,
        );
        let reltable = RelationTable {
            table_conf: reltbc.clone(),
            table_info: reltbl.clone(),
            major_field_name: Some(format!(
                "{}s",
                safe_struct_field_name(&reltbc.clone().unwrap().api_handler_name)
            )),
            one_many: true,
            dialog_form: cl.use_dialog_form,
            fields: relfields.clone(),
        };

        relform.relations.push(reltable.clone());
        for cp in reltable.fields.clone() {
            if cp.relation.is_some() {
                let relation_table_name = cp.relation.unwrap();
                let relconf = ctx.get_table_conf(&relation_table_name);
                if relconf.is_some() {
                    relform
                        .relation_map
                        .insert(relation_table_name.clone(), relconf.unwrap());
                }
            }
            if cp.dict.is_some() {
                if cp.dict == Some("area".to_string()) {
                    relform.has_area = true;
                } else {
                    relform.dict_list.push(cp.dict.clone().unwrap_or_default());
                }
            }
        }
    }

    relform.dict_list = relform
        .dict_list
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    relform.relation_count = relform.relations.len() as u64;

    let context = match Context::from_serialize(&relform) {
        Ok(c) => c,
        Err(_) => Context::new(),
    };

    match tera.render("form.vue", &context) {
        Ok(text) => text,
        Err(err) => {
            log::info!(
                "Error for parse form.vue for {} the template: {}",
                rel.major_table.clone(),
                err.to_string()
            );
            String::new()
        }
    }
}
