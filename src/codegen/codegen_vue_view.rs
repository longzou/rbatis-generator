use crate::codegen::GenerateContext;
use crate::config::TEMPLATES;
use crate::schema::TableInfo;
use std::collections::HashSet;
use tera::Context;

use super::{parse_composite_column_list, tera_pascal, RelationForm, RustStruct};

pub fn generate_vue_view_for_table(
    ctx: &mut GenerateContext,
    tbl: &TableInfo,
    comps: &Option<RustStruct>,
) -> Vec<String> {
    let mut tera = TEMPLATES.clone();
    let tbl_name = tbl.table_name.clone().unwrap_or_default();
    tera.register_function("pascal", tera_pascal);

    let tbl = ctx.get_table_info(&tbl_name);
    let tbc = ctx.get_table_conf(&tbl_name);

    let cols = ctx.get_table_columns(&tbl_name);

    let mut relform = RelationForm::default();
    relform.codegen = ctx.codegen_conf.clone();
    relform.table_info = tbl.clone();
    relform.table_conf = tbc.clone();
    relform.relation_conf = ctx.get_relation_config(&tbl_name);
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

    relform.dict_list = relform
        .dict_list
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    relform.relation_count = relform.relations.len() as u64;

    let mut context = match Context::from_serialize(&relform) {
        Ok(c) => c,
        Err(_) => Context::new(),
    };

    context.insert("have_composition_handler", &comps.is_some());
    if comps.is_some() {
        context.insert("composition_struct", &comps.clone().unwrap());
    }

    match tera.render("index.vue", &context) {
        Ok(text) => vec![text],
        Err(err) => {
            log::info!(
                "Error for parse the index.vue for {} template: {}",
                tbl_name.clone(),
                err
            );
            vec![]
        }
    }
}
