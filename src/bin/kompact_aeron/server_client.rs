use crate::types::*;
use anyhow::Result;
use futures_util::{SinkExt, Stream, StreamExt};
use kompact::prelude::*;
use lib::{
    client::{ws::*, ws_types::*},
    types::*,
};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::{
    tungstenite::{handshake::client::Response, Message},
    MaybeTlsStream, WebSocketStream,
};

pub const BINANCE: &str = "wss://fstream.binance.com";

///
/// NOTE: currently setup with hardcoded connection to binance ws via connect_combined_async +
/// StreamNameGenerator
///
//TODO: add fields to pass into instantiator for generalized websocket server connections -> remove
//hardcoded connection
pub enum ClientTypes {
    Websocket,
    Rest,
    Rpc,
}

type WsConnectFn =
    dyn Fn(
        &str,
    )
        -> Result<(WebSocketState<MaybeTlsStream<TcpStream>>, Response), tungstenite::Error>;

impl ClientTypes {
    fn connect_websocket(
        &self,
        connect: &WsConnectFn,
        url: &str,
    ) -> Result<(WebSocketState<MaybeTlsStream<TcpStream>>, Response), tungstenite::Error> {
        connect(url)
    }
}

pub struct Websocket {
    client: WebSocketState<MaybeTlsStream<TcpStream>>,
    sender: mpsc::Sender<StreamMessage>,
}

impl Websocket {
    pub async fn new(stream_name: Vec<&str>) -> Result<Self> {
        let (mut ws_state, Response) = Client::connect_combined_async(BINANCE, stream_name).await?;
        println!("{:?}", Response);

        let (mut tx, mut rx) = mpsc::channel::<StreamMessage>(100);
        Ok(Self {
            client: ws_state,
            sender: tx,
        })
    }

    pub async fn ws_listener(self) -> Result<()> {
        let (write, read) = self.client.socket.split();
        read.for_each(|message| async {
            match message {
                Ok(Message::Text(text)) => {
                    let value: Value = serde_json::from_str(&text).expect("some error 1");
                    let event = value.get("e").and_then(Value::as_str);
                    match event {
                        Some("aggTrade") => {
                            let trades = serde_json::from_value::<BinanceTrades>(value.clone())
                                .expect("error deserializing to binancetrades");
                            // self.sender.send(StreamMessage::TradeMessage(trades));
                        }
                        Some("depthUpdate") => {
                            let book = serde_json::from_value::<BinancePartialBook>(value.clone())
                                .expect("error deserializing to PartialBook");
                            // self.sender.send(StreamMessage::BookMessage(book));
                        }
                        _ => {
                            eprintln!(
                                "Error matching deserialized fields, no aggTrade or depthUpdate"
                            );
                        }
                    }
                }
                _ => (),
            }
        })
        .await;
        Ok(())
    }
}
