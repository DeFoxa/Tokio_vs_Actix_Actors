use anyhow::Result;
use futures_util::{Stream, StreamExt};
use lib::client::ws_types::*;
use lib::{
    client::{ws::*, ws_types::*},
    types::*,
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{handshake::client::Response, protocol::Message, Error},
    MaybeTlsStream, WebSocketStream,
};

use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};

/// TODO: will come back to effecient server_client spawning later, current focus is hybrid model
/// TODO: write enum/methods for kompact manager input -> exchange specific (multi-exchange) stream
/// connection generator. i.e. one api interface to connect to ws variations and return the Stream/Response
// #[derive(Debug, Clone)]

// pub struct WsComponents<T> {
//     socket: WebSocketStream<T>,
//     stream_name: Option<StreamNameGenerator>,
//    }
#[derive(Debug)]
pub enum StreamMessage {
    TradeMessage(BinanceTrades),
    BookMessage(BinancePartialBook),
}
