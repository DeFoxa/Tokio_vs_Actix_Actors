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
