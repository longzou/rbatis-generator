mod codegen;
pub use codegen::*;

mod codegen_entity;
pub use codegen_entity::*;

mod codegen_entity_param;
pub use codegen_entity_param::*;

mod codegen_handler;
pub use codegen_handler::*;

mod codegen_query;
pub use codegen_query::*;

mod codegen_relation;
pub use codegen_relation::*;

mod codegen_yaml;
pub use codegen_yaml::*;

mod codegen_js_api;
pub use codegen_js_api::*;

mod codegen_vue_view;
pub use codegen_vue_view::*;

pub fn is_date_time_type(dt: &String) -> bool {
    match dt.as_str() {
        "rbatis::DateTimeNative" => true,
        "rbatis::DateNative" => true,
        "rbatis::TimeNative" => true,
        "rbatis::DateTimeUtc" => true,
        "rbatis::DateUtc" => true,
        "rbatis::TimeUtc" => true,
        "rbatis::Timestamp" => true,
        "rbatis::TimestampZ" => true,
        "DateTimeNative" => true,
        "DateNative" => true,
        "TimeNative" => true,
        "DateTimeUtc" => true,
        "DateUtc" => true,
        "TimeUtc" => true,
        "Timestamp" => true,
        "TimestampZ" => true,
        _ => false,
    }
}

pub fn is_copied_data_type(dt: &str) -> bool {
    is_copied_type(&parse_data_type_as_rust_type(dt))
}

pub fn is_copied_type(dt: &String) -> bool {
    match dt.as_str() {
        "rbatis::DateTimeNative" => true,
        "rbatis::DateNative" => true,
        "rbatis::TimeNative" => true,
        "rbatis::DateTimeUtc" => true,
        "rbatis::DateUtc" => true,
        "rbatis::TimeUtc" => true,
        "rbatis::Timestamp" => true,
        "rbatis::TimestampZ" => true,
        "DateTimeNative" => true,
        "DateNative" => true,
        "TimeNative" => true,
        "DateTimeUtc" => true,
        "DateUtc" => true,
        "TimeUtc" => true,
        "Timestamp" => true,
        "TimestampZ" => true,
        "i64" => true,
        "i32" => true,
        "usize" => true,
        "u32" => true,
        "u64" => true,
        "i128" => true,
        "u128" => true,
        "bool" => true,
        "f32" => true,
        "f64" => true,
        _ => false,
    }
}

pub fn is_multi_item_field(safe_fdname: &String) -> bool {
    safe_fdname.ends_with("id")
        || safe_fdname.ends_with("status")
        || safe_fdname.ends_with("category")
        || safe_fdname.ends_with("_no")
        || safe_fdname.ends_with("_type")
        || safe_fdname.ends_with("code")
}

pub fn parse_data_type_as_rust_type(dt: &str) -> String {
    match dt {
        "smallint" => "i16".to_string(),
        "smallint unsigned" => "u16".to_string(),
        "int" => "i32".to_string(),
        "int unsigned" => "u32".to_string(),
        "bigint" => "i64".to_string(),
        "bigint unsigned" => "u64".to_string(),
        "tinyint" => "bool".to_string(),
        "tinyint unsigned" => "bool".to_string(),
        "tinyint signed" => "bool".to_string(),
        "boolean" => "bool".to_string(),
        "bit" => "i32".to_string(),
        "longtext" => "String".to_string(),
        "text" => "String".to_string(),
        "mediumtext" => "String".to_string(),
        "char" => "String".to_string(),
        "varchar" => "String".to_string(),
        "float" => "f32".to_string(),
        "double" => "f64".to_string(),
        "decimal" => "rbatis::Decimal".to_string(),
        "datetime" => "rbatis::DateTimeNative".to_string(),
        "date" => "rbatis::DateNative".to_string(),
        "timestamp" => "rbatis::Timestamp".to_string(),
        "time" => "rbatis::TimeNative".to_string(),
        "blob" => "rbatis::Bytes".to_string(),
        "binary" => "rbatis::Bytes".to_string(),
        "varbinary" => "rbatis::Bytes".to_string(),
        "mediumblob" => "rbatis::Bytes".to_string(),
        "longblob" => "rbatis::Bytes".to_string(),
        "json" => "rbatis::Json".to_string(),
        _ => "String".to_string(),
    }
}
