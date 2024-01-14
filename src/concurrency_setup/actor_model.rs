use crate::types::*;
use crate::utils::*;
use actix::prelude::*;
use anyhow::Result;
use std::fmt;
use std::fmt::Display;
use std::sync::Arc;

pub struct TradeStreamMessage<T>
where
    T: ToTakerTrades,
{
    pub data: T,
}

impl<T> Message for TradeStreamMessage<T>
where
    T: ToTakerTrades + 'static,
{
    type Result = Result<TakerTrades>;
}
impl<T> fmt::Display for TradeStreamMessage<T>
where
    T: ToTakerTrades + Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "message: {}", self.data)
    }
}

pub struct TradeStreamActor;

impl Actor for TradeStreamActor {
    type Context = Context<Self>;
}
impl<T: ToTakerTrades + Display + 'static> Handler<TradeStreamMessage<T>> for TradeStreamActor {
    type Result = Result<TakerTrades>;
    fn handle(&mut self, msg: TradeStreamMessage<T>, _ctx: &mut Self::Context) -> Self::Result {
        println!("data {}", msg);
        msg.data.to_trades_type()
    }
}
