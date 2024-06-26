use crate::codegen::{
    parse_data_type_annotions, parse_data_type_as_rust_type, GenerateContext, RustFunc,
};
use crate::config::{safe_struct_field_name, AppConfig, QueryConfig};
use change_case::snake_case;
use sqlx::Column;
use sqlx::Row;
use sqlx::TypeInfo;
use substring::Substring;

use super::{is_copied_data_type, CodeGenerator, CodeModelType, RustFileImpl, RustStruct, RustStructField};

pub struct TransformRow {
    pub fields: Vec<RustStructField>,
    pub columns: String,
    pub usings: Vec<String>,
}

pub async fn execute_sql(
    ctx: &GenerateContext,
    sql: &str,
    fds: &Vec<String>,
) -> Result<TransformRow, sqlx::Error> {
    let conf = AppConfig::get().lock().unwrap().to_owned();
    log::info!("Connection:{}", conf.mysql_conf.url.clone().as_str());
    let driver = conf.mysql_conf.url.clone();
    // let driver = "mysql://chimes:ks123456@localhost/swms".to_string();

    let pool = sqlx::MySqlPool::connect_lazy(driver.replace("127.0.0.1", "localhost").as_str())?;

    match pool.acquire().await {
        Ok(cn) => {
            let mut mcn = cn;
            let mut tfrow = TransformRow {
                fields: vec![],
                columns: String::new(),
                usings: vec![],
            };
            let mut qry = sqlx::query(sql);
            for fs in fds.clone() {
                qry = qry.bind(fs);
            }
            log::info!("Connection Prepared.");
            match qry.fetch_one(&mut mcn).await {
                Ok(rs) => {
                    let mut column_text = String::new();
                    for col in rs.columns().into_iter() {
                        // let tid = col.type_id().to_owned();
                        column_text.push_str(col.name().to_string().as_str());
                        column_text.push_str(",");
                        // log::info!("Column: {} type is {}.", col.name().clone().to_string(), col.type_info().clone().name().to_string().to_lowercase());
                        let field_type = parse_data_type_as_rust_type(
                            &col.type_info().name().to_string().to_lowercase(),
                        );
                        // let mut usings = vec![];
                        let annts = parse_data_type_annotions(ctx, &field_type, &mut tfrow.usings);
                        let rsf = RustStructField {
                            is_pub: true,
                            schema_name: None,
                            column_name: col.name().to_string(),
                            field_name: safe_struct_field_name(
                                &col.name().to_string().to_lowercase(),
                            ),
                            field_type: field_type,
                            is_option: true,
                            orignal_field_name: None,
                            comment: None,
                            length: 0i64,
                            annotations: annts,
                        };
                        tfrow.fields.push(rsf);
                        tfrow.columns = column_text.substring(0, column_text.len() - 1).to_string();
                    }
                    Ok(tfrow)
                }
                Err(err) => Err(err),
            }
        }
        Err(err) => {
            log::info!("Acquire connection: {}", err);
            Err(err)
        }
    }
}

pub fn parse_query_as_file(
    ctx: &GenerateContext,
    tbl: &QueryConfig,
    cols: &TransformRow,
) -> RustFileImpl {
    let mut usinglist = CodeGenerator::get_default_entity_using(ctx, !tbl.single_result, false, CodeModelType::Query);
    let st = parse_query_as_struct(ctx, tbl, cols, &mut usinglist);
    let st_params = parse_query_params_as_struct(ctx, tbl, &mut usinglist);

    RustFileImpl {
        file_name: snake_case(tbl.struct_name.clone().as_str()) + ".rs",
        mod_name: "query".to_string(),
        caretlist: vec![],
        usinglist: usinglist,
        structlist: vec![st_params, st],
        funclist: vec![],
    }
}

/**
 * 解析查询结果成为struct
 * 这个要求该查询的结果返回必须至少有一条数据，才能准确分析出该struct
 */
pub fn parse_query_as_struct(
    ctx: &GenerateContext,
    tbl: &QueryConfig,
    cols: &TransformRow,
    usings: &mut Vec<String>
) -> RustStruct {
    let fields = cols.fields.clone();

    let crudtbl = format!(
        "#[crud_table(table_name:\"{}\"|table_columns:\"{}\")]",
        tbl.struct_name.clone(),
        cols.columns.clone()
    );
    let anno = vec![
        crudtbl,
        "#[derive(Debug, Clone, Default, Deserialize, Serialize)]".to_string(),
    ];

    let mut funclist = vec![];

    if tbl.single_result {
        let onerowfunc = parse_query_as_func(ctx, tbl, false, true);
        funclist.push(onerowfunc);
    } else {
        let queryfunc = parse_query_as_func(ctx, tbl, false, false);
        let pagedfunc = parse_query_as_func(ctx, tbl, true, false);

        funclist.push(queryfunc);
        funclist.push(pagedfunc);
    }

    let mut ctmut_usings = cols.usings.clone();
    usings.append(&mut ctmut_usings);

    RustStruct {
        is_pub: true,
        has_paging: !tbl.single_result,
        struct_name: tbl.struct_name.clone(),
        annotations: anno,
        fields: fields,
        funclist: funclist,
        usings: usings.clone()
    }
}

/**
 * 解析查询参数成为struct
 */
pub fn parse_query_params_as_struct(ctx: &GenerateContext, tbl: &QueryConfig, usings: &mut Vec<String>) -> RustStruct {
    let mut fields = vec![];
    let anno = vec!["#[derive(Debug, Clone, Default, Deserialize, Serialize)]".to_string()];
    let funclist = vec![];

    for fd in tbl.params.clone() {
        let field_type = parse_data_type_as_rust_type(&fd.column_types.unwrap());
        let annts = parse_data_type_annotions(ctx, &field_type, usings);
        let st = RustStructField {
            is_pub: true,
            schema_name: None,
            column_name: String::new(),
            field_name: safe_struct_field_name(&fd.column_names.unwrap()),
            field_type: field_type,
            is_option: true,
            orignal_field_name: None,
            comment: None,
            length: 0i64,
            annotations: annts,
        };
        fields.push(st);
    }

    for fd in tbl.variant_params.clone() {
        match fd.column_names {
            Some(cn) => match fd.column_types {
                Some(ct) => {
                    let cns: Vec<String> = cn
                        .clone()
                        .split(",")
                        .into_iter()
                        .map(|f| f.to_string())
                        .collect();
                    let cts: Vec<String> = ct
                        .clone()
                        .split(",")
                        .into_iter()
                        .map(|f| f.to_string())
                        .collect();
                    if cts.len() == cns.len() {
                        let mut ii = 0;
                        while ii < cts.len() {
                            let fdname = cns[ii].clone();
                            let fdtype = cts[ii].clone();
                            let field_type = parse_data_type_as_rust_type(&fdtype);

                            let st = RustStructField {
                                is_pub: true,
                                schema_name: None,
                                column_name: String::new(),
                                field_name: safe_struct_field_name(&fdname),
                                field_type: field_type.clone(),
                                is_option: true,
                                orignal_field_name: None,
                                comment: None,
                                length: 0i64,
                                annotations: parse_data_type_annotions(ctx, &field_type, usings),
                            };
                            fields.push(st);
                            ii += 1;
                        }
                    } else {
                        log::info!("Variant Param's name and type are not matched. count({}) != count({}).", cn.clone(), ct.clone());
                    }
                }
                None => {}
            },
            None => {}
        };
    }

    RustStruct {
        is_pub: true,
        has_paging: !tbl.single_result,
        struct_name: tbl.struct_name.clone() + "Params",
        annotations: anno,
        fields: fields,
        funclist: funclist,
        usings: usings.clone(),
    }
}

pub fn parse_query_as_func(
    _ctx: &GenerateContext,
    tbl: &QueryConfig,
    paging: bool,
    onerow: bool,
) -> RustFunc {
    let mut params = vec![];
    let mut body = vec![];
    let param_type = tbl.struct_name.clone() + "Params";

    params.push(("rb".to_string(), "&Rbatis".to_string()));
    params.push(("param".to_string(), format!("&{}", param_type)));
    if paging {
        params.push(("curr".to_string(), "u64".to_string()));
        params.push(("size".to_string(), "u64".to_string()));
    }

    if tbl.variant_params.is_empty() {
        body.push(format!("let mut sql = \"{}\".to_string();", tbl.base_sql));
    } else {
        body.push(format!("let mut sql = \"{}\".to_string();", tbl.base_sql));
    }
    body.push("let mut rb_args = vec![];".to_string());

    for sp in tbl.params.clone() {
        let fd_type = sp.column_types.unwrap_or_default();
        match sp.column_names {
            Some(spn) => {
                body.push(format!("sql.push_str(\" and {} = ?\");", spn.clone()));
                if is_copied_data_type(&fd_type) {
                    body.push(format!("rb_args.push(rbson::to_bson(param.{}.unwrap_or_default()).unwrap_or_default());", safe_struct_field_name(&spn)));
                } else {
                    body.push(format!("rb_args.push(rbson::to_bson(param.{}.clone().unwrap_or_default()).unwrap_or_default());", safe_struct_field_name(&spn)));
                }
            }
            None => {}
        };
    }

    for sp in tbl.variant_params.clone() {
        match sp.column_names {
            Some(cn) => match sp.column_types {
                Some(ct) => {
                    let cns: Vec<String> = cn
                        .clone()
                        .split(",")
                        .into_iter()
                        .map(|f| f.to_string())
                        .collect();
                    let cts: Vec<String> = ct
                        .clone()
                        .split(",")
                        .into_iter()
                        .map(|f| f.to_string())
                        .collect();
                    if cts.len() == cns.len() {
                        let mut ii = 0;
                        let mut testexpr = String::new();
                        testexpr.push_str("if");
                        while ii < cts.len() {
                            let fdname = cns[ii].clone();
                            testexpr.push_str(
                                format!(" param.{}.is_some() &&", safe_struct_field_name(&fdname))
                                    .as_str(),
                            );
                            ii += 1;
                        }
                        if testexpr.ends_with("&&") {
                            testexpr = testexpr.substring(0, testexpr.len() - 2).to_string();
                            testexpr.push_str("{");
                            body.push(testexpr);
                        }
                        if sp.column_express.is_none() {
                            body.push(format!("sql.push_str(\" and {} = ? \");", cn.clone()));
                        } else {
                            body.push(format!(
                                "sql.push_str(\" {} \");",
                                sp.column_express.unwrap()
                            ));
                        }
                        ii = 0;
                        while ii < cts.len() {
                            let fdname = cns[ii].clone();
                            let fdtype = cts[ii].clone();
                            if is_copied_data_type(&fdtype) {
                                body.push(format!("rb_args.push(rbson::to_bson(param.{}.unwrap_or_default()).unwrap_or_default());", safe_struct_field_name(&fdname)));
                            } else {
                                body.push(format!("rb_args.push(rbson::to_bson(param.{}.clone().unwrap_or_default()).unwrap_or_default());", safe_struct_field_name(&fdname)));
                            }
                            ii += 1;
                        }
                        body.push("}".to_string());
                    } else {
                        log::info!("Variant Param's name and type are not matched. count({}) != count({}).", cn.clone(), ct.clone());
                    }
                }
                None => {}
            },
            None => {}
        };
    }

    if paging {
        body.push(
            "rb.fetch_page(&sql, rb_args, &PageRequest::new(curr, size)).await".to_string(),
        );
    } else {
        if onerow {
            body.push("rb.fetch(&sql, rb_args).await".to_string());
        } else {
            body.push("rb.fetch(&sql, rb_args).await".to_string());
        }
    }

    let ret_type = if paging {
        format!("Page<{}>", tbl.struct_name.clone())
    } else {
        if onerow {
            format!("Option<{}>", tbl.struct_name.clone())
        } else {
            format!("Vec<{}>", tbl.struct_name.clone())
        }
    };

    let comment = if paging {
        format!("{}分页", tbl.comment.clone())
    } else {
        if onerow {
            format!("{}获取", tbl.comment.clone())
        } else {
            format!("{}列表", tbl.comment.clone())
        }
    };

    RustFunc {
        is_struct_fn: true,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: true,
        func_name: if paging {
            "query_paged".to_string()
        } else {
            "query".to_string()
        },
        return_is_option: false,
        return_is_result: true,
        return_type: Some(ret_type),
        params: params,
        bodylines: body,
        macros: vec!["#[allow(dead_code)]".to_string()],
        comment: Some(comment.clone()),
        api_method: None,
        api_pattern: None,
    }
}

/**
 * 生成Query 的Handler
 */
pub fn generate_handler_query_for_query(
    ctx: &GenerateContext,
    tbl: &QueryConfig,
    paging: bool,
    onerow: bool,
) -> RustFunc {
    let tbl_name = tbl.struct_name.clone();
    let tbl_param_name = format!("{}Params", tbl_name);

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);

    let mut has_cond = false;
    let mut somebody = vec![];
    if ctx.codegen_conf.multi_tenancy {
        
        for col in tbl.params.clone() {
            if col.column_names == Some("company_id".to_string())
                || col.column_names == Some("company_code".to_string())
            {
                somebody.push(format!(
                    "val.{} = su.{}.clone();",
                    col.column_names.clone().unwrap_or_default(),
                    col.column_names.clone().unwrap_or_default(),
                ));
                has_cond = true;
            }
        }
    }

    if ctx.codegen_conf.multi_tenancy {
        if has_cond {
            params.push(("su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));
        } else {
            params.push(("_su".to_string(), "SystemUser<ChimesUserInfo>".to_string()));
        }
    }

    params.push((
        "req".to_string(),
        format!("web::Json<{}>", tbl_param_name.clone()),
    ));

    if paging {
        params.push(("path_param".to_string(), format!("web::Path<(u64, u64)>")));
    }

    let mut body = vec![];

    body.push(format!("let rb = get_rbatis();"));
    if paging {
        body.push(format!("let (current, size) = path_param.into_inner();"));
    }


    if has_cond {
        body.push(format!("let mut val = req.to_owned();"));
        body.append(&mut somebody);
    } else {
        body.push(format!("let val = req.to_owned();"));
    }

    if paging {
        body.push(format!(
            "match {}::query_paged(rb, &val, current, size).await {{",
            tbl_name.clone()
        ));
    } else {
        body.push(format!(
            "match {}::query(rb, &val).await {{",
            tbl_name.clone()
        ));
    }

    body.push(format!("Ok(st) => {{"));
    if paging {
        body.push(format!(
            "let ret: web::Json<ApiResult<Page<{}>>> = web::Json(ApiResult::ok(st));",
            tbl_name.clone()
        ));
        body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    } else {
        if onerow {
            body.push(format!("match st {{"));
            body.push(format!("Some(vst) => {{"));
            body.push(format!(
                "let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::ok(vst));",
                tbl_name.clone()
            ));
            body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
            body.push(format!("}}"));
            body.push(format!("None => {{"));
            body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5404, &\"NOT-Found\".to_string()));", tbl_name.clone()));
            body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
            body.push(format!("}}"));
            body.push(format!("}}"));
        } else {
            body.push(format!(
                "let ret: web::Json<ApiResult<Vec<{}>>> = web::Json(ApiResult::ok(st));",
                tbl_name.clone()
            ));
            body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
        }
    }
    body.push("}".to_string());
    body.push("Err(err) => {".to_string());
    if paging {
        body.push(format!("let ret: web::Json<ApiResult<Page<{}>>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_name.clone()));
    } else {
        if onerow {
            body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_name.clone()));
        } else {
            body.push(format!("let ret: web::Json<ApiResult<Vec<{}>>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_name.clone()));
        }
    }
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push("}".to_string());
    body.push("}".to_string());

    let func_name = if paging {
        format!("{}_paged", snake_case(tbl.struct_name.clone().as_str()))
    } else {
        format!("{}_query", snake_case(tbl.struct_name.clone().as_str()))
    };

    let url_pattern = if paging {
        format!(
            "{}/{}/paged/{{current}}/{{size}}",
            ctx.codegen_conf.api_handler_prefix.clone(),
            tbl.api_handler_name.clone()
        )
    } else {
        format!(
            "{}/{}/query",
            ctx.codegen_conf.api_handler_prefix.clone(),
            tbl.api_handler_name.clone()
        )
    };

    let comment = if paging {
        format!("{}列表", tbl.comment.clone())
    } else {
        format!("{}分页", tbl.comment.clone())
    };

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
        api_method: Some("POST".to_string()),
        api_pattern: Some(url_pattern.clone()),
        comment: Some(comment.clone()),
    }
}

pub fn parse_query_handler_as_file(
    ctx: &mut GenerateContext,
    tbl: &QueryConfig,
    _cols: &TransformRow,
) -> RustFileImpl {
    let tbl_name = tbl.struct_name.clone();
    let tbl_param_name = format!("{}Params", tbl_name);

    let mut usinglist = CodeGenerator::get_default_handler_using(ctx, !tbl.single_result, false);
    usinglist.push(format!(
        "crate::query::{{{}, {}}}",
        tbl_name.clone(),
        tbl_param_name.clone()
    ));
    usinglist.push(format!("rbatis::Page"));

    let queryfunc = generate_handler_query_for_query(ctx, tbl, false, false);
    let pagefunc = generate_handler_query_for_query(ctx, tbl, true, false);
    let onerowfunc = generate_handler_query_for_query(ctx, tbl, false, true);
    let funclist = if tbl.single_result {
        vec![onerowfunc]
    } else {
        vec![pagefunc, queryfunc]
    };

    ctx.add_permission_for_query(&tbl, &funclist);

    RustFileImpl {
        file_name: snake_case(tbl.struct_name.clone().as_str()) + ".rs",
        mod_name: "handler".to_string(),
        caretlist: vec![],
        usinglist: usinglist,
        structlist: vec![],
        funclist: funclist,
    }
}
