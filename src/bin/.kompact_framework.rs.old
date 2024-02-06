#![allow(warnings)]
use anyhow::Result;
use kompact::prelude::*;
use lib::concurrency_setup::kompact_hybrid_model::*;
use lib::models::BinanceTradesNewModel;
use lib::{
    client::{ws::*, ws_types::*},
    types::*,
    utils::*,
};

pub fn main() -> Result<()> {
    let sys = KompactConfig::default().build().expect("whatever");
    let component = sys.create(TradeStreamComponent::new);
    sys.start(&component);
    sys.await_termination();
    Ok(())
}
