use crate::types::*;
use crate::utils::*;
use anyhow::Result;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fmt;
use std::fmt::{Debug, Display};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration as TD, Interval};
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
    state_receiver: mpsc::Sender<StateManagementMessage>,
    matching_engine_actor: mpsc::Sender<MatchingEngineMessage>,
    pub queue: VecDeque<TakerTrades>,
    last_ob_update: Instant,
    sequencer_state: SequencerState,
    previous_trade_timestamp: Option<i64>,
}

impl SequencerActor {
    pub fn new(
        receiver: mpsc::Receiver<SequencerMessage>,
        state_receiver: mpsc::Sender<StateManagementMessage>,
        matching_engine_actor: mpsc::Sender<MatchingEngineMessage>,
    ) -> Self {
        SequencerActor {
            receiver,
            state_receiver,
            matching_engine_actor,
            queue: VecDeque::new(),
            last_ob_update: Instant::now(),
            previous_trade_timestamp: None,
            sequencer_state: SequencerState::default(),
        }
    }
    pub fn handle_message(&mut self, msg: SequencerMessage) {
        //NOTE: handle SequencerState = Processing
        let matching_engine_msg = match msg {
            SequencerMessage::TakerTrade(trades) => {
                MatchingEngineMessage::TakerTrade(Arc::new(trades))
            }

            SequencerMessage::BookModelUpdate(book_update) => {
                MatchingEngineMessage::BookModelUpdate(book_update)
            }
        };
        self.matching_engine_actor.send(matching_engine_msg);

        //NOTE: Below is pseudocode for verifying that each new trade msg is sequenced properly
        //relative to previous messages, im including this but don't think it will be performant
        //enough to justify the safety measures. Matching engine will be too slow relative to
        //exchange matching engine.
        //
        //Going to test the accuracy of stream messages seperately during high vol to see if such a safety
        //measure is necessary and then will test safe/accurate system relative to unsafe for
        //perf. Including this here so I don't forget later. Another option is to spin off another
        //thread that handles verifying transaction ordering with ability to send new state update
        //to Sequencer to pause if x number of incoming trades arrived in inaccurate sequence.
        //
        // match msg {
        //     SequencerMessage::TakerTrade(trade) => {
        //         if msg.transaction_timestamp >= self.previous_trade_timestamp{
        //             self.matching_engine_actor.send(msg);
        //         } else {
        //             self.stream_sequencer_queue.push_back(msg);
        //             self.process_stream_error_queue(); write a method that will handle queued
        //             messages in cases where the sequential ordering of data coming from
        //             exchange stream
        //             is inaccurate.
        //         }
        //     }
        // }
    }

    pub async fn run(&mut self, message: SequencerMessage) {
        match self.sequencer_state {
            SequencerState::Processing => {
                self.handle_message(message);
                // not adding a log::info! message for standard processing for time being, only want
                // to see non standard scenarios in logs
            }
            SequencerState::PauseProcessing => match message {
                SequencerMessage::TakerTrade(message) => {
                    self.queue.push_back(message);
                    log::info!("processing pause trade message sent to queue");
                }
                SequencerMessage::BookModelUpdate(message) => {
                    self.state_receiver
                        .send(StateManagementMessage::ResumeProcessing);
                    log::info!("sequencer run: book_model_update")
                }
            },
            SequencerState::ResumeProcessing => {
                self.process_queue(message);
                log::info!(
                    "resume process from run, state should update from StateManagement, verify"
                );
            }
        }
    }

    pub async fn process_queue(&mut self, ob_update_timestamp: SequencerMessage) {
        // NOTE: this implementation assumes sequential ordering, by timestamp, of the data coming
        // from the ws stream, must be verified that this is the common behavior of the stream.
        // i.e. no common, (we can always handle rare cases later), errors in transaction timestamp sent through the stream.
        self.queue
            .make_contiguous()
            .sort_by(|a, b| a.transaction_timestamp.cmp(&b.transaction_timestamp));
        let index = match self
            .queue
            .binary_search_by_key(&ob_update_timestamp.timestamp(), |message| {
                message.transaction_timestamp
            }) {
            Ok(index) => index,
            Err(index) => index,
        };

        for message in self.queue.drain(..index) {
            let matching_engine_message = MatchingEngineMessage::TakerTrade(Arc::new(message));
            self.matching_engine_actor
                .send(matching_engine_message)
                .await;
        }
    }
}

#[derive(Debug)]
pub struct SequencerHandler {
    sender: mpsc::Sender<SequencerMessage>,
}
impl SequencerHandler {
    async fn new(matching_engine_sender: mpsc::Sender<MatchingEngineMessage>) -> Self {
        // let (sender, reciever) = mpsc::channel(32);
        let (state_sender, state_receiver) = mpsc::channel(32);
        let (matching_eng_sender, matching_eng_recv) = mpsc::channel(32);
        let (sequencer_sender, sequencer_receiver) = mpsc::channel(32);
        let (timer_sender, timer_reciver) = mpsc::channel(32);

        let sequencer_actor = SequencerActor::new(
            sequencer_receiver,
            state_sender.clone(),
            matching_eng_sender,
        );
        let sequencer_state_actor = SequencerStateActor::new(state_receiver, timer_sender);

        // tokio::spawn(async move {
        //     state_management_actor.
        // });

        Self {
            sender: sequencer_sender,
        }
    }
    // let actor = SequencerActor::new()
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

pub struct SequencerStateActor {
    receiver: mpsc::Receiver<StateManagementMessage>,
    timer_sender: mpsc::Sender<()>,
    state: SequencerState,
}
impl SequencerStateActor {
    fn new(
        receiver: mpsc::Receiver<StateManagementMessage>,
        timer_sender: mpsc::Sender<()>,
    ) -> Self {
        SequencerStateActor {
            receiver,
            timer_sender,
            state: SequencerState::Processing,
        }
    }
    async fn run(&mut self, msg: StateManagementMessage) {
        match msg {
            StateManagementMessage::Processing => {
                self.state = SequencerState::to_processing();
            }
            StateManagementMessage::PauseProcessing => {
                self.state = SequencerState::to_pause_processing();
            }
            StateManagementMessage::ResumeProcessing => {
                self.state = SequencerState::to_resume_proceessing();
            }
        }
    }
}
#[derive(Debug)]
pub enum SequencerState {
    Processing,
    PauseProcessing,
    ResumeProcessing,
}
impl SequencerState {
    fn to_processing() -> Self {
        SequencerState::Processing
    }
    fn to_pause_processing() -> Self {
        SequencerState::PauseProcessing
    }
    fn to_resume_proceessing() -> Self {
        SequencerState::ResumeProcessing
    }
}
impl Default for SequencerState {
    fn default() -> Self {
        SequencerState::Processing
    }
}
#[derive(Debug)]
pub enum StateManagementMessage {
    Processing,
    PauseProcessing,
    ResumeProcessing,
}

pub struct TimerActor {
    receiver: mpsc::Receiver<()>,
    state_management_sender: mpsc::Sender<StateManagementMessage>,
}
impl TimerActor {
    pub async fn run_timer(&self, mut receiver: mpsc::Receiver<SequencerMessage>) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                Some(msg) = receiver.recv() => {
                    match msg {
                        SequencerMessage::BookModelUpdate(ref book_model) => {
                            interval.reset();
                        }
                        _ => log::debug!("Incorrect SequencerMessage type being passed to timer"),
                    }
                }
                _ = interval.tick() => {
                    self.state_management_sender
                        .send(StateManagementMessage::PauseProcessing)
                        .await?;
                    break;
                }
            }
        }

        Ok(())
    }
}
//
// MATCHING ENGINE
//

#[derive(Debug, Clone)]
pub struct MatchingEngineActor<S>
where
    S: Send + Sync,
{
    pub data: S,
}

impl<S: Send + Sync> MatchingEngineActor<S> {
    fn handle_message(&self, msg: MatchingEngineMessage) {
        println!("{:?}", msg);
    }
}
#[derive(Debug)]
pub enum MatchingEngineMessage {
    TakerTrade(Arc<TakerTrades>),
    BookModelUpdate(BookModel),
}
//OLD RUN
// loop {
//     tokio::select! {
//         Some(message) = self.receiver.recv() => {
//             match message {
//                 SequencerMessage::TakerTrade => {
//                     self.handle_message(message)
//                 }
//                 SequencerMessage::BookModelUpdate {
//                     log::info!("state_management will handle updated")
//                 }
//             }
//
//
//         }
//         Some(state_message) = self.state_receiver.recv() =>  {
//             match state_message {
//                 StateManagementMessage::Processing => {
//                     self.is_processing_paused = false;
//                 }
//                 StateManagementMessage::PauseProcessing => {
//                     self.is_processing_paused = true;
//                 }
//                 StateManagementMessage::ResumeProcessing => {
//                     self.is_processing_paused = false;
//                     while let Some(queued_message) = self.queue.pop_front() {
//                         self.process_queue(queued_message).await;
//                     }
//                 }
//             }
//
//         }
//     }
// }

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
