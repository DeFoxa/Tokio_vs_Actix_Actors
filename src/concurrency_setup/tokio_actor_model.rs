use crate::types::*;
use crate::utils::*;
use anyhow::Result;
use async_trait::async_trait;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tracing::{event, info, instrument, Level};
use tracing_subscriber::prelude::*;

#[async_trait]
trait Handler<S, T, G> {
    async fn new(sender: mpsc::Sender<S>) -> Self;
    async fn handle_message(&mut self, msg: T) -> Result<()>;
    async fn send(&self, msg: G) -> Result<()>;
    // fn get_receiver(&self) -> &mpsc::Reciever<T>;
}
#[async_trait]
trait Actor<T, G> {
    async fn new(receiver: mpsc::Receiver<T>, sender: mpsc::Sender<G>) -> Self;
}
// TRADE STREAM
impl<T: ToTakerTrades + Send + Sync + 'static> Actor<T, G> for TradeStreamActor<T> {
    async fn new(receiver: mpsc::Receiver<T>, sender: mpsc::Sender<G>) -> Self {
        Self { receiver, sender }
    }
}
#[derive(Debug, Clone)]
pub struct TradeStreamMessage<T>
where
    T: ToTakerTrades + Send + Sync + 'static,
{
    pub data: T,
}

#[derive(Debug)]
pub struct TradeStreamActor<T>
where
    T: ToTakerTrades + Send + Sync + 'static,
{
    pub receiver: mpsc::Receiver<TradeStreamMessage<T>>,
    pub sender: mpsc::Sender<SequencerMessage>,
}

impl<T: ToTakerTrades + Send + Sync + 'static> TradeStreamActor<T> {
    fn new(
        receiver: mpsc::Receiver<TradeStreamMessage<T>>,
        sender: mpsc::Sender<SequencerMessage>,
    ) -> Self {
        TradeStreamActor { receiver, sender }
    }
}

#[derive(Debug, Clone)]
pub struct TradeStreamActorHandler<T>
where
    T: ToTakerTrades + Send + Sync,
{
    sender: mpsc::Sender<SequencerMessage>,
    _marker: PhantomData<T>,
}

#[async_trait]
impl<T: ToTakerTrades + Send + Sync>
    Handler<SequencerMessage, TradeStreamMessage<T>, SequencerMessage>
    for TradeStreamActorHandler<T>
{
    async fn new(sequencer_sender: mpsc::Sender<SequencerMessage>) -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let actor: TradeStreamActor<BinanceTrades> =
            TradeStreamActor::new(receiver, sequencer_sender);
        tokio::spawn(run_actor(actor));
        Self {
            sender: sequencer_sender,
            _marker: PhantomData,
        }
    }
    async fn handle_message(&mut self, msg: TradeStreamMessage<T>) -> Result<()> {
        let tt = msg.data.to_trades_type()?;
        self.sender.send(SequencerMessage::TakerTrade(tt.clone()));

        tracing::info!("test 2 ");

        // println!("data {}", tt);
        Ok(())
    }

    async fn send(&self, msg: SequencerMessage) -> Result<()> {
        self.sender.send(msg).await;
        Ok(())

        // Ok(())
    }
    // fn receiver(&self) -> &mpsc::Receiver<TradeStreamMessage<T>> {
    //     &self.receiver
    // }
}
//
// SEQUENCER

#[derive(Debug)]
pub struct SequencerActor {
    receiver: mpsc::Receiver<SequencerMessage>,
    queue: BinaryHeap<SequencerMessage>,
    matching_engine_actor: MatchingEngineActor<SequencerMessage>,
    last_ob_update: Instant,
    is_processing_paused: bool,
}

#[derive(Debug, Clone)]
pub enum SequencerMessage {
    TakerTrade(TakerTrades),
    BookModelUpdate(BookModel),
}
impl SequencerMessage {
    #[instrument]
    fn timestamp(&self) -> i64 {
        match self {
            SequencerMessage::TakerTrade(trade) => trade.transaction_timestamp,
            SequencerMessage::BookModelUpdate(book) => book.timestamp,
        }
    }
}

// MATCHING ENGINE
//
#[derive(Debug, Clone)]
pub struct MatchingEngineActor<S>
where
    S: Send,
{
    pub data: S,
}

async fn run_actor<B>(mut actor: B)
where
    B: Actor<T, G> + Handler<S, T, G>,
{
    while let Some(message) = actor.receiver.recv().await {
        actor.handle_message(message).await;
    }
}

// #[async_trait]
// trait ActorHandle {
//     async fn new(&self) -> Self;
//     async fn get_unique_id(&self) -> Self;
// }

// #[async_trait]
// impl<T: ToTakerTrades + Send + 'static> ActorHandle for TradeStreamActor<T> {
//     async fn new(&self) -> Self {
//         todo!();
//     }
//     async fn get_unique_id(&self) -> Self {
//         todo!();
//     }
// }
