use crate::types::*;
use crate::utils::*;
use anyhow::Result;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::VecDeque;
use std::fmt::{Debug, Display};
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
            run_trade_actor(actor.await);
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

async fn run_trade_actor<T: ToTakerTrades + Send + Sync>(
    mut actor: TradeStreamActor<T>,
) -> Result<()> {
    while let Some(message) = actor.receiver.recv().await {
        let send_message = actor.handle_message(message).await?;
    }
    Ok(())
}

//
// SEQUENCER
//
// Sequencer Thread NOTE:
//
// The sequencer thread handles ordering of two message types(this may grow
// as we incorporate other ob stream data), Trade Stream and OB update messages. The thread contain
// three actors, the sequencer logic, statemanagement for the sequencer and a timer to handle state management.
// OB updates determine orderbook state and update every 250ms, if the ob updates are late or cut
// off for some reason, then the matching engine state is no longer accurate and the sequencer
// thread must be paused. The state actor and timer actor look at incoming ob update messages and
// time their arrival. If the arrival is late, in this case we are testing late as > 1000ms then
// the system is paused (the 1000ms cutoff can be lowered later). When this happens the sequencer
// state changes to paused = true and all incoming trade stream messages are rerouted to a queue,
// where they are sorted by timestamp. When the new ob_update comes through than the associated
// timestamp is logged and only the messages that are queued after the ob update timestamp are
// sent to the matching engine  along with the ob_update

#[derive(Debug)]
pub struct SequencerActor {
    receiver: mpsc::Receiver<SequencerMessage>,
    pub queue: VecDeque<SequencerMessage>,
    matching_engine_actor: MatchingEngineActor<SequencerMessage>,
    last_ob_update: Instant,
    is_processing_paused: bool,
}

impl SequencerActor {
    pub fn new(
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
    pub fn handle_message(&mut self, message: SequencerMessage) {
        todo!();
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

//
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

//
// QUEUED MESSAGE TYPES FOR SEQUENCER - Old Code
//
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
