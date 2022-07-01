/**
 * Generate the file for chimes_permission_info.rs, 
 */

use std::fmt::{Debug};
use serde_derive::{Deserialize, Serialize};
use rbatis::crud_table;
use rbatis::rbatis::{Rbatis};
use rbatis::error::Error;
use rbatis::Page;
use rbatis::PageRequest;
use rbson::Bson;
use rbatis::crud::{CRUD, Skip};

#[crud_table(table_name:"chimes_permission"|table_columns:"id,alias,create_time,name,pid,api_pattern,service_id,api_method,api_bypass")]
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ChimesPermissionInfo {
    pub id: Option<i64>,
    pub alias: Option<String>,
    pub create_time: Option<rbatis::DateTimeNative>,
    pub name: Option<String>,
    pub pid: Option<i64>,
    pub api_pattern: Option<String>,
    pub service_id: Option<String>,
    pub api_method: Option<String>,
    pub api_bypass: Option<String>,
}


impl ChimesPermissionInfo {
    #[allow(dead_code)]
    pub async fn from_id(rb: &Rbatis,id: &i64) -> Result<Option<Self>, Error> {
        let wp = rb.new_wrapper()
            .eq("id", id);
        rb.fetch_by_wrapper::<Option<Self>>(wp).await
    }


    #[allow(dead_code)]
    pub async fn save(&mut self,rb: &Rbatis) -> Result<u64, Error> {
        match rb.save(self, &[Skip::Column("id"),Skip::Column("create_time"),Skip::Column("id")]).await {
            Ok(ds) => {
                self.id = ds.last_insert_id;
                Ok(ds.rows_affected)
            }
            Err(err) => {
                Err(err)
            }
        }
    }


    #[allow(dead_code)]
    pub async fn update(&self,rb: &Rbatis) -> Result<u64, Error> {
        let wp = rb.new_wrapper()
            .eq("id", self.id);
        rb.update_by_wrapper(self, wp, &[Skip::Column("id"),Skip::Column("create_time"),Skip::Column("id")]).await
    }


    #[allow(dead_code)]
    pub async fn update_selective(&self,rb: &Rbatis) -> Result<u64, Error> {
        let wp = rb.new_wrapper()
            .eq("id", self.id);
        rb.update_by_wrapper(self, wp, &[Skip::Value(Bson::Null)]).await
    }


    #[allow(dead_code)]
    pub async fn remove_batch(&self,rb: &Rbatis) -> Result<u64, Error> {
        let wp = rb.new_wrapper()
                 .r#if(self.id.clone().is_some(), |w| w.and().eq("id", self.id.clone().unwrap()))
                 .r#if(self.alias.clone().is_some(), |w| w.and().eq("alias", self.alias.clone().unwrap()))
                 .r#if(self.create_time.clone().is_some(), |w| w.and().eq("create_time", self.create_time.clone().unwrap()))
                 .r#if(self.name.clone().is_some(), |w| w.and().eq("name", self.name.clone().unwrap()))
                 .r#if(self.pid.clone().is_some(), |w| w.and().eq("pid", self.pid.clone().unwrap()))
                 .r#if(self.api_pattern.clone().is_some(), |w| w.and().eq("api_pattern", self.api_pattern.clone().unwrap()))
                 .r#if(self.service_id.clone().is_some(), |w| w.and().eq("service_id", self.service_id.clone().unwrap()))
                 .r#if(self.api_method.clone().is_some(), |w| w.and().eq("api_method", self.api_method.clone().unwrap()))
                 .r#if(self.api_bypass.clone().is_some(), |w| w.and().eq("api_bypass", self.api_bypass.clone().unwrap()));
        rb.remove_by_wrapper::<Self>(wp).await
    }


    #[allow(dead_code)]
    pub async fn remove(&mut self,rb: &Rbatis) -> Result<u64, Error> {
        let wp = rb.new_wrapper()
            .eq("id", self.id);
        rb.remove_by_wrapper::<Self>(wp).await
    }


    #[allow(dead_code)]
    pub async fn remove_ids(rb: &Rbatis,ids: &[i64]) -> Result<u64, Error> {
        let wp = rb.new_wrapper()
            .r#in("id", ids);
        rb.remove_by_wrapper::<Self>(wp).await
    }


    #[allow(dead_code)]
    pub async fn query_paged(&self,rb: &Rbatis,curr: u64,ps: u64) -> Result<Page<Self>, Error> {
        let wp = rb.new_wrapper()
                 .r#if(self.id.clone().is_some(), |w| w.and().eq("id", self.id.clone().unwrap()))
                 .r#if(self.alias.clone().is_some(), |w| w.and().eq("alias", self.alias.clone().unwrap()))
                 .r#if(self.create_time.clone().is_some(), |w| w.and().eq("create_time", self.create_time.clone().unwrap()))
                 .r#if(self.name.clone().is_some(), |w| w.and().eq("name", self.name.clone().unwrap()))
                 .r#if(self.pid.clone().is_some(), |w| w.and().eq("pid", self.pid.clone().unwrap()))
                 .r#if(self.pid.clone().is_none(), |w| w.and().eq("pid", Some(0)))
                 .r#if(self.api_pattern.clone().is_some(), |w| w.and().eq("api_pattern", self.api_pattern.clone().unwrap()))
                 .r#if(self.service_id.clone().is_some(), |w| w.and().eq("service_id", self.service_id.clone().unwrap()))
                 .r#if(self.api_method.clone().is_some(), |w| w.and().eq("api_method", self.api_method.clone().unwrap()))
                 .r#if(self.api_bypass.clone().is_some(), |w| w.and().eq("api_bypass", self.api_bypass.clone().unwrap()));
        rb.fetch_page_by_wrapper::<Self>(wp, &PageRequest::new(curr, ps)).await
    }


    #[allow(dead_code)]
    pub async fn query_list(&self,rb: &Rbatis) -> Result<Vec<Self>, Error> {
        let wp = rb.new_wrapper()
                 .r#if(self.id.clone().is_some(), |w| w.and().eq("id", self.id.clone().unwrap()))
                 .r#if(self.alias.clone().is_some(), |w| w.and().eq("alias", self.alias.clone().unwrap()))
                 .r#if(self.create_time.clone().is_some(), |w| w.and().eq("create_time", self.create_time.clone().unwrap()))
                 .r#if(self.name.clone().is_some(), |w| w.and().eq("name", self.name.clone().unwrap()))
                 .r#if(self.pid.clone().is_some(), |w| w.and().eq("pid", self.pid.clone().unwrap()))
                 .r#if(self.api_pattern.clone().is_some(), |w| w.and().eq("api_pattern", self.api_pattern.clone().unwrap()))
                 .r#if(self.service_id.clone().is_some(), |w| w.and().eq("service_id", self.service_id.clone().unwrap()))
                 .r#if(self.api_method.clone().is_some(), |w| w.and().eq("api_method", self.api_method.clone().unwrap()))
                 .r#if(self.api_bypass.clone().is_some(), |w| w.and().eq("api_bypass", self.api_bypass.clone().unwrap()));
        rb.fetch_list_by_wrapper::<Self>(wp).await
    }


    #[allow(dead_code)]
    pub async fn query_all(rb: &Rbatis) -> Result<Vec<Self>, Error> {
        let wp = rb.new_wrapper();
        rb.fetch_list_by_wrapper::<Self>(wp).await
    }


    #[allow(dead_code)]
    pub async fn query_tree(rb: &Rbatis,pid: &Option<i64>) -> Result<Vec<Self>, Error> {
        let wp = rb.new_wrapper()
                 .r#if(pid.clone().is_some(), |w| w.and().eq("pid", pid.unwrap()))
                 .r#if(pid.clone().is_none(), |w| w.and().eq("pid", 0));
        rb.fetch_list_by_wrapper::<Self>(wp).await
    }

    pub async fn save_or_update(&mut self, rb: &Rbatis) -> Result<u64, Error> {
        let mut query = ChimesPermissionInfo::default();
        query.alias = self.alias.clone();
        query.service_id = self.service_id.clone();
        match query.query_list(rb).await {
            Ok(rs) => {
                if rs.len() > 0 {
                    let mut perm = rs[0].clone();
                    // perm.api_bypass = self.api_bypass.clone();
                    perm.api_method = self.api_method.clone();
                    perm.api_pattern = self.api_pattern.clone();
                    match perm.update(rb).await {
                        Ok(r) => {
                            Ok(r)
                        }
                        Err(err) => {
                            Err(err)
                        }
                    }
                } else {
                    match self.save(rb).await {
                        Ok(r) => {
                            Ok(r)
                        }
                        Err(err) => {
                            Err(err)
                        }
                    }
                }
            }
            Err(err) => {
                log::info!("Error: {}", err.to_string());
                Err(err)
            }
        }
    }



}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ChimesPermissionInfoValue {
    pub id: Option<i64>,
    pub alias: Option<String>,
    pub create_time: Option<rbatis::DateTimeNative>,
    pub name: Option<String>,
    pub pid: Option<i64>,
    pub api_pattern: Option<String>,
    pub service_id: Option<String>,
    pub api_method: Option<String>,
    pub api_bypass: Option<String>,
    pub leaf: bool,
    pub label: Option<String>,
    pub has_children: bool,
    #[serde(default)]
    pub children: Vec<ChimesPermissionInfoValue>,
}


impl ChimesPermissionInfoValue {
    #[allow(dead_code)]
    pub fn from_entity(param: &ChimesPermissionInfo) -> Self {
        Self {
            id: param.id.clone(),
            alias: param.alias.clone(),
            create_time: param.create_time.clone(),
            name: param.name.clone(),
            pid: param.pid.clone(),
            api_pattern: param.api_pattern.clone(),
            service_id: param.service_id.clone(),
            api_method: param.api_method.clone(),
            api_bypass: param.api_bypass.clone(),
            has_children: false,
            leaf: false,
            children: vec![],
            label: param.name.clone(),
        }
    }


    #[allow(dead_code)]
    pub fn from_entity_with(param: &ChimesPermissionInfo,haschild: bool,children: &Vec<Self>) -> Self {
        Self {
            id: param.id.clone(),
            alias: param.alias.clone(),
            create_time: param.create_time.clone(),
            name: param.name.clone(),
            pid: param.pid.clone(),
            api_pattern: param.api_pattern.clone(),
            service_id: param.service_id.clone(),
            api_method: param.api_method.clone(),
            api_bypass: param.api_bypass.clone(),
            has_children: haschild,
            leaf: haschild == false,
            children: children.clone(),
            label: param.name.clone(),
        }
    }


    #[allow(dead_code)]
    pub fn to_entity(&self) -> ChimesPermissionInfo {
        ChimesPermissionInfo {
            id: self.id.clone(),
            alias: self.alias.clone(),
            create_time: self.create_time.clone(),
            name: self.name.clone(),
            pid: self.pid.clone(),
            api_pattern: self.api_pattern.clone(),
            service_id: self.service_id.clone(),
            api_method: self.api_method.clone(),
            api_bypass: self.api_bypass.clone(),
        }
    }


    fn recurse_build_tree(items: &Vec<Self>,parent_item: &mut Self) {
        for xip in items.clone() {
            if xip.pid == parent_item.id {
                let mut mip = xip;
                Self::recurse_build_tree(items, &mut mip);
                if mip.children.is_empty() {
                    mip.leaf = true;
                    mip.has_children = false;
                }
                parent_item.children.push(mip);
            }
        }
    }


    #[allow(dead_code)]
    pub fn build_tree(items: &Vec<Self>) -> Vec<Self> {
        let mut tmptree = vec![];
        for xip in items.clone() {
            if xip.pid.is_none() || xip.pid == Some(0) {
                tmptree.push(xip.clone());
            }
        }
        let mut tree = vec![];
        for mut it in tmptree {
            Self::recurse_build_tree(items, &mut it);
            tree.push(it);
        }
        tree
    }


    

}

