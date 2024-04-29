#![allow(warnings)]
use anyhow::Result;
use futures_util::StreamExt;
use lib::concurrency_setup::tokio_actor_model::TradeStreamActorHandler;
use lib::concurrency_setup::tokio_actor_model::{
    MockMatchingEngineHandler, OrderBookActorHandler, OrderBookStreamMessage as OBSM,
    SequencerHandler, TradeStreamMessage as TSM,
};
use lib::{
    client::{ws::*, ws_types::*},
    types::*,
};
use serde_json::Value;
use tokio_tungstenite::tungstenite::Message;
// use tracing_flame::FlameLayer;

pub const MAINNET: &str = "wss://fstream.binance.com";

#[tokio::main]
async fn main() -> Result<()> {
    let file_appender = tracing_appender::rolling::minutely(
        ".logs/tokio_model_logs",
        "concurrency_model_testing.log",
    );

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt().with_writer(non_blocking).init();
    stream_data_to_tokio_matching_engine().await?;

    Ok(())
}

async fn stream_data_to_tokio_matching_engine() -> Result<()> {
    let (matching_engine_sender, matching_engine_handle) = MockMatchingEngineHandler::new()?;
    let (sequencer_sender, sequencer_handle, timer_handle) =
        SequencerHandler::new(matching_engine_sender)?;
    let (trade_stream_sender, trade_stream_handle) =
        TradeStreamActorHandler::new(sequencer_sender.clone())?;
    let (order_book_sender, order_book_handle) = OrderBookActorHandler::new(sequencer_sender)?;

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

                        trade_stream_sender.send(TSM { data: trades }).await;
                    }
                    Some("depthUpdate") => {
                        let book = serde_json::from_value::<BinancePartialBook>(value.clone())
                            .expect("error deserializing to PartialBook");
                        order_book_sender.send(OBSM { data: book }).await;
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
    let _ = tokio::join!(
        matching_engine_handle,
        sequencer_handle,
        trade_stream_handle,
        order_book_handle,
        timer_handle,
    );

    Ok(())
}
