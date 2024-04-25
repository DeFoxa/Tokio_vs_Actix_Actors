use crate::schema::{binancepartialbook, binancetrades};
use serde::Serialize;

use diesel::{Insertable, Queryable};
use serde_json::Value as JsonValue;

#[derive(Queryable, Debug, Serialize)]
#[diesel(table_name = binancetrades)]
pub struct BinanceTradesModel {
    pub id: i32,
    pub event_type: Option<String>,
    pub event_time: Option<i64>,
    pub symbol: Option<String>,
    pub aggegate_id: Option<i64>,
    pub price: Option<f64>,
    pub quantity: Option<f64>,
    pub first_trade_id: Option<i64>,
    pub last_trade_id: Option<i64>,
    pub trade_timestamp: Option<i64>,
    pub is_buyer_mm: Option<bool>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = binancetrades)]
pub struct BinanceTradesDBModel {
    pub event_type: Option<String>,
    pub event_time: Option<i64>,
    pub symbol: Option<String>,
    pub aggegate_id: Option<i64>,
    pub price: Option<f64>,
    pub quantity: Option<f64>,
    pub first_trade_id: Option<i64>,
    pub last_trade_id: Option<i64>,
    pub trade_timestamp: Option<i64>,
    pub is_buyer_mm: Option<bool>,
}

#[derive(Queryable, Debug)]
#[diesel(table_name = binancepartialbook)]
pub struct BinancePartialBookModel {
    pub id: i32,
    pub depth_update: Option<String>,
    pub event_timestamp: Option<i64>,
    pub timestamp: Option<i64>,
    pub symbol: Option<String>,
    pub first_update_id: Option<i64>,
    pub final_update_id: Option<i64>,
    pub final_update_id_last_stream: Option<i64>,
    pub bids: Option<JsonValue>,
    pub asks: Option<JsonValue>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = binancepartialbook)]
pub struct BinancePartialBookModelInsertable {
    pub depth_update: Option<String>,
    pub event_timestamp: Option<i64>,
    pub timestamp: Option<i64>,
    pub symbol: Option<String>,
    pub first_update_id: Option<i64>,
    pub final_update_id: Option<i64>,
    pub final_update_id_last_stream: Option<i64>,
    pub bids: Option<JsonValue>,
    pub asks: Option<JsonValue>,
}
