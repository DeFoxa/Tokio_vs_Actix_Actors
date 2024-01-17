use crate::schema::{binancepartialbook, binancetrades};
use diesel::sql_types::{Bool, Float8, Int4, Int8, Jsonb, Nullable, VarChar};
use diesel::Expression;
use diesel::{Insertable, Queryable, Table};
use serde_json::Value as JsonValue;

#[derive(Queryable, Debug)]
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
pub struct BinanceTradesNewModel {
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

#[derive(Queryable, Insertable, Debug)]
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
