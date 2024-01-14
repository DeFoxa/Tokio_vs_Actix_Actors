use crate::utils::*;
use crate::utils::*;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{
    de::{self, Deserializer as deser, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::collections::HashMap;
use std::fmt;
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

#[derive(Debug, Clone)]
pub struct BookModel<'a> {
    pub symbol: &'a str,
    pub bids: Vec<Quotes>,
    pub asks: Vec<Quotes>,
    pub timestamp: Option<i64>,
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
