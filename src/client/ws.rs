use crate::client::ws_types::*;
use chrono::*;
use futures_util::SinkExt;
use once_cell::sync::Lazy;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client as rest_client;
use serde_derive::{Deserialize, Serialize};
use std::env;

/// WSS URL
pub const MAINNET: &str = "wss://fstream.binance.com";
pub const AUTH_MAINNET: &str = "wss://fstream-auth.binance.com";
pub const TESTNET: &str = "wss://stream.binancefuture.com";

/// REST full url for listen_key generation, renew, delete
pub const LISTENKEY: &str = "https://fapi.binance.com/fapi/v1/listenKey";
pub const TESTNET_LISTENKEY: &str = "https://testnet.binancefuture.com/fapi/v1/listenKey";

static API_KEY: Lazy<String> = Lazy::new(|| {
    env::var("BN_TESTNET_API_KEY").expect("Missing environment variable BINANCE_API_KEY")
});

static API_SECRET: Lazy<String> = Lazy::new(|| {
    env::var("BN_TESTNET_API_SECRET").expect("Missing environment variable BINANCE_API_SECRET!!!")
});

///
/// MARKET DATA
///

/// Trades
#[derive(Debug)]
pub struct TradeStream {
    symbol: String,
}
impl TradeStream {
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_lowercase(),
        }
    }
}
impl From<TradeStream> for Stream {
    fn from(stream: TradeStream) -> Stream {
        Stream::new(&format!("{}@aggTrade", stream.symbol))
    }
}
/// BOOK
pub struct BookTickerStream {
    symbol: Option<String>,
}

impl BookTickerStream {
    pub fn all_symbols() -> Self {
        Self { symbol: None }.into()
    }

    pub fn from_symbol(symbol: &str) -> Self {
        Self {
            symbol: Some(symbol.to_lowercase()),
        }
    }
}

impl From<BookTickerStream> for Stream {
    // Returns stream name as `<symbol>@bookTicker` or `!bookTicker@arr`
    fn from(stream: BookTickerStream) -> Stream {
        if let Some(symbol) = stream.symbol {
            Stream::new(&format!("{}@bookTicker", symbol))
        } else {
            Stream::new("!bookTicker")
        }
    }
}
/// MarkPrice
#[derive(Debug)]
pub struct MarkPriceStream {
    symbol: Option<String>,
}
impl MarkPriceStream {
    pub fn all_symbols() -> Self {
        Self { symbol: None }
    }
    pub fn from_symbol(symbol: &str) -> Self {
        Self {
            symbol: Some(symbol.to_lowercase()),
        }
    }
}
impl From<MarkPriceStream> for Stream {
    fn from(stream: MarkPriceStream) -> Stream {
        if let Some(symbol) = stream.symbol {
            Stream::new(&format!("{}@markPrice", symbol))
        } else {
            Stream::new("!markPrice@arr@1s")
        }
    }
}
///KLINE
#[derive(Debug)]
pub struct KlineStream {
    symbol: String,
    interval: String,
}
impl KlineStream {
    pub fn from_symbol(symbol: &str, _interval: String) -> Self {
        Self {
            symbol: symbol.to_lowercase(),
            interval: _interval,
        }
    }
}
impl From<KlineStream> for Stream {
    fn from(stream: KlineStream) -> Stream {
        Stream::new(&format!("{}@kline_{}", stream.symbol, stream.interval))
    }
}
///MiniTicker
#[derive(Debug)]
pub struct MiniTickerStream {
    symbol: Option<String>,
}
impl MiniTickerStream {
    pub fn all_symbols() -> Self {
        Self { symbol: None }
    }
    pub fn from_symbol(symbol: &str) -> Self {
        Self {
            symbol: Some(symbol.to_lowercase()),
        }
    }
}
impl From<MiniTickerStream> for Stream {
    fn from(stream: MiniTickerStream) -> Stream {
        if let Some(symbol) = stream.symbol {
            Stream::new(&format!("{}@miniTicker", symbol))
        } else {
            Stream::new("!miniTicker@arr")
        }
    }
}

///
/// USER DATA STREAMS
///

pub struct UserDataStream {
    pub listen_key: String,
}

impl UserDataStream {
    pub fn new(listen_key: &str) -> Self {
        Self {
            listen_key: listen_key.to_string(),
        }
    }
    pub fn with_req_params(stream: UserDataStream, request_param: &str) -> Stream {
        Stream::new(&format!("{}@{}", stream.listen_key, request_param))
    }
}
impl From<UserDataStream> for Stream {
    fn from(stream: UserDataStream) -> Stream {
        Stream::new(&stream.listen_key)
    }
}
///
/// CONNECTING WITH STREAM NAME
/// NOTE: methods will implement the full url to MAINNET WS passed to connect_with_stream_name method on Client
///

#[derive(Debug)]
pub struct StreamNameGenerator;

impl StreamNameGenerator {
    /// Trade Stream for ticker
    pub async fn trades_by_symbol(symbol: &str) -> String {
        format!("{}/ws/{}@aggTrade", MAINNET, &symbol)
    }

    pub async fn combined_stream_trades_by_symbol(symbol: &str) -> String {
        format!("{}@aggTrade", &symbol)
    }
    // pub async fn book_depth(symbol: &str, update_speed: &str) {
    //     format!("{}@depth@100ms")
    // }

    /// Ticker KLINE for some timeframe
    pub async fn kline(symbol: &str, timeframe: &str) -> String {
        format!("{}/ws/{}@kline_{}", &MAINNET, &symbol, &timeframe)
    }

    pub async fn combined_stream_kline(symbol: &str, timeframe: &str) -> String {
        format!("{}@kline_{}", &symbol, &timeframe)
    }
    /// L2 Book for individual ticker - with price level specifier - level options: 5, 10, 20
    pub async fn partial_book(symbol: &str, depth_levels: &str) -> String {
        format!("{}/ws/{}@depth{}@100ms", &MAINNET, &symbol, &depth_levels)
    }

    pub async fn combined_stream_partial_book(symbol: &str, depth_levels: &str) -> String {
        format!("{}@depth{}@100ms", &symbol, &depth_levels)
    }

    /// AllBook streams BBO for every ticker
    pub async fn all_book() -> String {
        format!("{}/ws/{}", MAINNET, "!bookTicker")
    }
    /// individual symbol Book ticker streams symbol BBO
    pub async fn individual_symbol_book_ticker(symbol: &str) -> String {
        format!("{}/ws/{}@{}", MAINNET, symbol, "bookTicker")
    }
}

///
/// LISTEN KEY: create, renew, delete
/// TODO: Rest request code can be cleaned up and optimized, written for quick impl/testing
///

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListenKey {
    pub listen_key: String,
}

pub async fn new_listen_key() -> Result<ListenKey, anyhow::Error> {
    let rest_client = rest_client::new();
    let mut map: HeaderMap = Default::default();
    map.insert(
        "X-MBX-APIKEY",
        HeaderValue::from_str(&API_KEY.to_string()).unwrap(),
    );
    let response = rest_client
        .post(TESTNET_LISTENKEY)
        .headers(map)
        .send()
        .await?
        .json::<ListenKey>()
        .await?;
    Ok(response)
}
pub async fn renew_listen_key(listen_key: &str) -> Result<String, anyhow::Error> {
    let rest_client = rest_client::new();
    let mut map: HeaderMap = Default::default();
    map.insert(
        "X-MBX-APIKEY",
        HeaderValue::from_str(&API_KEY.to_string()).unwrap(),
    );

    let url = format!(
        "{}?{}={}&{}={}",
        TESTNET_LISTENKEY,
        "listenKey",
        listen_key,
        "timestamp",
        generate_timestamp().await
    );
    let response = rest_client
        .put(url)
        .headers(map)
        .send()
        .await?
        .text()
        .await?;
    Ok(response)
}
pub async fn delete_listen_key(listen_key: &str) -> Result<String, anyhow::Error> {
    let rest_client = rest_client::new();
    let mut map: HeaderMap = Default::default();
    map.insert(
        "X-MBX-APIKEY",
        HeaderValue::from_str(&API_KEY.to_string()).unwrap(),
    );
    let url = format!(
        "{}?{}={}&{}={}",
        TESTNET_LISTENKEY,
        "listenKey",
        listen_key,
        "timestamp",
        generate_timestamp().await
    );
    let response = rest_client
        .delete(url)
        .headers(map)
        .send()
        .await?
        .text()
        .await?;
    Ok(response)
}

async fn generate_timestamp() -> String {
    let timestamp = Utc::now().timestamp_millis().to_string();
    timestamp
}
