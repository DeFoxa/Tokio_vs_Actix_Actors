#![allow(warnings)]
#[allow(unused_must_use)]
use lib::concurrency_setup::actix_actor_model::*;
use std::{collections::BinaryHeap, env};
use tracing_subscriber::{filter::EnvFilter, fmt, prelude::*, registry::Registry};

use actix::prelude::*;
use anyhow::Result;
use futures_util::StreamExt;
use lib::{
    client::{ws::*, ws_types::*},
    types::*,
};
use serde_json::Value;
use std::time::Instant;
use tokio_tungstenite::tungstenite::Message;

pub const MAINNET: &str = "wss://fstream.binance.com";

#[actix_rt::main]
async fn main() -> Result<()> {
    let file_appender =
        tracing_appender::rolling::minutely(".logs/actix_logs", "concurrency_model_testing.log");

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt().with_writer(non_blocking).init();
    stream_data_to_actix_matching_engine().await?;
    Ok(())
}

async fn stream_data_to_actix_matching_engine() -> Result<()> {
    let matching_engine_addr = MatchingEngineActor { data: 1 }.start();
    let seq_addr = SequencerActor {
        queue: BinaryHeap::new(),
        matching_engine_addr,
        last_ob_update: Instant::now(),
        is_processing_paused: false,
    }
    .start();

    let trade_stream_actor = TradeStreamActor {
        sequencer_addr: seq_addr.clone(),
    }
    .start();
    let book_actor = BookModelStreamActor {
        sequencer_addr: seq_addr,
    }
    .start();

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
                let event = value.get("e").and_then(Value::as_str);
                match event {
                    Some("aggTrade") => {
                        let trades = serde_json::from_value::<BinanceTrades>(value.clone())
                            .expect("error deserializing to binancetrades");

                        trade_stream_actor.do_send(TradeStreamMessage { data: trades });
                        // println!("{:?}", &trades);
                    }
                    Some("depthUpdate") => {
                        let book = serde_json::from_value::<BinancePartialBook>(value.clone())
                            .expect("error deserializing to PartialBook");
                        book_actor.do_send(BookModelStreamMessage { data: book });
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
