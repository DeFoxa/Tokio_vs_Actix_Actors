use kompact::prelude::*;
use lib::types::*;

#[derive(Debug, Clone)]
pub enum DeserializedData {
    TakerTrades(TakerTrades),
    BookModel(BookModel),
}
#[derive(ComponentDefinition)]
pub struct ServerClient {
    ctx: ComponentContext<Self>,
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

impl Provide<TradesPort> for ServerClient {
    fn handle(&mut self, _: Never) -> Handled {
        Handled::Ok
    }
}

impl Provide<ObPort> for ServerClient {
    fn handle(&mut self, _: Never) -> Handled {
        Handled::Ok
    }
}
impl ServerClient {
    fn new() -> ServerClient {
        ServerClient {
            ctx: ComponentContext::uninitialised(),
            trades_port: ProvidedPort::uninitialised(),
            ob_port: ProvidedPort::uninitialised(),
        }
    }
    fn route_deserialized_data(&self, data: DeserializedData) {
        match data {
            DeserializedData::TakerTrades(trade_data) => {
                // self.trades_port.trigger(trade_data);
                todo!();
            }
            DeserializedData::BookModel(ob_data) => {
                // self.ob_port.trigger(ob_data);
                todo!();
            }
        }
    }
}
// ignore_lifecycle!(ServerClient);
ignore_requests!(TradesPort, ObPort);
impl ComponentLifecycle for ServerClient {
    fn on_start(&mut self) -> Handled {
        // Fill these in and add fn on_pause and on_kill
        Handled::Ok
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
