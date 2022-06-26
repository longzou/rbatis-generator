use std::collections::HashMap;
use change_case::{pascal_case, snake_case};
use rbatis::rbatis::Rbatis;
use crate::codegen::{GenerateContext, RustFunc, parse_data_type_as_rust_type};
use crate::config::{TableConfig, get_rbatis};
use crate::schema::{TableInfo};


pub fn generate_actix_handler_for_table(ctx: &GenerateContext, tbl: &TableInfo, usinglist: &mut Vec<String>) -> Vec<RustFunc> {
  let mut funclist = vec![];
  let tbl_name = tbl.table_name.clone().unwrap_or_default();
  let tbl_struct_name = match ctx.get_struct_name(&tbl_name.clone()) {
    Some(t) => t,
    None => {
        pascal_case(tbl_name.clone().as_str())
    }
  };

  let tbc = ctx.get_table_conf(&tbl_name.clone()).unwrap();

  let save_handler = generate_handler_save_for_struct(ctx, tbl);
  funclist.push(save_handler);
  let update_handler = generate_handler_update_for_struct(ctx, tbl);
  funclist.push(update_handler);
  let delete_handler = generate_handler_delete_for_struct(ctx, tbl);
  funclist.push(delete_handler);
  let list_handler = generate_handler_query_list_for_struct(ctx, tbl);
  funclist.push(list_handler);
  if tbc.page_query {
    let page_handler = generate_handler_query_page_for_struct(ctx, tbl);
    funclist.push(page_handler);
    usinglist.push(format!("rbatis::Page"));
  }
  let get_handler = generate_handler_get_for_struct(ctx, tbl);
  funclist.push(get_handler);

  if tbc.tree_parent_field.is_some() {
    let tree_handler = generate_handler_query_tree_for_struct(ctx, tbl);
    funclist.push(tree_handler);
    usinglist.push(format!("actix_web::HttpRequest"));
    usinglist.push(format!("crate::entity::{{{}, {}Value}}", tbl_struct_name.clone(), tbl_struct_name.clone()).to_string());
  } else {
    usinglist.push(format!("crate::entity::{}", tbl_struct_name.clone()).to_string());
  }  

  funclist
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
    None => {
        pascal_case(tbl_name.clone().as_str())
    }
  };

  let mut params = Vec::new();
  // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
  params.push(("req".to_string(), format!("web::Json<{}>", tbl_struct_name.clone())));
 
  let mut body = vec![];
  
  body.push(format!("let rb = get_rbatis();"));
  body.push(format!("let val = req.to_owned();"));
  if tbc.update_seletive {
    body.push(format!("match val.update_selective(rb).await {{"));
  } else {
    body.push(format!("match val.update(rb).await {{"));
  }
  body.push(format!("Ok(_st) => {{"));
  body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::ok(val));", tbl_struct_name.clone()));
  body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
  body.push("}".to_string());
  body.push("Err(err) => {".to_string());
  body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_struct_name.clone()));
  body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
  body.push("}".to_string());
  body.push("}".to_string());
  let func_name = tbc.api_handler_name.clone() + "_update";
  let postmacro = format!("#[post(\"{}/{}/update\")]", ctx.codegen_conf.api_handler_prefix.clone(), tbc.api_handler_name.clone());
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
      macros: vec![postmacro]
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
    None => {
        pascal_case(tbl_name.clone().as_str())
    }
  };

  let mut params = Vec::new();
  // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
  params.push(("req".to_string(), format!("web::Json<{}>", tbl_struct_name.clone())));
 
  let mut body = vec![];
  
  body.push(format!("let rb = get_rbatis();"));
  body.push(format!("let mut val = req.to_owned();"));
  
  body.push(format!("match val.save(rb).await {{"));
  
  body.push(format!("Ok(_st) => {{"));
  body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::ok(val));", tbl_struct_name.clone()));
  body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
  body.push("}".to_string());
  body.push("Err(err) => {".to_string());
  body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_struct_name.clone()));
  body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
  body.push("}".to_string());
  body.push("}".to_string());
  let func_name = tbc.api_handler_name.clone() + "_save";

  let postmacro = format!("#[post(\"{}/{}/save\")]", ctx.codegen_conf.api_handler_prefix.clone(), tbc.api_handler_name.clone());
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
      macros: vec![postmacro]
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
    None => {
        pascal_case(tbl_name.clone().as_str())
    }
  };

  let mut params = Vec::new();
  // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
  params.push(("req".to_string(), format!("web::Json<{}>", tbl_struct_name.clone())));
 
  let mut body = vec![];
  
  body.push(format!("let rb = get_rbatis();"));
  body.push(format!("let mut val = req.to_owned();"));
  
  body.push(format!("match val.remove(rb).await {{"));
  
  body.push(format!("Ok(_st) => {{"));
  body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::ok(val));", tbl_struct_name.clone()));
  body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
  body.push("}".to_string());
  body.push("Err(err) => {".to_string());
  body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_struct_name.clone()));
  body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
  body.push("}".to_string());
  body.push("}".to_string());
  let func_name = tbc.api_handler_name.clone() + "_delete";

  let postmacro = format!("#[post(\"{}/{}/delete\")]", ctx.codegen_conf.api_handler_prefix.clone(), tbc.api_handler_name.clone());
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
      macros: vec![postmacro]
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
    None => {
        pascal_case(tbl_name.clone().as_str())
    }
  };

  let mut params = Vec::new();
  // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
  params.push(("req".to_string(), format!("web::Json<{}>", tbl_struct_name.clone())));
 
  let mut body = vec![];
  
  body.push(format!("let rb = get_rbatis();"));
  body.push(format!("let val = req.to_owned();"));
  
  body.push(format!("match val.query_list(rb).await {{"));
  
  body.push(format!("Ok(st) => {{"));
  if tbc.tree_parent_field.is_some() {
    body.push(format!("let mtts:Vec<{}Value> = st.into_iter().map(|f| {}Value::from_entity_with(&f, true, &vec![])).collect();", tbl_struct_name.clone(), tbl_struct_name.clone()));
    body.push(format!("let ret: web::Json<ApiResult<Vec<{}Value>>> = web::Json(ApiResult::ok(mtts));", tbl_struct_name.clone()));
  } else {
    body.push(format!("let ret: web::Json<ApiResult<Vec<{}>>> = web::Json(ApiResult::ok(st));", tbl_struct_name.clone()));
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

  let postmacro = format!("#[post(\"{}/{}/search\")]", ctx.codegen_conf.api_handler_prefix.clone(), tbc.api_handler_name.clone());
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
      macros: vec![postmacro]
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
    None => {
        pascal_case(tbl_name.clone().as_str())
    }
  };

  let mut params = Vec::new();
  // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
  params.push(("req".to_string(), format!("web::Json<{}>", tbl_struct_name.clone())));
  params.push(("path_param".to_string(), format!("web::Path<(u64, u64)>")));

  let mut body = vec![];
  
  body.push(format!("let rb = get_rbatis();"));
  body.push(format!("let val = req.to_owned();"));
  body.push(format!("let (current, size) = path_param.into_inner();"));
  
  body.push(format!("match val.query_paged(rb, current, size).await {{"));
  
  body.push(format!("Ok(st) => {{"));
  if tbc.tree_parent_field.is_some() {
    body.push(format!("let mtts:Vec<{}Value> = st.records.into_iter().map(|f| {}Value::from_entity_with(&f, true, &vec![])).collect();", tbl_struct_name.clone(), tbl_struct_name.clone()));
    body.push(format!("let mut newpage = Page::new_total(st.page_no, st.page_size, st.total);"));
    body.push(format!("newpage.records = mtts;"));
    body.push(format!("let ret: web::Json<ApiResult<Page<{}Value>>> = web::Json(ApiResult::ok(newpage));", tbl_struct_name.clone()));
  } else {
    body.push(format!("let ret: web::Json<ApiResult<Page<{}>>> = web::Json(ApiResult::ok(st));", tbl_struct_name.clone()));
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

  let postmacro = format!("#[post(\"{}/{}/paged/{{current}}/{{size}}\")]", ctx.codegen_conf.api_handler_prefix.clone(), tbc.api_handler_name.clone());
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
      macros: vec![postmacro]
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
    None => {
        pascal_case(tbl_name.clone().as_str())
    }
  };

  let treecol = ctx.find_table_column(&tbl_name.clone(), &tbc.tree_parent_field.unwrap()).unwrap();
  let treecol_type = parse_data_type_as_rust_type(&treecol.data_type.unwrap_or_default());

  let mut params = Vec::new();

  

  // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
  params.push(("req".to_string(), format!("HttpRequest")));
 
  let mut body = vec![];
  
  body.push(format!("let rb = get_rbatis();"));

  body.push(format!("let query = req.query_string();"));
  body.push(format!("let dic = crate::utils::parse_query(query);"));
  body.push(format!("let val = crate::utils::get_hash_value(&dic, \"pid\");"));

  if treecol_type == "String" {
    body.push(format!("let valopt = if val.is_empty() {{"));
    body.push(format!("None"));
    body.push(format!("}} else {{"));
    body.push(format!("Some(val)"));
    body.push(format!("}};"));
  } else { // actuall the should be i64, u64 etc some number type
    body.push(format!("let valopt = match val.parse::<{}>() {{", treecol_type));
    body.push(format!("Ok(tv) => Some(tv),"));
    body.push(format!("Err(_) => None"));
    body.push(format!("}};"));
  }

  body.push(format!("match {}::query_tree(rb, &valopt).await {{", tbl_struct_name.clone()));
  body.push(format!("Ok(st) => {{"));
  body.push(format!("let mtts:Vec<{}Value> = st.into_iter().map(|f| {}Value::from_entity_with(&f, true, &vec![])).collect();", tbl_struct_name.clone(), tbl_struct_name.clone()));
  body.push(format!("let ret: web::Json<ApiResult<Vec<{}Value>>> = web::Json(ApiResult::ok(mtts));", tbl_struct_name.clone()));
  body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
  body.push("}".to_string());
  body.push("Err(err) => {".to_string());
  body.push(format!("let ret: web::Json<ApiResult<Vec<{}Value>>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_struct_name.clone()));
  body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
  body.push("}".to_string());
  body.push("}".to_string());
  let func_name = tbc.api_handler_name.clone() + "_tree";

  let postmacro = format!("#[get(\"{}/{}/tree\")]", ctx.codegen_conf.api_handler_prefix.clone(), tbc.api_handler_name.clone());
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
      macros: vec![postmacro]
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
    None => {
        pascal_case(tbl_name.clone().as_str())
    }
  };

  let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
  if pkcols.is_empty() {
      pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
  }

  let mut param_text = String::new();
  let mut params = Vec::new();
  // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
  // params.push(("req".to_string(), format!("web::Path<{}>", tbl_struct_name.clone())));
  for col in pkcols.clone() {
      let dt = parse_data_type_as_rust_type(&col.data_type.unwrap_or_default());
      let colname = col.column_name.unwrap_or_default().to_lowercase();
      params.push((format!("{}_req", colname.clone()), format!("web::Path<{}>", dt)));
      param_text.push_str(format!(", &{}", colname.as_str()).as_str());
  }
  // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
  
  let mut body = vec![];
  
  body.push(format!("let rb = get_rbatis();"));
  for col in pkcols.clone() {
    let colname = col.column_name.unwrap_or_default().to_lowercase();
    body.push(format!("let {} = {}_req.to_owned();", colname.clone(), colname.clone()));
  }
  
  body.push(format!("match {}::from_id(rb{}).await {{", tbl_struct_name.clone(), param_text.clone()));
  
  body.push(format!("Ok(st) => {{"));
  body.push(format!("match st {{"));
  body.push(format!("Some(tv) => {{"));
  body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::ok(tv));", tbl_struct_name.clone()));
  body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
  body.push("}".to_string());
  body.push(format!("None => {{"));
  body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5040, &\"Not-Found\".to_string()));", tbl_struct_name.clone()));
  body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
  body.push("}".to_string());
  body.push("}".to_string());
  body.push("}".to_string());
  body.push("Err(err) => {".to_string());
  body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl_struct_name.clone()));
  body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
  body.push("}".to_string());
  body.push("}".to_string());
  let func_name = tbc.api_handler_name.clone() + "_get";

  let postmacro = format!("#[get(\"{}/{}/get/{{id}}\")]", ctx.codegen_conf.api_handler_prefix.clone(), tbc.api_handler_name.clone());
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
      macros: vec![postmacro]
  }
}
