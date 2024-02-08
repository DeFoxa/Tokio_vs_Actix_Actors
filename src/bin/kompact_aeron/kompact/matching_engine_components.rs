use std::sync::mpsc;

use crate::kompact::client_components::*;
use crate::types::*;
use anyhow::Result;
use tokio::net::TcpStream;
// use futures::stream::StreamExt;
use futures_util::{SinkExt, Stream, StreamExt};
use kompact::prelude::*;
use lib::{
    client::{ws::*, ws_types::*},
    types::*,
};
use serde_json::Value;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

pub const BINANCE: &str = "wss://fstream.binance.com";

#[derive(Debug, Clone)]
pub enum DeserializedData {
    TakerTrades(TakerTrades),
    BookModel(BookModel),
}
#[derive(ComponentDefinition)]
pub struct ServerClient {
    ctx: ComponentContext<Self>,
    client: ClientTypes,
    // socket: Option<WebSocketState<MaybeTlsStream<TcpStream>>>,
    // trades_port: ProvidedPort<TradesPort>,
    // ob_port: ProvidedPort<ObPort>,
}

//TODO: considering rewriting on start, to call a method on server_client that initializes the
//client connection, instead of directly from on_start. on_start initializes the connection, but
//message handling is
impl ComponentLifecycle for ServerClient {
    fn on_start(&mut self) -> Handled {
        info!(self.ctx.log(), "server client start event");
        //TODO: starts our server_client component based on inst input -> sends messages to Deserializer

        match &self.client {
            ClientTypes::Websocket(ws) => {
                // generate the ws component
                todo!();
            }
            ClientTypes::Rest => {
                // generate the rest component
                todo!();
            }
            ClientTypes::Rpc => {
                //generate the rest component
                todo!();
            }
        }
        Handled::Ok
    }
    fn on_stop(&mut self) -> Handled {
        //TODO: Fill these out
        info!(self.ctx.log(), "server client stop event");
        Handled::Ok
    }
    fn on_kill(&mut self) -> Handled {
        self.on_stop()
    }
}
impl Actor for ServerClient {
    type Message = ();

    fn receive_local(&mut self, _msg: Self::Message) -> Handled {
        Handled::Ok
    }

    fn receive_network(&mut self, _msg: NetMessage) -> Handled {
        unimplemented!("ignoring networking");
    }
}

#[derive(Debug, Clone)]
struct ObPort;

impl Port for ObPort {
    type Indication = BookModel;
    type Request = Never;
}

#[derive(Debug, Clone)]
struct TradesPort;

impl Port for TradesPort {
    type Indication = TakerTrades;
    type Request = Never;
}

//TODO: change indication types to DeserializedData wrapped around deserialized stream types
// impl Provide<TradesPort> for ServerClient {
//     fn handle(&mut self, _: Never) -> Handled {
//         Handled::Ok
//     }
// }
//
// impl Provide<ObPort> for ServerClient {
//     fn handle(&mut self, _: Never) -> Handled {
//         Handled::Ok
//     }
// }
//
// impl ServerClient {
//     fn new(server_type: ClientTypes) -> ServerClient {
//         ServerClient {
//             ctx: ComponentContext::uninitialised(),
//             client: server_type,
//             // socket: None,
//             // trades_port: ProvidedPort::uninitialised(),
//             // ob_port: ProvidedPort::uninitialised(),
//         }
//     }
// }
// ignore_lifecycle!(ServerClient);
// ignore_requests!(TradesPort, ObPort);

#[derive(ComponentDefinition)]
pub struct Sequencer {
    ctx: ComponentContext<Self>,
    trades_port: RequiredPort<TradesPort>,
    ob_port: RequiredPort<ObPort>,
}
impl Sequencer {
    fn process_trade_data(&self, trade_data: TakerTrades) {
        todo!();
    }
    fn process_ob_data(&self, ob_data: BookModel) {
        todo!();
    }
}

impl Actor for Sequencer {
    type Message = DeserializedData;

    fn receive_local(&mut self, msg: Self::Message) -> Handled {
        match msg {
            DeserializedData::BookModel(data) => println!("Received Trade {:?}", data),
            DeserializedData::TakerTrades(data) => println!("Received Ob Update {:?}", data),
        }
        Handled::Ok
    }
    fn receive_network(&mut self, msg: NetMessage) -> Handled {
        todo!();
        Handled::Ok
    }
}
ignore_lifecycle!(Sequencer);
impl Require<TradesPort> for Sequencer {
    fn handle(&mut self, event: TakerTrades) -> Handled {
        todo!();
        Handled::Ok
    }
}

impl Require<ObPort> for Sequencer {
    fn handle(&mut self, event: BookModel) -> Handled {
        todo!();
        Handled::Ok
    }
}
