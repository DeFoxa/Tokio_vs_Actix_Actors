use crate::types::*;
use futures_util::{SinkExt, StreamExt};
use kompact::prelude::*;
use lib::client::{ws::*, ws_types::*};
use std::fmt::Debug;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

pub const BINANCE: &str = "wss://fstream.binance.com";

#[derive(ComponentDefinition)]
pub struct WebSocketComponent<M>
where
    M: 'static + Send + Debug,
{
    ctx: ComponentContext<Self>,
    /// using StreamName with combined_connection method on binance WS server for now, will add Options for other
    /// connection methods later
    stream_names: Vec<String>,
    //Option<string> until multiple exchange ws clients implemented, will default to Binance with
    //const BINANCE hardcode in fn on_start() until then
    exchange: Option<String>,
    data_normalizer: ActorRefStrong<M>,
}

impl<M: Send + Debug> WebSocketComponent<M> {
    pub fn new(
        stream_names: Vec<String>,
        exchange: Option<String>,
        data_normalizer: ActorRefStrong<M>,
    ) -> Self {
        Self {
            ctx: ComponentContext::uninitialised(),
            stream_names,
            exchange: None,
            data_normalizer,
        }
    }
}

impl<M: Send + Debug> ComponentLifecycle for WebSocketComponent<M> {
    fn on_start(&mut self) -> Handled {
        self.spawn_local(move |async_self| async move {
            let (mut client, Response) =
                Client::connect_combined_owned(BINANCE, async_self.stream_names.clone())
                    .await
                    .unwrap();
            if let (mut write, mut read) = client.socket.split() {
                while let Some(message) = read.next().await {
                    /// NOTE: testing place holder
                    println!("{:?}", message);
                }
            }

            Handled::Ok
        });
        Handled::Ok
    }
}
impl<M: Send + Debug> Actor for WebSocketComponent<M> {
    type Message = ();
    fn receive_local(&mut self, _msg: Self::Message) -> Handled {
        Handled::Ok
    }
    fn receive_network(&mut self, _msg: NetMessage) -> Handled {
        unimplemented!("ignoring network for now");
    }
}
