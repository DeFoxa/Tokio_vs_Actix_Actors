use crate::types::*;
use crate::utils::*;
use anyhow::Result;
use async_trait::async_trait;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::VecDeque;
use std::fmt;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tracing::{event, info, instrument, Level};
use tracing_subscriber::prelude::*;

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
    pub sequencer_sender: mpsc::Sender<SequencerMessage>,
}

impl<T: ToTakerTrades + Send + Sync + 'static> TradeStreamActor<T> {
    async fn new(
        receiver: mpsc::Receiver<TradeStreamMessage<T>>,
        sequencer_sender: mpsc::Sender<SequencerMessage>,
    ) -> Self {
        TradeStreamActor {
            receiver,
            sequencer_sender,
        }
    }
    async fn handle_message(&mut self, msg: TradeStreamMessage<T>) -> Result<()> {
        let sequencer_message = SequencerMessage::TakerTrade(msg.data.to_trades_type()?);

        self.sequencer_sender.send(sequencer_message).await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TradeStreamActorHandler<T>
where
    T: ToTakerTrades + Send + Sync + 'static,
{
    sender: mpsc::Sender<TradeStreamMessage<T>>,
}
impl<T: ToTakerTrades + Send + Sync + 'static> TradeStreamActorHandler<T> {
    pub async fn new(sequencer_sender: mpsc::Sender<SequencerMessage>) -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let actor = TradeStreamActor::new(receiver, sequencer_sender);
        tokio::spawn(async move {
            run_actor(actor.await);
        });
        Self { sender }
    }
    pub async fn send(
        &self,
        msg: TradeStreamMessage<T>,
    ) -> Result<(), mpsc::error::SendError<TradeStreamMessage<T>>> {
        self.sender.send(msg).await
    }
}

async fn run_actor<T: ToTakerTrades + Send + Sync>(mut actor: TradeStreamActor<T>) -> Result<()> {
    while let Some(message) = actor.receiver.recv().await {
        let send_message = actor.handle_message(message).await?;
    }
    Ok(())
}
// SEQUENCER

#[derive(Debug)]
pub struct SequencerActor {
    receiver: mpsc::Receiver<SequencerMessage>,
    pub queue: VecDeque<SequencerMessage>,
    matching_engine_actor: MatchingEngineActor<SequencerMessage>,
    last_ob_update: Instant,
    is_processing_paused: bool,
}

impl SequencerActor {
    fn new(
        receiver: mpsc::Receiver<SequencerMessage>,
        matching_engine_actor: MatchingEngineActor<SequencerMessage>,
    ) -> Self {
        SequencerActor {
            receiver,
            queue: VecDeque::new(),
            matching_engine_actor,
            last_ob_update: Instant::now(),
            is_processing_paused: false,
        }
    }
    pub fn enqueue_message(&mut self, message: SequencerMessage) {
        self.queue.push_back(message);
    }
    pub async fn process_queue(&mut self, is_processing_paused: bool) {
        while let Some(queued_message) = self.queue.pop_front() {
            todo!();
            break;
        }
    }
}

//

#[derive(Debug)]
pub struct SequencerHandler {
    sender: mpsc::Sender<SequencerMessage>,
}
impl SequencerHandler {
    async fn new(matching_engine_sender: mpsc::Sender<MatchingEngineMessage>) -> Self {
        let (sender, reciever) = mpsc::channel(32);
        // let actor = SequencerActor::new()
        Self { sender }
    }
}
#[derive(Debug)]
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
pub enum MatchingEngineMessage {
    TakerTrade,
    BookModelUpdate,
}

// QUEUED MESSAGE TYPES FOR SEQUENCER
// #[derive(Debug, PartialEq, Eq)]
// pub struct SequencerMessageWrapper {
//     pub message: SequencerMessage,
// }
//
// #[derive(Debug, Eq, PartialEq)]
// pub struct QueuedMessages
// // where
// //     T: Eq + PartialEq,
// {
//     pub timestamp: Instant,
//     pub message: SequencerMessage,
// }
// impl Ord for QueuedMessages {
//     fn cmp(&self, other: &Self) -> Ordering {
//         other.timestamp.cmp(&self.timestamp)
//     }
// }
// impl PartialOrd for QueuedMessages {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         Some(self.cmp(other))
//     }
// }

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
//

//
//
// #[async_trait]
// impl<T: ToTakerTrades + Send + Sync>
//     Handler<SequencerMessage, TradeStreamMessage<T>, SequencerMessage>
//     for TradeStreamActorHandler<T>
// {
//     async fn new(sequencer_sender: mpsc::Sender<SequencerMessage>) -> Self {
//         let (sender, receiver) = mpsc::channel(32);
//         let actor: TradeStreamActor<BinanceTrades> =
//             TradeStreamActor::new(receiver, sequencer_sender);
//         tokio::spawn(run_actor(actor));
//         Self {
//             sender: sequencer_sender,
//             _marker: PhantomData,
//         }
//     }
//     async fn handle_message(&mut self, msg: TradeStreamMessage<T>) -> Result<()> {
//         let tt = msg.data.to_trades_type()?;
//         self.sender.send(SequencerMessage::TakerTrade(tt.clone()));
//
//         tracing::info!("test 2 ");
//
//         // println!("data {}", tt);
//         Ok(())
//     }
//
//     async fn send(&self, msg: SequencerMessage) -> Result<()> {
//         self.sender.send(msg).await;
//         Ok(())
//
//         // Ok(())
//     }
//     // fn receiver(&self) -> &mpsc::Receiver<TradeStreamMessage<T>> {
//     //     &self.receiver
//     // }
// }
//
//
//
// #[async_trait]
// trait Handler<S, T, G> {
//     async fn new(sender: mpsc::Sender<S>) -> Self;
//     async fn handle_message(&mut self, msg: T) -> Result<()>;
//     async fn send(&self, msg: G) -> Result<()>;
//     // fn get_receiver(&self) -> &mpsc::Reciever<T>;
// }
// #[async_trait]
// trait Actor<T, G> {
//     async fn new(receiver: mpsc::Receiver<T>, sender: mpsc::Sender<G>) -> Self;
// }
// TRADE STREAM
// impl<T: ToTakerTrades + Send + Sync + 'static> Actor<T, G> for TradeStreamActor<T> {
//     async fn new(receiver: mpsc::Receiver<T>, sender: mpsc::Sender<G>) -> Self {
//         Self { receiver, sender }
//     }
// }
