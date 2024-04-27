use std::{fs::File, vec};

use change_case::{pascal_case, snake_case};
use substring::Substring;
use yaml_rust::{yaml, Yaml};

use super::{RustFileImpl, RustFunc, RustStruct, RustStructField};

#[allow(dead_code)]
pub fn parse_yaml_as_file(conf_path: &str, filename: &String) -> RustFileImpl {
    let sts = parse_yaml_as_struct(conf_path);
    let mut usinglist = vec![];
    usinglist.push("std::collections::HashMap".to_string());
    usinglist.push("std::fs::File".to_string());
    usinglist.push("std::io::Read".to_string());
    usinglist.push("std::io::Error".to_string());
    usinglist.push("std::mem::MaybeUninit".to_string());
    usinglist.push("std::sync::{Mutex, Once}".to_string());
    usinglist.push("std::fmt::Debug".to_string());
    usinglist.push("serde_derive::{Deserialize, Serialize}".to_string());
    usinglist.push("change_case::snake_case".to_string());
    usinglist.push("yaml_rust::{Yaml, yaml}".to_string());

    RustFileImpl {
        file_name: filename.clone(),
        mod_name: "conf".to_owned(),
        caretlist: vec![],
        usinglist: usinglist,
        structlist: sts,
        funclist: vec![],
    }
}

/**
 * 根据Yaml文件生成与之对应的解释程序
 */

pub fn parse_yaml_as_struct(conf_path: &str) -> Vec<RustStruct> {
    // open file
    let mut f = match File::open(conf_path) {
        Ok(f) => f,
        Err(_) => return vec![],
    };
    let mut s = String::new();
    use std::io::Read;

    match f.read_to_string(&mut s) {
        Ok(_) => {}
        Err(_) => {}
    };
    // f.read_to_string(&mut s).unwrap(); // read file content to s
    // load string to yaml loader
    let docs = yaml::YamlLoader::load_from_str(&s).unwrap();
    // get first yaml hash doc
    // get server value
    // let server = yaml_doc["weapp"].clone();
    let mut parsedst = vec![];
    let mut docst = RustStruct {
        is_pub: true,
        has_paging: false,
        struct_name: "AppConfig".to_string(),
        annotations: vec!["#[derive(Debug, Clone, Default, Deserialize, Serialize)]".to_string()],
        fields: vec![],
        funclist: vec![],
        usings: vec![],
    };
    parse_yaml_node_as_struct(&docs[0], &mut docst, &mut parsedst);
    generate_load_from_node_for_struct(&mut docst);
    generate_load_from_yaml_for_struct(&mut docst);
    generate_single_get_for_struct(&mut docst);
    add_struct_into_vec(&mut parsedst, docst.clone());

    parsedst
}

/**
 * 将当前节点解析成为具体的struct，并加入到struct list中
 */
fn parse_yaml_node_as_struct(doc: &Yaml, st: &mut RustStruct, stlist: &mut Vec<RustStruct>) {
    let hash = doc.as_hash();
    match hash {
        Some(hs) => {
            for ks in hs.clone() {
                let key = ks.0.as_str().unwrap_or_default();
                let mut option_fd = true;
                let value_type = match ks.1.clone() {
                    Yaml::Real(_) => "f64".to_string(),
                    Yaml::Integer(_) => "i64".to_string(),
                    Yaml::String(_) => "String".to_string(),
                    Yaml::Boolean(_) => "bool".to_string(),
                    Yaml::Array(tt) => {
                        let fnd = &tt[0];
                        let fnt = match fnd.clone() {
                            Yaml::Real(_) => "f64".to_string(),
                            Yaml::Integer(_) => "i64".to_string(),
                            Yaml::String(_) => "String".to_string(),
                            Yaml::Boolean(_) => "bool".to_string(),
                            Yaml::Array(st) => {
                                let sttype = format!("{}Config", pascal_case(key));
                                let mut newst = RustStruct {
                                    is_pub: true,
                                    has_paging: false,
                                    struct_name: sttype.clone(),
                                    annotations: vec![
                                        "#[derive(Debug, Clone, Default, Deserialize, Serialize)]"
                                            .to_string(),
                                    ],
                                    fields: vec![],
                                    funclist: vec![],
                                    usings: vec![],
                                };

                                parse_yaml_node_as_struct(&st[0], &mut newst, stlist);
                                generate_load_from_node_for_struct(&mut newst);
                                add_struct_into_vec(stlist, newst.clone());

                                // option_fd = false;
                                sttype
                            }
                            Yaml::Hash(_) => {
                                let sttype = format!("{}Config", pascal_case(key));
                                let mut newst = RustStruct {
                                    is_pub: true,
                                    has_paging: false,
                                    struct_name: sttype.clone(),
                                    annotations: vec![
                                        "#[derive(Debug, Clone, Default, Deserialize, Serialize)]"
                                            .to_string(),
                                    ],
                                    fields: vec![],
                                    funclist: vec![],
                                    usings: vec![],
                                };
                                parse_yaml_node_as_struct(&fnd.clone(), &mut newst, stlist);
                                generate_load_from_node_for_struct(&mut newst);
                                add_struct_into_vec(stlist, newst.clone());
                                // option_fd = false;

                                sttype
                            }
                            _ => "String".to_string(),
                        };
                        option_fd = false;
                        format!("Vec<{}>", fnt)
                    }
                    Yaml::Hash(_) => {
                        let sttype = format!("{}Config", pascal_case(key));

                        let mut newst = RustStruct {
                            is_pub: true,
                            has_paging: false,
                            struct_name: sttype.clone(),
                            annotations: vec![
                                "#[derive(Debug, Clone, Default, Deserialize, Serialize)]"
                                    .to_string(),
                            ],
                            fields: vec![],
                            funclist: vec![],
                            usings: vec![],
                        };

                        parse_yaml_node_as_struct(&ks.1.clone(), &mut newst, stlist);
                        generate_load_from_node_for_struct(&mut newst);
                        add_struct_into_vec(stlist, newst.clone());

                        sttype
                    }
                    Yaml::Alias(_) => {
                        log::info!("Alias for Yaml line: {}", key);
                        "String".to_string()
                    }
                    Yaml::Null => {
                        log::info!("Null for Yaml line: {}", key);
                        "String".to_string()
                    }
                    Yaml::BadValue => {
                        log::info!("BadValue for Yaml line: {}", key);
                        "String".to_string()
                    }
                };
                st.fields.push(RustStructField {
                    is_pub: true,
                    schema_name: None,
                    column_name: key.to_string(),
                    field_name: snake_case(key).to_string(),
                    field_type: value_type,
                    is_option: option_fd,
                    orignal_field_name: None,
                    comment: None,
                    length: 0i64,
                    annotations: vec![],
                });
            }
            // stlist.push(st.clone());
            // add_struct_into_vec(stlist, st.clone());
        }
        None => {}
    }
}

fn generate_load_from_node_for_struct(st: &mut RustStruct) {
    let mut params = vec![];
    params.push(("node".to_string(), "&Yaml".to_string()));

    let mut body = vec![];
    body.push(format!("{} {{", st.struct_name.clone()));

    for rstfd in st.fields.clone() {
        match rstfd.field_type.clone().as_str() {
            "String" => {
                body.push(format!(
                    "{}: match node[\"{}\"].as_str() {{",
                    rstfd.field_name, rstfd.column_name
                ));
                body.push(format!("Some(st) => Some(st.to_string()),"));
                body.push(format!("None => None,"));
                body.push(format!("}},"));
            }
            "i64" => {
                body.push(format!(
                    "{}: node[\"{}\"].as_i64(),",
                    rstfd.field_name, rstfd.column_name
                ));
            }
            "f64" => {
                body.push(format!(
                    "{}: node[\"{}\"].as_f64(),",
                    rstfd.field_name, rstfd.column_name
                ));
            }
            "bool" => {
                body.push(format!(
                    "{}: node[\"{}\"].as_bool(),",
                    rstfd.field_name, rstfd.column_name
                ));
            }
            "Vec<String>" => {
                body.push(format!(
                    "{}: match node[\"{}\"].as_vec() {{",
                    rstfd.field_name, rstfd.column_name
                ));
                body.push(format!("Some(vst) => {{"));
                body.push(format!("let mut vlist = vec![];"));
                body.push(format!("for xst in vst.clone() {{"));
                body.push(format!("let s = match xst.as_str() {{"));
                body.push(format!("Some(st) => Some(st.to_string()),"));
                body.push(format!("None => None,"));
                body.push(format!("}};"));
                body.push(format!("if s.is_some() {{"));
                body.push(format!("vlist.push(s.unwrap_or_default());"));
                body.push(format!("}}"));
                body.push(format!("vlist"));
                body.push(format!("}}"));
                body.push(format!("}},"));
                body.push(format!("None => vec![],"));
                body.push(format!("}},"));
            }
            "Vec<i64>" => {
                body.push(format!(
                    "{}: match node[\"{}\"].as_vec() {{",
                    rstfd.field_name, rstfd.column_name
                ));
                body.push(format!("Some(vst) => {{"));
                body.push(format!("let mut vlist = vec![];"));
                body.push(format!("for xst in vst.clone() {{"));
                body.push(format!("let s = xst.as_i64();"));
                body.push(format!("if s.is_some() {{"));
                body.push(format!("vlist.push(s.unwrap_or_default());"));
                body.push(format!("}}"));
                body.push(format!("}}"));
                body.push(format!("vlist"));
                body.push(format!("}},"));
                body.push(format!("None => vec![],"));
                body.push(format!("}},"));
            }
            "Vec<f64>" => {
                body.push(format!(
                    "{}: match node[\"{}\"].as_vec() {{",
                    rstfd.field_name, rstfd.column_name
                ));
                body.push(format!("Some(vst) => {{"));
                body.push(format!("let mut vlist = vec![];"));
                body.push(format!("for xst in vst.clone() {{"));
                body.push(format!("let s = xst.as_f64();"));
                body.push(format!("if s.is_some() {{"));
                body.push(format!("vlist.push(s.unwrap_or_default());"));
                body.push(format!("}}"));
                body.push(format!("}}"));
                body.push(format!("vlist"));
                body.push(format!("}},"));
                body.push(format!("None => vec![],"));
                body.push(format!("}},"));
            }
            "Vec<bool>" => {
                body.push(format!(
                    "{}: match node[\"{}\"].as_vec() {{",
                    rstfd.field_name, rstfd.column_name
                ));
                body.push(format!("Some(vst) => {{"));
                body.push(format!("let mut vlist = vec![];"));
                body.push(format!("for xst in vst.clone() {{"));
                body.push(format!("let s = xst.as_bool();"));
                body.push(format!("if s.is_some() {{"));
                body.push(format!("vlist.push(s.unwrap_or_default());"));
                body.push(format!("}}"));
                body.push(format!("}}"));
                body.push(format!("vlist"));
                body.push(format!("}},"));
                body.push(format!("None => vec![],"));
                body.push(format!("}},"));
            }
            _ => {
                let fdtype = rstfd.field_type.clone();
                if fdtype.starts_with("Vec<") && fdtype.ends_with(">") {
                    let field_type = fdtype.substring(4, fdtype.len() - 1);
                    body.push(format!(
                        "{}: match node[\"{}\"].as_vec() {{",
                        rstfd.field_name.clone(),
                        rstfd.column_name.clone()
                    ));
                    body.push(format!("Some(vst) => {{"));
                    body.push(format!("let mut vlist = vec![];"));
                    body.push(format!("for xst in vst.clone() {{"));
                    body.push(format!("let sop = {}::load_from_node(&xst);", field_type));
                    body.push(format!("vlist.push(sop);"));
                    body.push(format!("}}"));
                    body.push(format!("vlist"));
                    body.push(format!("}},"));
                    body.push(format!("None => vec![],"));
                    body.push(format!("}},"));
                } else {
                    if rstfd.is_option {
                        body.push(format!(
                            "{}: Some({}::load_from_node(&node[\"{}\"])), ",
                            rstfd.field_name.clone(),
                            rstfd.field_type.clone(),
                            rstfd.column_name.clone()
                        ));
                    } else {
                        body.push(format!(
                            "{}: {}::load_from_node(&node[\"{}\"]), ",
                            rstfd.field_name.clone(),
                            rstfd.field_type.clone(),
                            rstfd.column_name.clone()
                        ));
                    }
                }
            }
        }
    }

    body.push(format!("}}"));

    let func = RustFunc {
        is_struct_fn: true,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: false,
        is_async: false,
        func_name: "load_from_node".to_string(),
        return_is_option: false,
        return_is_result: false,
        return_type: Some(st.struct_name.clone()),
        params: params,
        bodylines: body,
        macros: vec![],
        comment: None,
        api_method: None,
        api_pattern: None,
    };
    st.funclist.push(func);
}

fn add_struct_into_vec(list: &mut Vec<RustStruct>, st: RustStruct) {
    for li in list.clone() {
        if li.struct_name == st.struct_name {
            return;
        }
    }
    list.push(st);
}

fn generate_load_from_yaml_for_struct(st: &mut RustStruct) {
    let mut params = vec![];
    params.push(("conf_path".to_string(), "&String".to_string()));

    let mut body = vec![];

    body.push(format!("let mut f = match File::open(conf_path) {{"));
    body.push(format!("Ok(f) => f,"));
    body.push(format!("Err(err) => {{"));
    body.push(format!("return Err(err);"));
    body.push(format!("}}"));
    body.push(format!("}};"));

    body.push(format!("let mut s = String::new();"));
    body.push(format!("match f.read_to_string(&mut s) {{"));
    body.push(format!("Ok(_) => {{}}"));
    body.push(format!("Err(err) => {{"));
    body.push(format!("return Err(err);"));
    body.push(format!("}}"));
    body.push(format!("}};"));

    body.push(format!(
        "let docs = yaml::YamlLoader::load_from_str(&s).unwrap();"
    ));
    body.push(format!("let doc = &docs[0];"));

    body.push(format!(
        "let conf = {}::load_from_node(doc);",
        st.struct_name.clone()
    ));

    for fd in st.fields.clone() {
        body.push(format!(
            "self.{} = conf.{}.clone();",
            fd.field_name.clone(),
            fd.field_name.clone()
        ));
    }

    body.push(format!("Ok(conf)"));

    let func = RustFunc {
        is_struct_fn: true,
        is_self_fn: true,
        is_self_mut: true,
        is_pub: true,
        is_async: false,
        func_name: "load_yaml_file".to_string(),
        return_is_option: false,
        return_is_result: true,
        return_type: Some(st.struct_name.clone()),
        params: params,
        bodylines: body,
        macros: vec![],
        comment: None,
        api_method: None,
        api_pattern: None,
    };
    st.funclist.push(func);
}

fn generate_single_get_for_struct(st: &mut RustStruct) {
    let params = vec![];
    let mut body = vec![];

    body.push(format!(
        "static mut CONF: MaybeUninit<Mutex<{}>> = MaybeUninit::uninit();",
        st.struct_name.clone()
    ));
    body.push(format!("// Once带锁保证只进行一次初始化"));
    body.push(format!("static ONCE: Once = Once::new();"));

    body.push(format!("ONCE.call_once(|| unsafe {{"));
    body.push(format!(
        "CONF.as_mut_ptr().write(Mutex::new({}::default()));",
        st.struct_name.clone()
    ));
    body.push(format!("}});"));
    body.push(format!("unsafe {{ &*CONF.as_ptr() }}"));

    let func = RustFunc {
        is_struct_fn: true,
        is_self_fn: false,
        is_self_mut: false,
        is_pub: true,
        is_async: false,
        func_name: "get".to_string(),
        return_is_option: false,
        return_is_result: false,
        return_type: Some(format!("&'static Mutex<{}>", st.struct_name.clone())),
        params: params,
        bodylines: body,
        macros: vec![],
        comment: None,
        api_method: None,
        api_pattern: None,
    };
    st.funclist.push(func);
}
