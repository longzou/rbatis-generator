use rbatis::error::Error;
use rbatis::rbatis::Rbatis;
use rbatis::DateTimeNative;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TableInfo {
    pub table_catalog: Option<String>,
    pub table_schema: Option<String>,
    pub table_type: Option<String>,
    pub table_name: Option<String>,
    pub table_collation: Option<String>,
    pub table_comment: Option<String>,
    pub create_time: Option<DateTimeNative>,
    pub update_time: Option<DateTimeNative>,
}

impl TableInfo {
    pub async fn load_table(
        rb: &Rbatis,
        table_schema: &str,
        table_name: &str,
    ) -> Result<Option<TableInfo>, Error> {
        // log::info!("TS: {}, TN: {}", table_schema.clone(), table_name.clone());
        let mut rb_args = vec![];
        rb_args.push(rbson::to_bson(table_schema).unwrap_or_default());
        rb_args.push(rbson::to_bson(table_name).unwrap_or_default());

        return rb.fetch::<Option<TableInfo>>(&
        "SELECT table_catalog as table_catalog, table_schema as table_schema, table_type as table_type, 
            table_name as table_name, table_collation as table_collation, table_comment as table_comment, 
            create_time as create_time, update_time as update_time
            FROM INFORMATION_SCHEMA.TABLES WHERE table_schema = ? and table_name = ?",
        rb_args).await ;
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ColumnInfo {
    pub table_schema: Option<String>,
    pub table_name: Option<String>,
    pub column_name: Option<String>,
    pub column_type: Option<String>,
    pub column_comment: Option<String>,
    pub column_key: Option<String>,
    pub column_default: Option<String>,
    pub data_type: Option<String>,
    pub extra: Option<String>,
    pub ordinal_position: Option<i64>,
    pub character_maximum_length: Option<i64>,
    pub is_nullable: Option<String>,
    pub numeric_precision: Option<i64>,
    pub numeric_scale: Option<i64>,
}

impl ColumnInfo {
    //#[sql("SELECT table_schema, table_name,  column_name, column_type, column_comment, column_key,
    //        column_default, data_type, ordinal_position, character_maximum_length, is_nullable, numeric_precision, numeric_scale,
    //        FROM INFORMATION_SCHEMA.COLUMNS WHERE table_schema = ? and table_name = ?")]
    pub async fn load_columns(rb: &Rbatis, ts: &str, tn: &str) -> Result<Vec<Self>, Error> {
        let mut rb_args = vec![];
        rb_args.push(rbson::to_bson(ts).unwrap_or_default());
        rb_args.push(rbson::to_bson(tn).unwrap_or_default());
        // rb.update_by_wrapper(table, w, skips);
        let _con = redis::Client::open("");
        return rb.fetch(&
        "SELECT table_schema as table_schema, table_name as table_name,  column_name as column_name, column_type as column_type, column_comment as column_comment, column_key as column_key,
        column_default as column_default, data_type as data_type, ordinal_position as ordinal_position, character_maximum_length as character_maximum_length, 
        is_nullable as is_nullable, numeric_precision as numeric_precision, numeric_scale as numeric_scale, extra as extra
        FROM INFORMATION_SCHEMA.COLUMNS WHERE table_schema = ? and table_name = ? order by ORDINAL_POSITION ASC ",
        rb_args).await ;
    }
}
