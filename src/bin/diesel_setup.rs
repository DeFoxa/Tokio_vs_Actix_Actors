#![allow(unused)]
use actix::Actor;
use anyhow::Result;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use futures_util::StreamExt;
use lib::concurrency_setup::actix_actor_model::*;
use lib::{
    client::{ws::*, ws_types::*},
    types::*,
    utils::*,
};
use serde_json::Value;
use tokio_tungstenite::tungstenite::Message;

use std::env;

pub const MAINNET: &str = "wss://fstream.binance.com";

#[actix_rt::main]
async fn main() -> Result<()> {
    book_data_to_db().await?;
    Ok(())
}
fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL not found, must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("error connecting to DB: {}", database_url))
}

async fn book_data_to_db() -> Result<()> {
    let pool = create_db_pool();
    let db_actor = TradeStreamDBActor { pool: pool.clone() }.start();
    let book_db_actor = BookModelDbActor { pool }.start();

    let (mut ws_state, Response) = Client::connect_combined_async(
        MAINNET,
        vec![
            &StreamNameGenerator::combined_stream_partial_book("ethusdt", "10").await,
            &StreamNameGenerator::combined_stream_trades_by_symbol("ethusdt").await,
        ],
    )
    .await?;

    let (write, read) = ws_state.socket.split();
    read.for_each(|message| async {
        match message {
            Ok(Message::Text(text)) => {
                let value: Value = serde_json::from_str(&text).expect("some error 1");
                let other_event_type = value.get("e").and_then(Value::as_str);
                match other_event_type {
                    Some("aggTrade") => {
                        let trades = serde_json::from_value::<BinanceTrades>(value.clone())
                            .expect("error deserializing to binancetrades");

                        db_actor.try_send(TradeStreamDBMessage {
                            data: trades.to_db_model(),
                        });
                        // println!("{:?}", &trades);
                    }
                    Some("depthUpdate") => {
                        let book = serde_json::from_value::<BinancePartialBook>(value.clone())
                            .expect("error deserializing to PartialBook");
                        book_db_actor.try_send(BookModelDbMessage {
                            data: book.to_db_model(),
                        });
                        // println!("{:?}", &book);
                    }
                    _ => {
                        eprintln!("Error matching deserialized fields, no aggTrade or depthUpdate");
                    }
                }
            }
            _ => (),
        }
    })
    .await;
    Ok(())
}
