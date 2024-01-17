use self::schema::binancetrades;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use serde::Deserialize;
use std::env;
//

//
fn main() {
    let test = BinanceTrades {
        event_type: "testing".to_string(),
        event_time: 1234,
        symbol: "TEST_BTC".to_string(),
        aggregate_id: 1,
        price: 46999.01,
        quantity: 1.0,
        first_trade_id: 2,
        last_trade_id: 4,
        trade_timestamp: 56789,
        is_buyer_mm: false,
    };
    let connection = &mut establish_connection;
    let entry = diesel::insert_into(binancetrades)
        .values(test)
        .execute(connection);
    // todo!();
}
fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL not found, must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("error connecting to DB: {}", database_url))
}

#[derive(Debug, Clone, Deserialize)]
pub struct BinanceTrades {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: i64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "a")]
    pub aggregate_id: i64,
    #[serde(rename = "p")]
    pub price: f64,
    #[serde(rename = "q")]
    pub quantity: f64,
    #[serde(rename = "f")]
    pub first_trade_id: i64,
    #[serde(rename = "l")]
    pub last_trade_id: i64,
    #[serde(rename = "T")]
    pub trade_timestamp: i64,
    #[serde(rename = "m")]
    pub is_buyer_mm: bool,
}
