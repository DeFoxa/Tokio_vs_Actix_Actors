use anyhow::Result;
use futures_util::{Stream, StreamExt};
use lib::client::ws_types::*;
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
pub enum ClientTypes {
    Websocket, /* (WebSocketState<MaybeTlsStream<TcpStream>>) */
    Rest,
    Rpc,
}

type WsConnectFn =
    dyn Fn(
        &str,
    )
        -> Result<(WebSocketState<MaybeTlsStream<TcpStream>>, Response), tungstenite::Error>;

impl ClientTypes {
    fn get_url(&self, url: &str, endpoint: Option<&str>) {
        match self {
            ClientTypes::Websocket => todo!(),
            ClientTypes::Rest => todo!(),
            ClientTypes::Rpc => todo!(),
        }
    }
    fn connect_websocket(
        &self,
        connect: &WsConnectFn,
        url: &str,
    ) -> Result<(WebSocketState<MaybeTlsStream<TcpStream>>, Response), tungstenite::Error> {
        connect(url)
    }
}
// pub struct WsComponents<T> {
//     socket: WebSocketStream<T>,
//     stream_name: Option<StreamNameGenerator>,
//    }
#[derive(Debug)]
pub enum StreamMessage {
    TradeMessage(String),
    BookMessage(String),
}
