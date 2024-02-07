use kompact::prelude::*;
use lib::types::*;

#[derive(Debug, Clone)]
pub enum DeserializedData {
    TakerTrades(TakerTrades),
    BookModel(BookModel),
}
pub struct ServerClient {
    ctx: ComponentContext<Self>,
    trades_port: ProvidedPort<TradesPort>,
    ob_port: ProvidedPort<ObPort>,
}

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
        Handled::OK
    }
}
impl ServerClient {
    fn route_deserialized_data(&self, data: DeserializedData) {
        match data {
            DeserializedData::TakerTrades(trade_data) => {
                self.trades_port.trigger(trade_data);
            }
            DeserializedData::BookModel(ob_data) => {
                self.ob_port.trigger(ob_data);
            }
        }
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
        self.handle_incoming_data(msg);
        Handled::Ok
    }
    fn receive_network(&mut self, msg: NetMessage) -> Handled {
        todo!();
        Handled::Ok
    }
}
ignore_lifecycle!(Sequencer);
// impl ComponentLifecycle for Sequencer {
//
// }
