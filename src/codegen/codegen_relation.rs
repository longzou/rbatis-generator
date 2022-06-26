use change_case::{pascal_case, snake_case};
use crate::codegen::{RustStructField, GenerateContext, RustStruct, RustFunc, parse_data_type_as_rust_type, parse_column_list, make_skip_columns};
use crate::config::{TableConfig, get_rbatis, safe_struct_field_name, RelationConfig};
use crate::schema::{TableInfo, ColumnInfo};
use substring::Substring;

use super::{RustFileImpl, CodeGenerator};

/**
 * 解析关系并生成文件
 */
pub fn parse_relation_as_file(ctx: &GenerateContext, rel: &RelationConfig) -> Option<RustFileImpl> {
    let st = parse_relation_as_struct(ctx, rel);
    let tbc = ctx.get_table_conf(&rel.major_table.clone());
    match tbc {
        Some(tbconf) => {
            let mut usinglist = CodeGenerator::get_default_entity_using(tbconf.page_query);
        
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
                funclist: vec![]
            };
            Some(rfi)
        }
        None => {
            None
        }
    }
}
  

/**
 * 解析关系并生成文件
 */
pub fn parse_relation_handlers_as_file(ctx: &GenerateContext, rel: &RelationConfig) -> Option<RustFileImpl> {
    let st = parse_relation_as_struct(ctx, rel);
    let tbc = ctx.get_table_conf(&rel.major_table.clone());
    
    match tbc {
        Some(tbconf) => {
            let mut usinglist = CodeGenerator::get_default_handler_using(tbconf.page_query);
            usinglist.push(format!("crate::entity::{}", st.struct_name));
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

            let mut funclist = vec![];
            if rel.generate_select {
                let funcsel = generate_handler_load_for_relation(ctx, rel);
                funclist.push(funcsel);
            }

            if rel.generate_delete {
                let funcdel = generate_handler_remove_for_relation(ctx, rel);
                funclist.push(funcdel);
            }

            if rel.generate_save {
                let funcsave = generate_handler_save_for_relation(ctx, rel);
                funclist.push(funcsave);
            }            

            let rfi = RustFileImpl { 
                file_name: snake_case(rel.struct_name.clone().as_str()) + ".rs",
                mod_name: "handler".to_string(), 
                caretlist: vec![],
                usinglist: usinglist, 
                structlist: vec![],
                funclist: funclist
            };
            Some(rfi)
        }
        None => {
            None
        }
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

    let mut fields = if rel.extend_major {
        let parsed_fields = parse_column_list(ctx, &tbconf, &cols, &mut columns);
        if columns.ends_with(",") {
            columns = columns.substring(0, columns.len() - 1).to_string();
        }
        parsed_fields
    } else {
        let fname = tbconf.api_handler_name.clone();
        vec![RustStructField { 
            is_pub: true, 
            column_name: String::new(), 
            field_name: safe_struct_field_name(&fname), 
            field_type: tbconf.struct_name.clone(), 
            is_option: true
        }]
    };

    for rl in rel.one_to_one.clone() {
        let rltbc = ctx.get_table_conf(&rl.table_name.unwrap_or_default());
        if rltbc.is_some() {
            let rltcnf = rltbc.unwrap();
            let rlfd = RustStructField { 
                is_pub: true, 
                column_name: rl.join_field.unwrap_or_default(), 
                field_name: safe_struct_field_name(&rltcnf.api_handler_name),
                field_type: rltcnf.struct_name.clone(), 
                is_option: true
            };
            fields.push(rlfd);
        }
    }

    for rl in rel.one_to_many.clone() {
        let rltbc = ctx.get_table_conf(&rl.table_name.unwrap_or_default());
        if rltbc.is_some() {
            let rltcnf = rltbc.unwrap();
            let rlfd = RustStructField {
                is_pub: true, 
                column_name: rl.join_field.unwrap_or_default(), 
                field_name: format!("{}s", safe_struct_field_name(&rltcnf.api_handler_name)), 
                field_type: format!("Vec<{}>", rltcnf.struct_name.clone()), 
                is_option: false
            };
            fields.push(rlfd);
        }
    }

    
    // let crudtbl = format!("#[crud_table(table_name:\"{}\"|table_columns:\"{}\")]", tbl_name.clone(), columns);
    let anno = vec!["#[derive(Debug, Clone, Default, Deserialize, Serialize)]".to_string()];

    let mut funclist = vec![];

    let from_func = generate_func_from_major_table(ctx, rel);
    let to_func = generate_func_to_major_table(ctx, rel);

    funclist.push(from_func);
    funclist.push(to_func);

    if rel.generate_select {
        let from_id = generate_func_from_pkey_for_relation(ctx, rel);
        funclist.push(from_id);
    }

    if rel.generate_save {
        let update_func = generate_func_update_for_relation(ctx, rel);
        funclist.push(update_func);
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
    }
}

fn generate_func_from_major_table(ctx: &GenerateContext, rel: &RelationConfig) -> RustFunc {
    let tbl_name = rel.major_table.clone();
    let tbc = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tbc.unwrap();
    let mut body = vec![];

    body.push(format!("{} {{", rel.struct_name.clone()));
    if rel.extend_major {
        let mut columns = String::new();
        let cols = ctx.get_table_columns(&tbl_name.clone());
        let parsed_fields = parse_column_list(ctx, &tbconf, &cols, &mut columns);
        for fd in parsed_fields {
            let fname = fd.field_name.clone();
            body.push(format!("{}: param.{}.clone(),", safe_struct_field_name(&fname), safe_struct_field_name(&fname)));
        }
    } else {
        let fname = tbconf.api_handler_name.clone();
        body.push(format!("{}: Some(param.clone()),", safe_struct_field_name(&fname)));
    }


    for rl in rel.one_to_one.clone() {
        let rltbc = ctx.get_table_conf(&rl.table_name.unwrap_or_default());
        if rltbc.is_some() {
            let rltcnf = rltbc.unwrap();
            body.push(format!("{}: None,", safe_struct_field_name(&rltcnf.api_handler_name)));
        }
    }

    for rl in rel.one_to_many.clone() {
        let rltbc = ctx.get_table_conf(&rl.table_name.unwrap_or_default());
        if rltbc.is_some() {
            let rltcnf = rltbc.unwrap();
            body.push(format!("{}: vec![],", format!("{}s", safe_struct_field_name(&rltcnf.api_handler_name))));
        }
    }

    body.push(format!("}}"));

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("param".to_string(), "&".to_owned() + tbconf.struct_name.clone().as_str()));

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
        macros: vec!["#[allow(dead_code)]".to_string()]
    }
}

fn generate_func_to_major_table(ctx: &GenerateContext, rel: &RelationConfig) -> RustFunc {
    let tbl_name = rel.major_table.clone();
    let tbc = ctx.get_table_conf(&tbl_name.clone());
    let tbconf = tbc.unwrap();
    let mut body = vec![];

    
    if rel.extend_major {
        let mut columns = String::new();
        let cols = ctx.get_table_columns(&tbl_name.clone());
        let parsed_fields = parse_column_list(ctx, &tbconf, &cols, &mut columns);
        body.push(format!("{} {{", tbconf.struct_name.clone()));
        for fd in parsed_fields {
            let fname = fd.field_name.clone();
            body.push(format!("{}: self.{}.clone(),", safe_struct_field_name(&fname), safe_struct_field_name(&fname)));
        }
        body.push(format!("}}"));
    } else {
        let fname = tbconf.api_handler_name.clone();
        body.push(format!("match self.{}.clone() {{", safe_struct_field_name(&fname)));
        body.push(format!("Some(st) => st,"));
        body.push(format!("None => {}::default()", tbconf.struct_name.clone()));
        body.push(format!("}}"));
    }

    let mut params = Vec::new();

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
        macros: vec!["#[allow(dead_code)]".to_string()]
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
        params.push((col.column_name.clone().unwrap_or_default().to_lowercase(), "&".to_string() + dt.as_str()));
        params_text.push_str(col.column_name.clone().unwrap_or_default().to_lowercase().as_str());
        params_text.push_str(",");
    }

    if params_text.ends_with(",") {
        params_text = params_text.substring(0, params_text.len() - 1).to_string();
    }

    let tbc = tblinfo.unwrap();

    let mut body = vec![];
    body.push(format!("match {}::from_id(rb, {}).await {{", tbc.struct_name.clone(), params_text));
    body.push(format!("Ok(ts) => {{"));
    body.push(format!("match ts {{"));
    body.push(format!("Some(mp) => {{"));
    body.push(format!("let mut selfmp = Self::from_{}(&mp);", tbc.api_handler_name.clone()));
    // Above is right
    for otp in tbl.one_to_one.clone() {
        let tpconf = ctx.get_table_conf(&otp.table_name.clone().unwrap_or_default());
        if tpconf.is_some() {
            let tpc = tpconf.unwrap();
            
            let mut optpkcols = ctx.get_table_column_by_primary_key(&otp.table_name.clone().unwrap_or_default());
            if optpkcols.is_empty() {
                optpkcols.append(&mut ctx.get_table_pkey_column(&&otp.table_name.clone().unwrap_or_default()));
            }
            
            // 关系型的表，目前代码生成中，支持一个主键，没有主键也不行
            let optpkcol = optpkcols.get(0).unwrap();
            let optpkcolname = safe_struct_field_name(&optpkcol.column_name.clone().unwrap_or_default().to_lowercase());

            body.push(format!("let mut tmp_{} = {}::default();", tpc.api_handler_name.clone(), tpc.struct_name.clone()));
            if tbl.extend_major {
                body.push(format!("tmp_{}.{} = selfmp.{}.clone();", tpc.api_handler_name.clone(), otp.join_field.clone().unwrap_or_default().to_lowercase(), otp.major_field.clone().unwrap_or_default().to_lowercase()));
            } else {
                body.push(format!("tmp_{}.{} = selfmp.{}.clone().unwrap().{}.clone();", tpc.api_handler_name.clone(), 
                                    otp.join_field.clone().unwrap_or_default().to_lowercase(), 
                                    tbc.api_handler_name.clone(),
                                    otp.major_field.clone().unwrap_or_default().to_lowercase()));
            }
            
            body.push(format!("selfmp.{} = match tmp_{}.query_list(rb).await {{", tpc.api_handler_name.clone(), tpc.api_handler_name.clone()));
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
            
            let mut optpkcols = ctx.get_table_column_by_primary_key(&otp.table_name.clone().unwrap_or_default());
            if optpkcols.is_empty() {
                optpkcols.append(&mut ctx.get_table_pkey_column(&&otp.table_name.clone().unwrap_or_default()));
            }

            let many_many = otp.middle_table.is_some();
            
            // 关系型的表，目前代码生成中，支持一个主键，没有主键也不行
            let optpkcol = optpkcols.get(0).unwrap();
            let optpkcolname = safe_struct_field_name(&optpkcol.column_name.clone().unwrap_or_default().to_lowercase());
            if many_many {
                let joinfd = otp.join_field.clone().unwrap_or_default();
                let majorfd = otp.major_field.clone().unwrap_or_default();

                let sql = format!("SELECT tp.* FROM {} tp INNER JOIN {} mt ON tp.{} = mt.{} WHERE mt.{} = ?", otp.table_name.clone().unwrap_or_default(), otp.middle_table.clone().unwrap_or_default(), joinfd.clone(), joinfd.clone(), majorfd.clone());
                body.push(format!("let mut rb_args = vec![];"));
                body.push(format!("let sql_{} = \"{}\";", tpc.api_handler_name.clone(), sql));
                if tbl.extend_major {
                    body.push(format!("rb_args.push(rbson::to_bson(&selfmp.{}.clone().unwrap_or_default()).unwrap_or_default());", majorfd.clone().to_lowercase()));
                } else {
                    body.push(format!("rb_args.push(rbson::to_bson(&selfmp.{}.clone().unwrap().{}.clone().unwrap_or_default()).unwrap_or_default());", 
                                                tbc.api_handler_name.clone(), majorfd.clone().to_lowercase()));
                }

                body.push(format!("selfmp.{}s = match rb.fetch(sql_{}, rb_args).await {{", tpc.api_handler_name.clone(), tpc.api_handler_name.clone()));
                body.push(format!("Ok(lst) => {{"));
                body.push(format!("lst"));
                body.push(format!("}}"));
                body.push(format!("Err(_) => {{"));
                body.push(format!("vec![]"));
                body.push(format!("}}"));                
                body.push(format!("}};"));
            } else {
                body.push(format!("let mut tmp_{} = {}::default();", tpc.api_handler_name.clone(), tpc.struct_name.clone()));
                body.push(format!("tmp_{}.{} = selfmp.{}.clone();", tpc.api_handler_name.clone(), otp.join_field.clone().unwrap_or_default().to_lowercase(), otp.major_field.clone().unwrap_or_default().to_lowercase()));
                body.push(format!("selfmp.{}s = match tmp_{}.query_list(rb).await {{", tpc.api_handler_name.clone(), tpc.api_handler_name.clone()));
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
        macros: vec!["#[allow(dead_code)]".to_string()]
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
    let pkcolname = safe_struct_field_name(&pkcol.column_name.clone().unwrap_or_default().to_lowercase());

    let mut params = Vec::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    params.push(("rb".to_string(), "&Rbatis".to_string()));

    let mut body = vec![];
    
    body.push(format!("let mut ret: Option<Error> = None;"));

    // Save the major table first
    body.push(format!("let mut self_{} = self.to_{}();", tbconf.api_handler_name.clone(), tbconf.api_handler_name.clone()));
    body.push(format!("if self_{}.{}.is_none() {{", tbconf.api_handler_name.clone(), pkcolname.clone()));
    body.push(format!("ret = match self_{}.save(rb).await {{", tbconf.api_handler_name.clone()));
    body.push(format!("Ok(_rs) => {{")); //  begin of Ok
    body.push(format!("None"));
    body.push(format!("}}")); //  end of Ok
    body.push(format!("Err(err) => {{")); //  begin of none
    body.push(format!("log::info!(\"Save {} occurred an error {{}}\", err);", tbconf.api_handler_name.clone()));
    body.push(format!("Some(err)"));
    body.push(format!("}}")); //  end of error
    body.push(format!("}}")); // end of if
    body.push(format!("}}")); // end of if
    body.push(format!("else {{"));
    body.push(format!("ret = match self_{}.update(rb).await {{", tbconf.api_handler_name.clone()));
    body.push(format!("Ok(_rs) => {{")); //  begin of Ok
    body.push(format!("None"));
    body.push(format!("}}")); //  end of Ok
    body.push(format!("Err(err) => {{")); //  begin of none
    body.push(format!("log::info!(\"Update {} occurred an error {{}}\", err);", tbconf.api_handler_name.clone()));
    body.push(format!("Some(err)"));
    body.push(format!("}}")); //  end of error
    body.push(format!("}}")); // end of if    
    body.push(format!("}}"));

    for otp in tbl.one_to_one.clone() {
        let tpconf = ctx.get_table_conf(&otp.table_name.clone().unwrap_or_default());
        if tpconf.is_some() {
            let tpc = tpconf.unwrap();
            
            let mut optpkcols = ctx.get_table_column_by_primary_key(&otp.table_name.clone().unwrap_or_default());
            if optpkcols.is_empty() {
                optpkcols.append(&mut ctx.get_table_pkey_column(&&otp.table_name.clone().unwrap_or_default()));
            }
            
            // 关系型的表，目前代码生成中，支持一个主键，没有主键也不行
            let optpkcol = optpkcols.get(0).unwrap();
            let optpkcolname = safe_struct_field_name(&optpkcol.column_name.clone().unwrap_or_default().to_lowercase());

            
            body.push(format!("if ret.is_none() {{"));
            body.push(format!("ret = match self.{}.clone() {{", tpc.api_handler_name.clone()));
            body.push(format!("Some(tp) => {{"));  // begin of Some
            body.push(format!("let mut mtp = tp.clone();"));
            body.push(format!("mtp.{} = self_{}.{}.clone();", safe_struct_field_name(&otp.join_field.clone().unwrap_or_default().to_lowercase()), tbconf.api_handler_name.clone(), pkcolname.clone()));
            body.push(format!("if mtp.{}.is_none() {{", optpkcolname.clone()));
            body.push(format!("match mtp.save(rb).await {{"));
            body.push(format!("Ok(_mtpsave) => {{")); //  begin of Ok
            body.push(format!("None"));
            body.push(format!("}}")); //  end of Ok
            body.push(format!("Err(err) => {{")); //  begin of none
            body.push(format!("log::info!(\"Save {} occurred an error {{}}\", err);", tpc.api_handler_name.clone()));
            body.push(format!("Some(err)"));
            body.push(format!("}}")); //  end of error
            body.push(format!("}}")); //  end of mtpsave

            body.push(format!("}} else {{")); //  end of Some(tp)

            body.push(format!("match mtp.update(rb).await {{"));
            body.push(format!("Ok(_mtpsave) => {{")); //  begin of Ok
            body.push(format!("None"));
            body.push(format!("}}")); //  end of Ok
            body.push(format!("Err(err) => {{")); //  begin of none
            body.push(format!("log::info!(\"Save {} occurred an error {{}}\", err);", tpc.api_handler_name.clone()));
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

        let targtbl = ctx.get_table_conf(&otp.table_name.clone().unwrap_or_default());

        let many_many = if otp.middle_table.clone().is_none() {
            false
        } else {
            true
        };

        let targtblconf = targtbl.unwrap();

        let mut optpkcols = ctx.get_table_column_by_primary_key(&otp.table_name.clone().unwrap_or_default());
        if optpkcols.is_empty() {
            optpkcols.append(&mut ctx.get_table_pkey_column(&&otp.table_name.clone().unwrap_or_default()));
        }
        
        // 关系型的表，目前代码生成中，支持一个主键，没有主键也不行
        let optpkcol = optpkcols.get(0).unwrap();
        let optpkcolname = safe_struct_field_name(&optpkcol.column_name.clone().unwrap_or_default().to_lowercase());

        if tpconf.is_some() {
            let tpc = tpconf.unwrap();
            body.push(format!("// remove batch for {}.", tpc.struct_name.clone()));
            
            body.push(format!("if ret.is_none() {{"));
            if many_many {
                body.push(format!("let mut rm_{} = {}::default();", tpc.api_handler_name.clone(), tpc.struct_name.clone()));
                if tbl.extend_major {
                    body.push(format!("rm_{}.{} = self.{};", tpc.api_handler_name.clone(), otp.join_field.clone().unwrap_or_default(), otp.major_field.clone().unwrap_or_default()));
                } else {
                    body.push(format!("rm_{}.{} = self.{}.clone().unwrap().{};", tpc.api_handler_name.clone(), 
                                                                otp.major_field.clone().unwrap_or_default(), 
                                                                tbconf.api_handler_name.clone(),
                                                                otp.major_field.clone().unwrap_or_default()));
                }
            //}
                body.push(format!("ret = match rm_{}.remove_batch(rb).await {{", tpc.api_handler_name.clone()));
                body.push(format!("Ok(_) => {{")); //  begin of Ok
                body.push(format!("None"));
                body.push(format!("}}")); //  end of Ok
                body.push(format!("Err(err) => {{")); //  begin of none
                body.push(format!("log::info!(\"Remove {} occurred an error {{}}\", err);", tpc.api_handler_name.clone()));
                body.push(format!("Some(err)"));
                body.push(format!("}}")); //  end of error
                body.push(format!("}};")); // end of rm_{}
                body.push(format!("}}")); // end of if

                body.push(format!("for row in self.{}s.clone() {{", targtblconf.api_handler_name));
                body.push(format!("let mut svrow_{} = {}::default();", tpc.api_handler_name.clone(), tpc.struct_name.clone()));
                if tbl.extend_major {
                    body.push(format!("svrow_{}.{} = self.{}.clone();", tpc.api_handler_name.clone(), otp.major_field.clone().unwrap_or_default(), pkcolname.clone()));
                } else {
                    body.push(format!("svrow_{}.{} = self.{}.clone().unwrap().{}.clone();", tpc.api_handler_name.clone(), otp.major_field.clone().unwrap_or_default(), 
                                        tbconf.api_handler_name.clone(), pkcolname.clone()));
                }
                
                body.push(format!("svrow_{}.{} = row.{}.clone();", tpc.api_handler_name.clone(), otp.join_field.clone().unwrap_or_default(), otp.join_field.clone().unwrap_or_default()));

                body.push(format!("ret = match svrow_{}.save(rb).await {{", tpc.api_handler_name.clone()));
                body.push(format!("Ok(_) => {{")); //  begin of Ok
                body.push(format!("None"));
                body.push(format!("}}")); //  end of Ok
                body.push(format!("Err(err) => {{")); //  begin of none
                body.push(format!("log::info!(\"Save {} occurred an error {{}}\", err);", tpc.api_handler_name.clone()));
                body.push(format!("Some(err)"));
                body.push(format!("}}")); //  end of error
                body.push(format!("}};")); // end of rm_{}
                body.push(format!("}}")); // end of for
            } else {
            // if otp.middle_table.clone().is_some() {
                body.push(format!("for row in self.{}s.clone() {{", targtblconf.api_handler_name));
                body.push(format!("let mut sv_{} = row.clone();", tpc.api_handler_name.clone()));
                body.push(format!("rm_{}.{} = self.{}.clone();", tpc.api_handler_name.clone(), otp.join_field.unwrap_or_default(), pkcolname.clone()));
             
                body.push(format!("if rm_{}.{}.is_none() {{", tpc.api_handler_name.clone(), optpkcolname.clone()));
                body.push(format!("ret = match rm_{}.save(rb).await {{", tpc.api_handler_name.clone()));
                body.push(format!("Ok(_) => {{")); //  begin of Ok
                body.push(format!("None"));
                body.push(format!("}}")); //  end of Ok
                body.push(format!("Err(err) => {{")); //  begin of none
                body.push(format!("log::info!(\"Remove {} occurred an error {{}}\", err);", tpc.api_handler_name.clone()));
                body.push(format!("Some(err)"));
                body.push(format!("}}")); //  end of error
                body.push(format!("}};")); // end of rm_{}
                body.push(format!("}}")); // end of if
                body.push(format!("}} else {{"));
                body.push(format!("ret = match rm_{}.update(rb).await {{", tpc.api_handler_name.clone()));
                body.push(format!("Ok(_) => {{")); //  begin of Ok
                body.push(format!("None"));
                body.push(format!("}}")); //  end of Ok
                body.push(format!("Err(err) => {{")); //  begin of none
                body.push(format!("log::info!(\"Remove {} occurred an error {{}}\", err);", tpc.api_handler_name.clone()));
                body.push(format!("Some(err)"));
                body.push(format!("}}")); //  end of error
                body.push(format!("}};")); // end of rm_{}
                body.push(format!("}}")); // end of if
                body.push(format!("}}")); // end of for
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
        macros: vec!["#[allow(dead_code)]".to_string()]
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
    params.push(("rb".to_string(), "&Rbatis".to_string()));

    let mut body = vec![];
    
    body.push(format!("let mut ret: Option<Error> = None;"));
    for otp in tbl.one_to_one.clone() {
        let tpconf = ctx.get_table_conf(&otp.table_name.unwrap_or_default());
        if tpconf.is_some() {
            let tpc = tpconf.unwrap();
            body.push(format!("if ret.is_none() {{"));
            body.push(format!("ret = match self.{}.clone() {{", tpc.api_handler_name.clone()));
            body.push(format!("Some(tp) => {{"));  // begin of Some
            body.push(format!("let mut mtp = tp.clone();"));
            body.push(format!("match mtp.remove(rb).await {{"));
            body.push(format!("Ok(_rtremove) => {{")); //  begin of Ok
            body.push(format!("None"));
            body.push(format!("}}")); //  end of Ok
            body.push(format!("Err(err) => {{")); //  begin of none
            body.push(format!("log::info!(\"Remove {} occurred an error {{}}\", err);", tpc.api_handler_name.clone()));
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

        if tpconf.is_some() {
            let mtpc = majtblconf.unwrap();
            let tpc = tpconf.unwrap();
            body.push(format!("// remove batch for {}.", tpc.struct_name.clone()));
            
            body.push(format!("if ret.is_none() {{"));
            // if otp.middle_table.clone().is_some() {
            if tbl.extend_major {
                body.push(format!("let mut rm_{} = {}::default();", tpc.api_handler_name.clone(), tpc.struct_name.clone()));
                body.push(format!("rm_{}.{} = self.{};", tpc.api_handler_name.clone(), otp.join_field.unwrap_or_default().to_lowercase(), otp.major_field.unwrap_or_default().to_lowercase()));
            } else {
                body.push(format!("let mut rm_{} = {}::default();", tpc.api_handler_name.clone(), tpc.struct_name.clone()));
                body.push(format!("rm_{}.{} = self.{}.clone().unwrap().{};", tpc.api_handler_name.clone(), 
                                                    otp.join_field.unwrap_or_default().to_lowercase(), 
                                                    tbconf.api_handler_name.clone(), 
                                                    otp.major_field.unwrap_or_default().to_lowercase()));
            }
            //}
            body.push(format!("ret = match rm_{}.remove_batch(rb).await {{", tpc.api_handler_name.clone()));
            body.push(format!("Ok(_rtremove) => {{")); //  begin of Ok
            body.push(format!("None"));
            body.push(format!("}}")); //  end of Ok
            body.push(format!("Err(err) => {{")); //  begin of none
            body.push(format!("log::info!(\"Remove {} occurred an error {{}}\", err);", tpc.api_handler_name.clone()));
            body.push(format!("Some(err)"));
            body.push(format!("}}")); //  end of error
            body.push(format!("}};")); // end of rm_{}
            body.push(format!("}}")); // end of if
        }
    }

    body.push(format!("if ret.is_none() {{"));
    body.push(format!("match self.to_{}().remove(rb).await {{", tbconf.api_handler_name.clone()));
    body.push(format!("Ok(_rs) => {{")); //  begin of Ok
    body.push(format!("Ok(true)"));
    body.push(format!("}}")); //  end of Ok
    body.push(format!("Err(err) => {{")); //  begin of none
    body.push(format!("log::info!(\"Remove {} occurred an error {{}}\", err);", tbconf.api_handler_name.clone()));
    body.push(format!("Err(err)"));
    body.push(format!("}}")); //  end of error
    body.push(format!("}}")); // end of if
    body.push(format!("}}")); // end of if
    body.push(format!("else {{"));
    body.push(format!("Err(ret.unwrap())"));
    body.push(format!("}}"));
    

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
        macros: vec!["#[allow(dead_code)]".to_string()]
    }
}



/**
 * 生成Relation加载load 的Handler
 */
pub fn generate_handler_load_for_relation(ctx: &GenerateContext, tbl: &RelationConfig) -> RustFunc {
    let tbl_name = tbl.major_table.clone();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    let mut params_text = String::new();
    let mut macrotext = String::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    for col in pkcols.clone() {
        let dt = parse_data_type_as_rust_type(&col.data_type.unwrap_or_default());
        params.push((col.column_name.clone().unwrap_or_default().to_lowercase(), format!("web::Path<{}>", dt.clone())));
        params_text.push_str(format!("&{}", col.column_name.clone().unwrap_or_default().to_lowercase()).as_str());
        params_text.push_str(",");
        macrotext.push_str("/");
        macrotext.push_str(format!("{{{}}}", col.column_name.clone().unwrap_or_default().to_lowercase()).as_str());
    }

    if params_text.ends_with(",") {
        params_text = params_text.substring(0, params_text.len() - 1).to_string();
    }

    let tbc = tblinfo.unwrap();
  
    let mut body = vec![];
    
    body.push(format!("let rb = get_rbatis();"));
    body.push(format!("match {}::load(rb, {}).await {{", tbl.struct_name.clone(), params_text));
    body.push(format!("Ok(st) => {{"));
    body.push(format!("let ret: web::Json<ApiResult<Option<{}>>> = web::Json(ApiResult::ok(st));", tbl.struct_name.clone()));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!("let ret: web::Json<ApiResult<Option<{}>>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl.struct_name.clone()));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    
    let postmacro = format!("#[get(\"{}/{}/load{}\")]", ctx.codegen_conf.api_handler_prefix.clone(), tbl.api_handler_name.clone().unwrap_or_default(), macrotext);

    let func_name = format!("{}_rel_load", tbl.api_handler_name.clone().unwrap_or_default());
  
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
 * 生成Relation Delete 的Handler
 */
pub fn generate_handler_remove_for_relation(ctx: &GenerateContext, tbl: &RelationConfig) -> RustFunc {
    let tbl_name = tbl.major_table.clone();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    let mut params_text = String::new();
    let mut macrotext = String::new();
    // let pk = ctx.get_table_column_by_name(tbl.table_name, tbl);
    for col in pkcols.clone() {
        let dt = parse_data_type_as_rust_type(&col.data_type.unwrap_or_default());
        params.push((col.column_name.clone().unwrap_or_default().to_lowercase(), format!("web::Path<{}>", dt.clone())));
        params_text.push_str(format!("&{}", col.column_name.clone().unwrap_or_default().to_lowercase()).as_str());
        params_text.push_str(",");
        macrotext.push_str("/");
        macrotext.push_str(format!("{{{}}}", col.column_name.clone().unwrap_or_default().to_lowercase()).as_str());
    }

    if params_text.ends_with(",") {
        params_text = params_text.substring(0, params_text.len() - 1).to_string();
    }

    let tbc = tblinfo.unwrap();
  
    let mut body = vec![];
    
    body.push(format!("let rb = get_rbatis();"));
    body.push(format!("match {}::load(rb, {}).await {{", tbl.struct_name.clone(), params_text));
    body.push(format!("Ok(st) => {{"));
    body.push(format!("match st {{"));
    body.push(format!("Some(cst) => {{"));
    body.push(format!("match cst.remove(rb).await {{"));
    body.push(format!("Ok(_) => {{"));
    body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::ok(cst));", tbl.struct_name.clone()));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5011, &err.to_string()));", tbl.struct_name.clone()));
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
    body.push(format!("let ret: web::Json<ApiResult<{}>> = web::Json(ApiResult::error(5010, &err.to_string()));", tbl.struct_name.clone()));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    
    
    let postmacro = format!("#[post(\"{}/{}/remove{}\")]", ctx.codegen_conf.api_handler_prefix.clone(), tbl.api_handler_name.clone().unwrap_or_default(), macrotext);

    let func_name = format!("{}_rel_remove", tbl.api_handler_name.clone().unwrap_or_default());
  
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
 * 生成Relation保存的Handler
 */
pub fn generate_handler_save_for_relation(ctx: &GenerateContext, tbl: &RelationConfig) -> RustFunc {
    let tbl_name = tbl.major_table.clone();
    let tblinfo = ctx.get_table_conf(&tbl_name.clone());
    // let pkcol = ctx.get_table_column_by_name(&tbl.table_name.unwrap_or_default(), &tbl.);
    let mut pkcols = ctx.get_table_column_by_primary_key(&tbl_name.clone());
    if pkcols.is_empty() {
        pkcols.append(&mut ctx.get_table_pkey_column(&tbl_name.clone()));
    }

    let mut params = Vec::new();
    params.push(("req".to_string(), format!("web::Json<{}>", tbl.struct_name.clone())));

    
    let tbc = tblinfo.unwrap();
  
    let mut body = vec![];
    
    body.push(format!("let rb = get_rbatis();"));
    body.push(format!("let mut val = req.to_owned();"));
    body.push(format!("match val.save(rb).await {{"));
    body.push(format!("Ok(_st) => {{"));
    body.push(format!("let ret: web::Json<ApiResult<String>> = web::Json(ApiResult::ok(\"SUCCESS\".to_string()));"));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!("let ret: web::Json<ApiResult<String>> = web::Json(ApiResult::error(5010, &err.to_string()));"));
    body.push(format!("Ok(HttpResponse::Ok().json(ret))"));
    body.push(format!("}}"));
    body.push(format!("}}"));
    
    let postmacro = format!("#[post(\"{}/{}/save\")]", ctx.codegen_conf.api_handler_prefix.clone(), tbl.api_handler_name.clone().unwrap_or_default());

    let func_name = format!("{}_rel_save", tbl.api_handler_name.clone().unwrap_or_default());
  
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

