use crate::kompact_components::client_components::*;
use crate::{server_client::*, types::*};
use anyhow::Result;
use tokio::join;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
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
pub struct DataNormalizer {
    ctx: ComponentContext<Self>,
    receiver: mpsc::Receiver<StreamMessage>,
}

impl DataNormalizer {
    pub fn new(recv: mpsc::Receiver<StreamMessage>) -> Self {
        Self {
            ctx: ComponentContext::uninitialised(),
            receiver: recv,
        }
    }
}

impl ComponentLifecycle for DataNormalizer {
    fn on_start(&mut self) -> Handled {
        info!(self.ctx.log(), "data normalizer start-up event");
        Handled::Ok
    }
}

impl Actor for DataNormalizer {
    type Message = StreamMessage;

    fn receive_local(&mut self, msg: Self::Message) -> Handled {
        match msg {
            StreamMessage::BookMessage(data) => println!("received book_message: {:?}", data),
            StreamMessage::TradeMessage(data) => println!("received trade message: {:?}", data),
        }
        Handled::Ok
    }
    fn receive_network(&mut self, _msg: NetMessage) -> Handled {
        unimplemented!("Data Normalizer does not handle network messages");
    }
}

// #[derive(ComponentDefinition)]
// pub struct ServerClient {
//     ctx: ComponentContext<Self>,
//     client: ClientTypes,
//     // client_components: Option<Vec<String>>,
//     normalizer_ref_tmp: Option<ActorRefStrong<StreamMessage>>,
// socket: Option<WebSocketState<MaybeTlsStream<TcpStream>>>,
// trades_port: ProvidedPort<TradesPort>,
// ob_port: ProvidedPort<ObPort>,
// }
// impl ServerClient {
//     pub fn new(client_type: ClientTypes) -> Self {
//         Self {
//             ctx: ComponentContext::uninitialised(),
//             client: client_type,
//             // client_components: None,
//             normalizer_ref_tmp: None,
//         }
//     }
// }
// //TODO: considering rewriting on start, to call a method on server_client that initializes the
// //client connection, instead of directly from on_start. on_start initializes the connection, but
// //message handling is
// impl ComponentLifecycle for ServerClient {
//     fn on_start(&mut self) -> Handled {
//         // info!(
//         //     self.ctx.log(),
//         //     "ClientComponents: {:?}", self.client_components
//         // );
//         info!(self.ctx.log(), "server client start event");
//         //TODO: starts our server_client component based on inst input -> sends messages to Deserializer
//
//         match &self.client {
//             ClientTypes::Websocket => {
//                 let system_clone = self.ctx.system();
//
//                 self.spawn_local(move |mut async_self| async move {
//                     println!("test");
//                     info!(
//                         async_self.ctx.log(),
//                         "!!!!!!!!!!!!!!!!!!!!!**************************!!!!!!!!!!!!!!!!",
//                     );
//
//                     let stream_names = vec![
//                         &StreamNameGenerator::combined_stream_partial_book("ethusdt", "10").await,
//                         &StreamNameGenerator::combined_stream_trades_by_symbol("ethusdt").await,
//                     ]
//                     .into_iter()
//                     .map(|s| s.to_string())
//                     .collect::<Vec<_>>();
//
//                     // async_self.client_components = Some(stream_names);
//                     let normalizer_actor = system_clone.create(|| DataNormalizer::new());
//                     system_clone.start(&normalizer_actor);
//
//                     let normalizer_ref = normalizer_actor
//                         .actor_ref()
//                         .hold()
//                         .expect("normalizer_ref not set");
//
//                     info!(async_self.ctx.log(), "ws_component creation");
//                     // let ws_component = async_self.ctx.system().create(|| {
//                     //     WebSocketComponent::<StreamMessage>::new(
//                     //         // self.client_components.clone().expect("Error: None returned on self.client_components in WSComponent instantiator"),
//                     //         stream_names.clone(),
//                     //         None,
//                     //         normalizer_ref,
//                     //     )
//                     // });
//                     // async_self.ctx.system().start(&ws_component);
//                     // system_clone.start(&ws_component);
//                     Handled::Ok
//                 });
//             }
//             ClientTypes::Rest => {
//                 // generate the rest component
//                 println!("Testing: server_client on_start from Rest match statement")
//             }
//
//             ClientTypes::Rpc => {
//                 //generate the rest component
//                 println!("Testing: server_client on_start from RPC match statement")
//             }
//         }
//         Handled::Ok
//     }
//     fn on_stop(&mut self) -> Handled {
//         //TODO: Fill these out
//         info!(self.ctx.log(), "server client stop event");
//         Handled::Ok
//     }
//     fn on_kill(&mut self) -> Handled {
//         self.on_stop()
//     }
// }
// impl Actor for ServerClient {
//     type Message = ();
//
//     fn receive_local(&mut self, _msg: Self::Message) -> Handled {
//         Handled::Ok
//     }
//
//     fn receive_network(&mut self, _msg: NetMessage) -> Handled {
//         unimplemented!("ignoring networking");
//     }
// }
//
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
