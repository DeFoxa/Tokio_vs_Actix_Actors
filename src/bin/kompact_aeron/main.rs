#![allow(warnings)]
use crate::kompact_components::{client_components::*, matching_engine_components::*};
use crate::{server_client::*, types::*};
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
pub mod server_client;
pub mod types;

pub const BINANCE: &str = "wss://fstream.binance.com";

#[tokio::main]
async fn main() -> Result<()> {
    let partial_book = StreamNameGenerator::combined_stream_partial_book("ethusdt", "10").await;
    let trades = StreamNameGenerator::combined_stream_trades_by_symbol("ethusdt").await;
    let vec = vec![trades.as_str(), partial_book.as_str()];

    let test = Websocket::new(vec).await?.ws_listener().await?;
    println!("{:?}", test);
    Ok(())
    // let cfg = KompactConfig::Default();
    // cfg.system_components(DeadLetterBox::new, NetworkConfig::default().build());
    // let system = cfg.build().expect("system");
    // let (sender, receiver) = mpsc::channel::<StreamMessage>(100);
    // let system = KompactConfig::default().build().expect("system");
    // let server_client = system.create(|| ServerClient::new(ClientTypes::Websocket));
    // system.start(&server_client);
    // println!("system running");
    // let _ = std::io::stdin().read_line(&mut String::new());
    // system.shutdown().expect("kompact system shutdown");
    // let client_comp = system.create(ServerClient::new());
}
