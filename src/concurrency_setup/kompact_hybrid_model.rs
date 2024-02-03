use kompact::prelude::*;
// use lib::models::BinanceTradesNewModel;
// use lib::{
//     client::{ws::*, ws_types::*},
//     types::*,
//     utils::*,
// };

#[derive(ComponentDefinition, Actor)]
pub struct TradeStreamComponent {
    ctx: ComponentContext<Self>,
}

impl TradeStreamComponent {
    pub fn new() -> Self {
        TradeStreamComponent {
            ctx: ComponentContext::uninitialised(),
        }
    }
}
impl ComponentLifecycle for TradeStreamComponent {
    fn on_start(&mut self) -> Handled {
        info!(self.log(), "Trade Stream Component Start-up");
        self.ctx.system().shutdown_async();
        Handled::Ok
    }
}
