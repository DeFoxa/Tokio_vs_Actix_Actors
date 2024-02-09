#![allow(warnings)]
use crate::kompact_components::{client_components::*, matching_engine_components::*};
use crate::types::*;
use anyhow::Result;
use futures_util::{SinkExt, Stream, StreamExt};
use kompact::prelude::*;
use lib::{
    client::{ws::*, ws_types::*},
    types::*,
};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
pub mod kompact_components;
pub mod types;

pub const BINANCE: &str = "wss://fstream.binance.com";

#[tokio::main]
async fn main() {
    // let cfg = KompactConfig::Default();
    // cfg.system_components(DeadLetterBox::new, NetworkConfig::default().build());
    // let system = cfg.build().expect("system");
    let system = KompactConfig::default().build().expect("system");
    let server_client = system.create(|| ServerClient::new(ClientTypes::Websocket));
    system.start(&server_client);
    println!("system running");
    let _ = std::io::stdin().read_line(&mut String::new());
    system.shutdown().expect("kompact system shutdown");
    // let client_comp = system.create(ServerClient::new());
}

async fn ws_listener(sender: mpsc::Sender<StreamMessage>) -> Result<()> {
    let channel = mpsc::channel::<StreamMessage>(100);

    let (mut ws_state, Response) = Client::connect_combined_async(
        BINANCE,
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

                        sender.send(StreamMessage::TradeMessage(trades));
                    }
                    Some("depthUpdate") => {
                        let book = serde_json::from_value::<BinancePartialBook>(value.clone())
                            .expect("error deserializing to PartialBook");
                    }
                    _ => {
                        eprintln!("Error matching deserialized fields, no aggTrade or depthUpdate");
                    }
                }
            }
            _ => (),
        }
    });
    Ok(())
}
