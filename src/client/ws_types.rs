use anyhow::Result;
use futures_util::SinkExt;

use std::fmt;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{handshake::client::Response, protocol::Message, Error},
    MaybeTlsStream, WebSocketStream,
};
use url::Url;

pub const MAINNET: &str = "wss://fstream.binance.com";
pub const AUTH_MAINNET: &str = "wss://fstream-auth.binance.com";
pub const TESTNET: &str = "wss://stream.binancefuture.com"; //"wss://testnet.binance.vision/ws";

pub struct Client;
impl Client {
    pub async fn connect_async(
        url: &str,
    ) -> Result<(WebSocketState<MaybeTlsStream<TcpStream>>, Response), Error> {
        let (socket, response) = connect_async(Url::parse(&url).unwrap()).await?;
        log::info!("connected to {}", url);
        log::debug!("Response http code: {}", response.status());
        log::debug!("rsponse headers:");
        for (ref header, _value) in response.headers() {
            log::debug!("* {}", header);
        }
        Ok((WebSocketState::new(socket), response))
    }

    pub async fn connect_with_stream_name(
        url: &String,
        /* stream_name: &str, */
    ) -> Result<(WebSocketState<MaybeTlsStream<TcpStream>>, Response), Error> {
        // let full_url = format!("{}/ws/{}", url, stream_name);
        let (socket, response) = connect_async(Url::parse(&url).unwrap()).await?;
        log::info!("connected to {}", url);
        log::debug!("Response http code: {}", response.status());
        log::debug!("rsponse headers:");
        for (ref header, _value) in response.headers() {
            log::debug!("* {}", header);
        }
        Ok((WebSocketState::new(socket), response))
    }

    ///
    /// Connect to Multiple Streams: pass vec with stream_names, connect_combined_path will format URL properly
    ///

    pub async fn connect_combined_async(
        url: &str,
        streams: Vec<&str>,
    ) -> Result<(WebSocketState<MaybeTlsStream<TcpStream>>, Response), Error> {
        let combined_path = streams
            .iter()
            .map(|&name| format!("{}", name))
            .collect::<Vec<_>>()
            .join("/");
        let full_url = format!("{}/ws/{}", url, combined_path);

        let (socket, response) = connect_async(Url::parse(&full_url).unwrap()).await?;
        log::info!("connected to {}", url);
        log::debug!("response http code: {}", response.status());
        log::debug!("response headers");
        for (ref header, _value) in response.headers() {
            log::debug!("* {}", header);
        }
        Ok((WebSocketState::new(socket), response))
    }
    ///
    /// Connect with listen_key for account, trade, balance, etc updates
    ///
    pub async fn connect_with_listenkey(
        url: &str,
        listen_key: &str,
    ) -> Result<(WebSocketState<MaybeTlsStream<TcpStream>>, Response), Error> {
        let full_url = format!("{}/ws/{}", url, listen_key);
        let (socket, response) = connect_async(Url::parse(&full_url).unwrap()).await?;
        log::info!("connected to {}", url);
        log::debug!("Response http code: {}", response.status());
        log::debug!("rsponse headers:");
        for (ref header, _value) in response.headers() {
            log::debug!("* {}", header);
        }
        Ok((WebSocketState::new(socket), response))
    }
}
pub struct WebSocketState<T> {
    pub socket: WebSocketStream<T>,
    pub id: u64,
}

impl<T: AsyncRead + AsyncWrite + Unpin> WebSocketState<T> {
    pub fn new(socket: WebSocketStream<T>) -> Self {
        Self { socket, id: 0 }
    }
    pub async fn send(&mut self, method: &str, params: impl IntoIterator<Item = &str>) -> u64 {
        let mut params_str: String = params
            .into_iter()
            .map(|param| format!("\"{}\"", param))
            .collect::<Vec<String>>()
            .join(",");
        if !params_str.is_empty() {
            params_str = format!("\"params\": [{params}],", params = params_str)
        };
        let id = self.id.clone();
        self.id += 1;

        let s = format!(
            "{{\"method\":\"{method}\",{params}\"id\":{id}}}",
            method = method,
            params = params_str,
            id = id
        );
        let message = Message::Text(s);
        println!("message {}", message);
        log::debug!("Send {}", message);
        self.socket.send(message).await.unwrap();
        id
    }

    pub async fn subscribe(&mut self, streams: impl IntoIterator<Item = &Stream>) -> u64 {
        self.send("SUBSCRIBE", streams.into_iter().map(|s| s.as_str()))
            .await
    }

    pub async fn unsubscribe(&mut self, streams: impl IntoIterator<Item = &Stream>) -> u64 {
        self.send("UNSUBSCRIBE", streams.into_iter().map(|s| s.as_str()))
            .await
    }

    pub async fn subscriptions(&mut self) -> u64 {
        self.send("LIST_SUBSCRIPTIONS", vec![]).await
    }

    pub async fn close(&mut self) -> Result<(), Error> {
        self.socket.close(None).await
    }
}

impl<T> From<WebSocketState<T>> for WebSocketStream<T> {
    fn from(conn: WebSocketState<T>) -> WebSocketStream<T> {
        conn.socket
    }
}
impl<T> AsMut<WebSocketStream<T>> for WebSocketState<T> {
    fn as_mut(&mut self) -> &mut WebSocketStream<T> {
        &mut self.socket
    }
}

pub struct Stream {
    stream_name: String,
}

impl Stream {
    pub fn new(stream_name: &str) -> Self {
        Self {
            stream_name: stream_name.to_owned(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.stream_name
    }
}

impl fmt::Display for Stream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.stream_name)
    }
}
