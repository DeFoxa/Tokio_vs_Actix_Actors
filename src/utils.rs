use crate::types::*;
use anyhow::Result;
use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, Pool},
};
use dotenvy::dotenv;
use serde::{
    de::{self, Deserializer as deser, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::Value;
use std::env;
use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub fn create_db_pool() -> Pool<ConnectionManager<PgConnection>> {
    dotenv().ok();
    let url = env::var("DATABASE_URL").expect("DATAASE url must be set");
    let manager = ConnectionManager::<PgConnection>::new(url);
    Pool::builder()
        .build(manager)
        .expect("failed to create db pool")
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

pub fn convert_quotes<'a>(data: Vec<[String; 2]>) -> Vec<Quotes> {
    data.into_iter()
        .filter_map(|quote| {
            let levels = quote[0].parse::<f64>().ok()?;
            let qtys = quote[1].parse::<f64>().ok()?;
            Some(Quotes {
                levels: levels,
                qtys: qtys,
                count: None,
            })
        })
        .collect()
}

/// Binance Structs deserialize directly from stream message, wrap -> input data to Automated::BookState for modeling
/// --------------------------------------------------------------------------------------------------------------------
/// drop any event where u < last_update_id in model
/// first processed event should have U <= local_model_update_id && u >= local_model_update_id
/// where local_model_update_id = id_number for last model update (i.e. the number associated with
/// model id from last stream update)
/// --------------------------------------------------------------------------------------------------------------------

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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceAllBook {
    pub e: String,
    #[serde(rename = "u")]
    pub updated_id: i64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "b")]
    pub bid_level: String,
    #[serde(rename = "B")]
    pub bid_qty: String,
    #[serde(rename = "a")]
    pub ask_level: String,
    #[serde(rename = "A")]
    pub ask_qty: String,
    //
    #[serde(rename = "T")]
    pub timestamp: i64,
    #[serde(rename = "E")]
    pub event_timestamp: i64,
}

/// SINGLE TICKER BBO
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceSingleBookTicker {
    #[serde(rename = "e")]
    pub book_ticker: String,
    #[serde(rename = "u")]
    pub update_id: i64,
    #[serde(rename = "E")]
    pub event_time: i64,
    #[serde(rename = "T")]
    pub transaction_time: i64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "b")]
    pub bid_level: String,
    #[serde(rename = "B")]
    pub bid_qty: String,
    #[serde(rename = "a")]
    pub ask_level: String,
    #[serde(rename = "A")]
    pub ask_qty: String,
}
