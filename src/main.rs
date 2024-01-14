#![allow(warnings)]
mod client;
mod concurrency_setup;
mod ob_model;
mod types;
mod utils;
use crate::concurrency_setup::actor_model::*;

use crate::{
    client::{ws::*, ws_types::*},
    types::*,
};
use actix::prelude::*;
use actix_rt::{task::spawn_blocking, Arbiter, System};
use anyhow::Result;
use concurrency_setup::actor_model;
use futures_util::{Stream, StreamExt};
use serde_json::Value;
use std::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

#[actix_rt::main]
async fn main() -> Result<()> {
    let addr = TradeStreamActor.start();
    let (mut ws_state, Response) =
        Client::connect_with_stream_name(&StreamNameGenerator::trades_by_symbol("ethusdt").await)
            .await?;
    let (write, read) = ws_state.socket.split();
    read.for_each(|message| async {
        match message {
            Ok(Message::Text(text)) => {
                let value: Value = serde_json::from_str(&text).expect("some error 1");
                let trades: BinanceTrades = serde_json::from_value(value).expect("some error 2");
                addr.do_send(TradeStreamMessage { data: trades });
            }
            _ => (),
        }
    })
    .await;
    // system.run().unwrap();
    Ok(())
}
// let _ = System::new();
//    let (tx, rx) = mpsc::channel::<BinanceTrades>();
//    let arbiter = Arbiter::new();
//

// arbiter.spawn_fn(move || {
//     tx.send(trades);
// });
//
