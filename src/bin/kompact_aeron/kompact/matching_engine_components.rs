use crate::types::*;
use anyhow::Result;
// use futures::stream::StreamExt;
use futures_util::{Stream, StreamExt};
use kompact::prelude::*;
use lib::{
    client::{ws::*, ws_types::*},
    types::*,
};
use serde_json::Value;
use tokio_tungstenite::tungstenite::Message;

pub const BINANCE: &str = "wss://fstream.binance.com";

#[derive(Debug, Clone)]
pub enum DeserializedData {
    TakerTrades(TakerTrades),
    BookModel(BookModel),
}
#[derive(ComponentDefinition)]
pub struct ServerClient<T: 'static + Send> {
    ctx: ComponentContext<Self>,
    client: ClientTypes<T>,
    trades_port: ProvidedPort<TradesPort>,
    ob_port: ProvidedPort<ObPort>,
}
//TODO: change indication types to DeserializedData wrapped around deserialized stream types
struct ObPort;
impl Port for ObPort {
    type Indication = BookModel;
    type Request = Never;
}

struct TradesPort;

impl Port for TradesPort {
    type Indication = TakerTrades;
    type Request = Never;
}

impl<T: 'static + Send> Provide<TradesPort> for ServerClient<T> {
    fn handle(&mut self, _: Never) -> Handled {
        Handled::Ok
    }
}

impl<T: 'static + Send> Provide<ObPort> for ServerClient<T> {
    fn handle(&mut self, _: Never) -> Handled {
        Handled::Ok
    }
}

impl<T: 'static + Send> ServerClient<T> {
    fn new(server_type: ClientTypes<T>) -> ServerClient<T> {
        ServerClient {
            ctx: ComponentContext::uninitialised(),
            client: server_type,
            trades_port: ProvidedPort::uninitialised(),
            ob_port: ProvidedPort::uninitialised(),
        }
    }
    //NOTE: connect_combined_async() takes streams as vec<&str>, ideally I'd like to pass
    //Vec<StreaMNameGenerator> to ws_combined_combined_stream, TODO: write a method on
    //StreamNameGenerator to implement this.
    async fn ws_combined_stream_connect(&mut self, url: &str, streams: Vec<&str>) -> Result<()> {
        let (mut ws_state, Response) = Client::connect_combined_async(url, streams).await?;
        let (write, read) = ws_state.socket.split();

        let route_data = |data: DeserializedData| {
            match data {
                DeserializedData::TakerTrades(trade_data) => {
                    self.trades_port.trigger(trade_data);
                    // todo!();
                }
                DeserializedData::BookModel(ob_data) => {
                    self.ob_port.trigger(ob_data);
                    // todo!();
                }
            }
        };
        //
        // read.for_each(|message| async {
        //     match message {
        //         Ok(Message::Text(text)) => {
        //             let value: Value = serde_json::from_str(&text).expect(
        //                 "value error from stream message serde_json::from_str deserializer",
        //             );
        //             let event = value.get("e").and_then(Value::as_str);
        //             match event {
        //                 Some("aggTrade") => {
        //                     let trades = serde_json::from_value::<BinanceTrades>(value.clone())
        //                         .expect("error deserializing to binancetrades");
        //                     let data = DeserializedData::TakerTrades(
        //                         trades
        //                             .to_trades_type()
        //                             .expect("error converting to TakerTrades"),
        //                     );
        //                     route_data(data);
        //                 }
        //                 Some("depthUpdate") => {
        //                     let book = serde_json::from_value::<BinancePartialBook>(value.clone())
        //                         .expect("error deserializing to PartialBook");
        //                     book.to_book_model();
        //                     // println!("{:?}", &book);
        //                 }
        //                 _ => {
        //                     eprintln!(
        //                         "Error matching deserialized fields, no aggTrade or depthUpdate"
        //                     );
        //                 }
        //             }
        //         }
        //         _ => (),
        //     }
        // });
        // .await;
        Ok(())
    }

    fn ws_single_connect(&self, stream: StreamNameGenerator) {}

    fn route_deserialized_data(&mut self, data: DeserializedData) {
        match data {
            DeserializedData::TakerTrades(trade_data) => {
                self.trades_port.trigger(trade_data);
                // todo!();
            }
            DeserializedData::BookModel(ob_data) => {
                self.ob_port.trigger(ob_data);
                // todo!();
            }
        }
    }
}
// ignore_lifecycle!(ServerClient);
ignore_requests!(TradesPort, ObPort);

impl<T: 'static + Send> ComponentLifecycle for ServerClient<T> {
    fn on_start(&mut self) -> Handled {
        Handled::block_on(self, move |mut async_self| async move {
            todo!();
        });
        //TODO: Fill these out
        info!(self.ctx.log(), "server client start event");
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
impl<T: 'static + Send> Actor for ServerClient<T> {
    type Message = ();

    fn receive_local(&mut self, _msg: Self::Message) -> Handled {
        Handled::Ok
    }

    fn receive_network(&mut self, _msg: NetMessage) -> Handled {
        unimplemented!("ignoring networking");
    }
}

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
// impl ComponentLifecycle for Sequencer {
//
// }
