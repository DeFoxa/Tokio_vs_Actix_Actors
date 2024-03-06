// use crate::binancetrades;
use crate::models::*;
// use crate::schema::*;
use crate::schema::{binancepartialbook, binancetrades};
use crate::utils::*;
use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::Insertable;
use serde::{
    de::{self, Deserializer as deser, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub trait ToTakerTrades {
    fn to_trades_type(&self) -> Result<TakerTrades>;
}

pub trait ToBookModels {
    fn to_bookstate(&self) -> Result<BookState>;
    fn to_book_model(&self) -> Result<BookModel>;
}

#[derive(Debug, Clone)]
pub struct TakerTrades {
    pub symbol: String,
    pub side: Side,
    pub price: f64,
    pub qty: f64,
    pub local_ids: u32,
    pub exch_id: i64,
    pub transaction_timestamp: i64,
}

impl fmt::Display for TakerTrades {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "symbol: {}, side: {}, price: {}, qty: {}, local_ids: {}, exch_id: {}, transaction_timestamp: {} ",
            self.symbol,
            self.side,
            self.price,
            self.qty,
            self.local_ids,
            self.exch_id,
            self.transaction_timestamp,
        )
    }
}

#[derive(Debug, Clone)]
pub struct BookModel {
    pub symbol: String,
    pub bids: Vec<Quotes>,
    pub asks: Vec<Quotes>,
    pub timestamp: i64,
    // pub exchange_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Quotes {
    pub levels: f64,
    pub qtys: f64,
    pub count: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct BookState {
    pub timestamp: Option<DateTime<Utc>>,
    pub mid: f32,
    pub bb_level: f32,
    pub bo_level: f32,
    pub bb_quantity: f32,
    pub bo_quantity: f32,
    pub bid_depth: f32, // quantity quoted calculated to 2% from bb
    pub ask_depth: f32, // quantity quoted calculated to 2% from bo
    pub spread: f32,
    pub slippage_per_tick: f32, // percentage (decimal) relative to contract tick size
}

impl BookState {
    pub fn update_from_orderbook<T>(mut self, orderbook_update: &T) -> Self {
        // self.timestamp = Some(orderbook_update.timestamp);
        self
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Side {
    Buy,
    Sell,
}
impl AsRef<str> for Side {
    fn as_ref(&self) -> &str {
        match self {
            Side::Buy => "buy",
            Side::Sell => "sell",
        }
    }
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Side::Buy => write!(f, "{}", "buy"),
            Side::Sell => write!(f, "{}", "Sell"),
        }
    }
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
    pub aggegate_id: i64,
    #[serde(rename = "p")]
    #[serde(deserialize_with = "DeserializeExchangeStreams::deserialize_string_to_f64")]
    pub price: f64,
    #[serde(rename = "q")]
    #[serde(deserialize_with = "DeserializeExchangeStreams::deserialize_string_to_f64")]
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

impl fmt::Display for BinanceTrades {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Event Type: {}, Event Time: {}, Symbol: {}, Aggregate ID: {}, Price: {}, Quantity: {}, First Trade ID: {}, Last Trade ID: {}, Trade Timestamp: {}",
            self.event_type,
            self.event_time,
            self.symbol,
            self.aggegate_id,
            self.price,
            self.quantity,
            self.first_trade_id,
            self.last_trade_id,
            self.trade_timestamp
        )
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinancePartialBook {
    #[serde(rename = "e")]
    pub depth_update: String,
    #[serde(rename = "E")]
    pub event_timestamp: i64,
    #[serde(rename = "T")]
    pub timestamp: i64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "U")]
    pub first_update_id: i64,
    #[serde(rename = "u")]
    pub final_update_id: i64,
    #[serde(rename = "pu")]
    pub final_update_id_last_stream: i64,
    ///array where index 0: price level, 1: quantity
    #[serde(rename = "b")]
    // #[serde(deserialize_with = "deserialize_string_array_to_f64_array")]
    pub bids: Vec<[String; 2]>,
    #[serde(rename = "a")]
    // #[serde(deserialize_with = "deserialize_string_array_to_f64_array")]
    pub asks: Vec<[String; 2]>,
}

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

impl ToTakerTrades for BinanceTrades {
    fn to_trades_type(&self) -> Result<TakerTrades> {
        let side = match self.is_buyer_mm {
            true => Side::Sell,
            false => Side::Buy,
        };
        Ok(TakerTrades {
            symbol: self.symbol.clone(),
            side: side,
            price: self.price,  /* .parse::<f64>()? */
            qty: self.quantity, /* .parse::<f64>()? */
            local_ids: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            exch_id: self.last_trade_id,
            transaction_timestamp: self.trade_timestamp,
        })
    }
}

impl ToBookModels for BinancePartialBook {
    fn to_bookstate(&self) -> Result<BookState> {
        todo!();
    }

    fn to_book_model(&self) -> Result<BookModel> {
        Ok(BookModel {
            symbol: self.symbol.clone(),
            bids: convert_quotes(self.bids.clone()),
            asks: convert_quotes(self.asks.clone()),
            timestamp: self.timestamp,
        })
    }
}

pub struct DeserializeExchangeStreams;

impl DeserializeExchangeStreams {
    fn deserialize_string_to_f32<'de, D>(deserializer: D) -> Result<f32, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringToF32Visitor;

        impl<'de> Visitor<'de> for StringToF32Visitor {
            type Value = f32;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("string that can be parsed into an f64")
            }

            fn visit_str<E>(self, value: &str) -> Result<f32, E>
            where
                E: de::Error,
            {
                value.parse::<f32>().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(StringToF32Visitor)
    }

    fn deserialize_string_to_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringToF64Visitor;

        impl<'de> Visitor<'de> for StringToF64Visitor {
            type Value = f64;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("string that can be parsed into an f64")
            }

            fn visit_str<E>(self, value: &str) -> Result<f64, E>
            where
                E: de::Error,
            {
                value.parse::<f64>().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(StringToF64Visitor)
    }
    fn deserialize_string_array_to_f64_array<'de, D>(
        deserializer: D,
    ) -> Result<Vec<[f64; 2]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringArrayToF64ArrayVisitor;

        impl<'de> Visitor<'de> for StringArrayToF64ArrayVisitor {
            type Value = Vec<[f64; 2]>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(
                    "an array of string arrays, each with two elements that can be parsed into f64",
                )
            }

            fn visit_seq<S>(self, mut seq: S) -> Result<Vec<[f64; 2]>, S::Error>
            where
                S: SeqAccess<'de>,
            {
                let mut result = Vec::new();

                while let Some(arr) = seq.next_element::<[String; 2]>()? {
                    let converted_arr = [
                        arr[0].parse::<f64>().map_err(de::Error::custom)?,
                        arr[1].parse::<f64>().map_err(de::Error::custom)?,
                    ];
                    result.push(converted_arr);
                }

                Ok(result)
            }
        }

        deserializer.deserialize_seq(StringArrayToF64ArrayVisitor)
    }
}
pub trait ToDbModel {
    type DbModel: Insertable<binancetrades::table>;

    fn to_db_model(&self) -> Self::DbModel;
}
pub trait ToDBBookModel {
    type DbModel: Insertable<binancepartialbook::table>;
    fn to_db_model(&self) -> Self::DbModel;
}
impl ToDbModel for BinanceTradesNewModel {
    type DbModel = BinanceTradesNewModel;

    fn to_db_model(&self) -> Self::DbModel {
        BinanceTradesNewModel {
            event_type: Some(self.event_type.clone().unwrap()),
            event_time: Some(self.event_time).unwrap(),
            symbol: Some(self.symbol.clone()).unwrap(),
            aggegate_id: Some(self.aggegate_id).unwrap(),
            price: Some(self.price).unwrap(),
            quantity: Some(self.quantity).unwrap(),
            first_trade_id: Some(self.first_trade_id).unwrap(),
            last_trade_id: Some(self.last_trade_id).unwrap(),
            trade_timestamp: Some(self.trade_timestamp).unwrap(),
            is_buyer_mm: Some(self.is_buyer_mm).unwrap(),
        }
    }
}
impl ToTakerTrades for BinanceTradesNewModel {
    fn to_trades_type(&self) -> Result<TakerTrades> {
        let side = match self.is_buyer_mm.unwrap() {
            true => Side::Sell,
            false => Side::Buy,
        };
        Ok(TakerTrades {
            symbol: self.symbol.clone().unwrap().to_string(),
            side: side,
            price: self.price.unwrap(),
            qty: self.quantity.unwrap(),
            local_ids: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            exch_id: self.last_trade_id.unwrap(),
            transaction_timestamp: self.trade_timestamp.unwrap(),
        })
    }
}
impl ToDbModel for BinanceTrades {
    type DbModel = BinanceTradesNewModel;

    fn to_db_model(&self) -> Self::DbModel {
        BinanceTradesNewModel {
            event_type: Some(self.event_type.clone()),
            event_time: Some(self.event_time),
            symbol: Some(self.symbol.clone()),
            aggegate_id: Some(self.aggegate_id),
            price: Some(self.price),
            quantity: Some(self.quantity),
            first_trade_id: Some(self.first_trade_id),
            last_trade_id: Some(self.last_trade_id),
            trade_timestamp: Some(self.trade_timestamp),
            is_buyer_mm: Some(self.is_buyer_mm),
        }
    }
}
impl ToBookModels for BinancePartialBookModelInsertable {
    fn to_bookstate(&self) -> Result<BookState> {
        todo!();
    }

    fn to_book_model(&self) -> Result<BookModel> {
        todo!();
    }
}
impl ToDBBookModel for BinancePartialBook {
    type DbModel = BinancePartialBookModelInsertable;

    fn to_db_model(&self) -> Self::DbModel {
        BinancePartialBookModelInsertable {
            depth_update: Some(self.depth_update.clone()),
            event_timestamp: Some(self.event_timestamp),
            timestamp: Some(self.timestamp),
            symbol: Some(self.symbol.clone()),
            first_update_id: Some(self.first_update_id),
            final_update_id: Some(self.final_update_id),
            final_update_id_last_stream: Some(self.final_update_id_last_stream),
            bids: serde_json::to_value(&self.bids).ok(),
            asks: serde_json::to_value(&self.asks).ok(),
        }
    }
}
impl ToDBBookModel for BinancePartialBookModelInsertable {
    type DbModel = BinancePartialBookModelInsertable;

    fn to_db_model(&self) -> Self::DbModel {
        BinancePartialBookModelInsertable {
            depth_update: self.depth_update.clone(),
            event_timestamp: self.event_timestamp,
            timestamp: self.timestamp,
            symbol: self.symbol.clone(),
            first_update_id: self.first_update_id,
            final_update_id: self.final_update_id,
            final_update_id_last_stream: self.final_update_id_last_stream,
            bids: serde_json::to_value(&self.bids).ok(),
            asks: serde_json::to_value(&self.asks).ok(),
        }
    }
}
