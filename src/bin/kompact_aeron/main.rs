#![allow(warnings)]
use crate::kompact_components::{client_components::*, matching_engine_components::*};
use crate::types::*;
use kompact::prelude::*;
pub mod kompact_components;
pub mod types;

fn main() {
    // let cfg = KompactConfig::Default();
    // cfg.system_components(DeadLetterBox::new, NetworkConfig::default().build());
    // let system = cfg.build().expect("system");
    let system = KompactConfig::default().build().expect("system");
    let server_client = system.create(|| ServerClient::new(ClientTypes::Websocket));
    system.start(&server_client);
    println!("system running");
    let _ = std::io::stdin().read_line(&mut String::new());
    system.shutdown().expect("kompact system shutdown");
    // let client_comp = system.create(ServerClient::new());
}
