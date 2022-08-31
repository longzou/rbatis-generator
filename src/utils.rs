use std::cell::Ref;
use std::fmt::{Debug};
use std::time::{SystemTime};
use serde_derive::{Deserialize, Serialize};
use chrono::offset::Local;
use chrono::DateTime;
use redis::aio::Connection;
use redis::AsyncCommands;
use crate::config::{AppConfig, get_redis_manager};


pub fn num_to_string (n:i64) -> String {
    let base_codec = ['A','B','C','D','E','F','G','H','J','K','L','M','N','O', 'P','Q','R','S','T','U','V','W','X','Y','Z','2','3','4','5','7','8','9'];
    let len = base_codec.len() as i64;
    let mut t = n;
    let mut result = "".to_string();
    while t > 0 {
        let idx = (t % len as i64) as usize;
        let ch = base_codec[idx];
        t = t / len;
        result.insert(0, ch);
    }
    result
}

pub fn f32_to_decimal(f: f32) -> Option<rbatis::Decimal> {
    match rbatis::Decimal::from_str(format!("{:.2}", f).as_str()) {
        Ok(r) => {
            Some(r)
        }
        Err(_) => {
            None
        }
    }
}

pub fn decimal_to_f32(dc: Option<rbatis::Decimal>) -> f32 {
    match dc {
        Some(r) => {
            match r.to_string().parse::<f32>() {
                Ok(t) => {
                    t
                }
                Err(_) => {
                    0f32
                }
            }
        }
        None => {
            0f32
        }
    }
}

pub fn make_decimal_negative(dc: Option<rbatis::Decimal>) -> Option<rbatis::Decimal> {
    match dc {
        Some(r) => {
            match r.to_string().parse::<f32>() {
                Ok(t) => {
                    f32_to_decimal(-t)
                }
                Err(_) => {
                    f32_to_decimal(0f32)
                }
            }
        }
        None => {
            f32_to_decimal(0f32)
        }
    }
}

pub fn generate_rand_string (len: usize) -> String {
    let mut retkey = "".to_string();

    while retkey.len() < len {
        let rng = rand::random::<u16>();
        let key = num_to_string(rng as i64);
        retkey = retkey + key.as_str();
    }

    retkey.chars().take(len).collect()
}

pub fn get_local_timestamp() -> u64 {
    let now = SystemTime::now();
    let date:DateTime<Local> = now.into();
    date.timestamp_millis() as u64
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct  ApiResult <T> {
    pub status: i32,
    pub message: String,
    pub data: Option<T>,
    pub timestamp: Option<u64>,
}

impl<T> ApiResult<T> {

    pub fn ok (dt: T) -> Self {
        ApiResult {
            status: 200,
            message: "OK".to_string(),
            data: Option::Some(dt),
            timestamp: Some(get_local_timestamp())
        }
    }

    pub fn error (code: i32, msg: &String) -> Self {
        ApiResult {
            status: code,
            message: msg.to_owned(),
            data: None,
            timestamp: Some(get_local_timestamp())
        }
    }

    pub fn new (code: i32, msg: &String, data: T, ts: u64) -> Self {
        ApiResult {
            status: code,
            message: msg.to_owned(),
            data: Some(data),
            timestamp: Some(ts)
        }
    }
}


pub async fn redis_get_string(key: &str) -> redis::RedisResult<String>
{
    let mut rcnn = get_redis_manager();
    redis::cmd("GET").arg(key).query_async::<Connection, String>(&mut rcnn).await
}

pub async fn redis_set_string<'a, T>(key: &str, t: &T) -> redis::RedisResult<String>
where
    T: serde::ser::Serialize
{
    let mut rcnn = get_redis_manager();
    let text = serde_json::to_string(t).unwrap_or_default();
    redis::cmd("SET").arg(key).arg(text).query_async::<Connection, String>(&mut rcnn).await
}
