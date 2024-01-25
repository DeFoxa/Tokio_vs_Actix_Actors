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
        // println!("SM {:?}", sequencer_message);
        println!("test 1");

        self.sequencer_sender
            .send(sequencer_message.clone())
            .await?;
        // .expect("error from trarde stream sequencer sender");
        // println!("SM {:?}", sequencer_message.clone());
        println!("test 2");
        // self.sequencer_sender.send(sequencer_message).await?;

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
            tracing::info!("tokio::spawn, run trade actor");
            run_trade_actor(actor.await).await;
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
#[derive(Debug)]
pub struct OrderBookStreamMessage<T>
where
    T: ToBookModels + Send + Sync + 'static,
{
    pub data: T,
}
#[derive(Debug)]
pub struct OrderBookActor<T>
where
    T: ToBookModels + Send + Sync + 'static,
{
    pub receiver: mpsc::Receiver<OrderBookStreamMessage<T>>,
    pub sequencer_sender: mpsc::Sender<SequencerMessage>,
}
impl<T: ToBookModels + Send + Sync + 'static> OrderBookActor<T> {
    async fn new(
        receiver: mpsc::Receiver<OrderBookStreamMessage<T>>,
        sequencer_sender: mpsc::Sender<SequencerMessage>,
    ) -> Self {
        OrderBookActor {
            receiver,
            sequencer_sender,
        }
    }
    async fn handle_message(&mut self, msg: OrderBookStreamMessage<T>) -> Result<()> {
        let sequencer_message = SequencerMessage::BookModelUpdate(msg.data.to_book_model()?);
        self.sequencer_sender.send(sequencer_message).await?;
        Ok(())
    }
}
#[derive(Debug)]
pub struct OrderBookActorHandler<T>
where
    T: ToBookModels + Send + Sync + 'static,
{
    sender: mpsc::Sender<OrderBookStreamMessage<T>>,
}
impl<T: ToBookModels + Send + Sync + 'static> OrderBookActorHandler<T> {
    pub async fn new(sequencer_sender: mpsc::Sender<SequencerMessage>) -> Self {
        let (sender, receiver) = mpsc::channel(32);
        let actor = OrderBookActor::new(receiver, sequencer_sender);
        tokio::spawn(async move {
            tracing::info!("tokio::spawn, run ob actor");
            run_book_actor(actor.await);
        });
        Self { sender }
    }
    pub async fn send(
        &self,
        msg: OrderBookStreamMessage<T>,
    ) -> Result<(), mpsc::error::SendError<OrderBookStreamMessage<T>>> {
        self.sender.send(msg).await
    }
}

pub async fn run_book_actor<T: ToBookModels + Send + Sync>(
    mut actor: OrderBookActor<T>,
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
//
// Eventually instead of processing pauses, I'll add the GET requests to fill in ob state but for
// now pauses are fine

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
pub struct SequencerActor {
    receiver: mpsc::Receiver<SequencerMessage>,
    state_receiver: mpsc::Receiver<StateManagementMessage>,
    state_update: mpsc::Sender<StateManagementMessage>,
    timer_sender: mpsc::Sender<SequencerMessage>,
    matching_engine_actor: mpsc::Sender<MatchingEngineMessage>,
    pub queue: VecDeque<TakerTrades>,
    sequencer_state: SequencerState,
}

impl SequencerActor {
    pub fn new(
        receiver: mpsc::Receiver<SequencerMessage>,
        state_receiver: mpsc::Receiver<StateManagementMessage>,
        state_update: mpsc::Sender<StateManagementMessage>,
        timer_sender: mpsc::Sender<SequencerMessage>,
        matching_engine_actor: mpsc::Sender<MatchingEngineMessage>,
    ) -> Self {
        SequencerActor {
            receiver,
            state_receiver,
            state_update,
            timer_sender,
            matching_engine_actor,
            queue: VecDeque::new(),
            sequencer_state: SequencerState::default(),
        }
    }

    pub async fn handle_message(&mut self, msg: SequencerMessage) {
        //NOTE: handle SequencerState = Processing
        let matching_engine_msg = match msg {
            SequencerMessage::TakerTrade(trades) => {
                MatchingEngineMessage::TakerTrade(Arc::new(trades))
            }

            SequencerMessage::BookModelUpdate(book_update) => {
                MatchingEngineMessage::BookModelUpdate(book_update)
            }
        };
        tracing::info!("state {:?}", self.sequencer_state);
        self.matching_engine_actor.send(matching_engine_msg).await;
        println!("matching engine send 2");
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                 Some(sequencer_msg) = self.receiver.recv() => {
                     match self.sequencer_state {
                        SequencerState::Processing => {
                            match sequencer_msg {
                                SequencerMessage::BookModelUpdate(ref book_update) => {
                                    tracing::info!("SequencerActor::run(): {:?}", sequencer_msg.clone());
                                    self.timer_sender.send(SequencerMessage::BookModelUpdate(book_update.clone())).await?;
                                    self.handle_message(SequencerMessage::BookModelUpdate(book_update.clone())).await;
                                }
                                _ => {
                                    tracing::info!("SequencerActor::run(): {:?}", sequencer_msg.clone());
                                    self.handle_message(sequencer_msg.clone()).await;
                                    println!("message from internal run looop {:?}", sequencer_msg.clone());
                                    },

                            }
                            // not adding a log::info! message for standard processing for time being, only want
                            // to see non standard scenarios in logs
                        }
                        SequencerState::PauseProcessing => match sequencer_msg {
                            SequencerMessage::TakerTrade(sequencer_msg) => {

                                println!("internal loop 2 ?");
                                self.queue.push_back(sequencer_msg);
                                tracing::info!("processing pause trade message sent to queue");
                            }
                            SequencerMessage::BookModelUpdate(sequencer_msg) => {
                                self.state_update
                                    .send(StateManagementMessage::ResumeProcessing).await;
                                tracing::info!("sequencer run: book_model_update")
                            }
                        },
                        SequencerState::ResumeProcessing => {

                            println!("internal loop 3 ?");
                            self.process_queue(sequencer_msg).await;
                            tracing::info!(
                                "Resuming process from SequencerActor::run()"
                            );
                        }
                    }
                }
                 Some(state_msg) = self.state_receiver.recv() => {
                     match state_msg {
                         StateManagementMessage::Processing => {
                             self.sequencer_state= SequencerState::to_processing();
                            }
                         StateManagementMessage::PauseProcessing => {
                             self.sequencer_state = SequencerState::to_pause_processing();
                         }
                         StateManagementMessage::ResumeProcessing => {
                             self.sequencer_state = SequencerState::to_resume_proceessing();
                         }

                     }
                }
            }
        }
        Ok(())
    }

    pub async fn process_queue(&mut self, ob_update_timestamp: SequencerMessage) {
        // NOTE: this implementation assumes sequential, sorted, ordering  by timestamp, of the incoming stream data
        //  must be verified that this is the common behavior of the stream data. Obviously the binary search is
        //  worthless if the timestamps aren't actually sequentially entering queue by ts, but from what i've seen
        //  re: local_id/timestamp ordering, it should be naturally sorted but I'll verify later. More interested
        //  in the diff b/w backpressure/capacity of the different concurrency models.

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

#[derive(Debug)]
pub enum StateManagementMessage {
    Processing,
    PauseProcessing,
    ResumeProcessing,
}

pub struct TimerActor {
    receiver: mpsc::Receiver<SequencerMessage>,
    state_management_sender: mpsc::Sender<StateManagementMessage>,
}

/// TimerActor is a watchdog for the orderbook state updates. OB stream updates come every ~250ms,
/// in the current implementation an interval is set for 1s, should no ob update come in
/// that period then a SequencerStateMessage is sent to set processing to paused until a new ob
/// update comes through. Eventually I'll change this to integrate rest requests for filling blanks
/// when stream isn't updating.
impl TimerActor {
    pub async fn run_timer(&mut self) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                Some(msg) = self.receiver.recv() => {
                    match msg {
                        SequencerMessage::BookModelUpdate(_) => {
                            interval.reset();
                        }
                        _ => log::debug!("Incorrect SequencerMessage type being passed to timer"),
                    }
                }
                _ = interval.tick() => {
                    self.state_management_sender
                        .send(StateManagementMessage::PauseProcessing)
                        .await?;
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct SequencerHandler;

impl SequencerHandler {
    pub async fn new(
        matching_engine_sender: mpsc::Sender<MatchingEngineMessage>,
    ) -> Result<(Self, mpsc::Sender<SequencerMessage>)> {
        // NOTE, for later reference: centralized instantiator of all sequencer channels,
        // includign timer. This method will be called from main() or whatever other function
        // handles the actor system. ruN_timer() should be called from sequenceractor run(), with
        // ob_update messages passed to the matching-engine, ResumeProcess matched to current
        // state and run_timer for interval resets.

        let (sequencer_sender, sequencer_receiver) = mpsc::channel::<SequencerMessage>(32);
        let (state_sender, state_receiver) = mpsc::channel::<StateManagementMessage>(32);
        let (timer_sender, timer_receiver) = mpsc::channel::<SequencerMessage>(32);
        let (matching_engine_sender, matching_engine_receiver) =
            mpsc::channel::<MatchingEngineMessage>(32);

        let mut sequencer_actor = SequencerActor::new(
            sequencer_receiver,
            state_receiver,
            state_sender.clone(),
            timer_sender,
            matching_engine_sender,
        );
        let mut timer_actor = TimerActor {
            receiver: timer_receiver,
            state_management_sender: state_sender,
        };
        tokio::spawn(async move {
            println!("sequencer run?");
            sequencer_actor.run().await.expect("SequencerActor Failed");
        });

        tokio::spawn(async move {
            timer_actor.run_timer().await.expect("TimerActor Failed");
            println!("sequencer_actor up");
        });
        Ok((SequencerHandler {}, sequencer_sender))
    }
    // pub async fn send(&self, msg: SequencerMessage) -> Result<()> {
    //     self.sequencer_sender.send(msg).await?;
    //     Ok(())
    // }
}

//
// MATCHING ENGINE
//

#[derive(Debug)]
pub struct MatchingEngineActor {
    pub receiver: mpsc::Receiver<MatchingEngineMessage>,
}

impl MatchingEngineActor {
    pub async fn handle_message(&mut self, msg: MatchingEngineMessage) {
        let message = self.receiver.recv().await;
        println!("MATCHING ENGINE MSG {:?}", message);
    }
}
#[derive(Debug)]
pub enum MatchingEngineMessage {
    TakerTrade(Arc<TakerTrades>),
    BookModelUpdate(BookModel),
}
#[derive(Debug)]
pub struct MatchingEngineHandler {}

//OLD SEQUENCER_STATE_ACTOR
//
// pub struct SequencerStateActor {
//     receiver: mpsc::Receiver<StateManagementMessage>,
//     timer_sender: mpsc::Sender<()>,
//     state: SequencerState,
// }
// impl SequencerStateActor {
//     fn new(
//         receiver: mpsc::Receiver<StateManagementMessage>,
//         timer_sender: mpsc::Sender<()>,
//     ) -> Self {
//         SequencerStateActor {
//             receiver,
//             timer_sender,
//             state: SequencerState::Processing,
//         }
//     }
//     async fn run(&mut self, msg: StateManagementMessage) {
//         match msg {
//             StateManagementMessage::Processing => {
//                 self.state = SequencerState::to_processing();
//             }
//             StateManagementMessage::PauseProcessing => {
//                 self.state = SequencerState::to_pause_processing();
//             }
//             StateManagementMessage::ResumeProcessing => {
//                 self.state = SequencerState::to_resume_proceessing();
//             }
//         }
//     }
// }
//
// OLD SEQUENCER RUn
// match self.sequencer_state {
//     SequencerState::Processing => {
//         self.handle_message(message);
//         // not adding a log::info! message for standard processing for time being, only want
//         // to see non standard scenarios in logs
//     }
//     SequencerState::PauseProcessing => match message {
//         SequencerMessage::TakerTrade(message) => {
//             self.queue.push_back(message);
//             log::info!("processing pause trade message sent to queue");
//         }
//         SequencerMessage::BookModelUpdate(message) => {
//             self.state_receiver
//                 .send(StateManagementMessage::ResumeProcessing);
//             log::info!("sequencer run: book_model_update")
//         }
//     },
//     SequencerState::ResumeProcessing => {
//         self.process_queue(message);
//         log::info!(
//             "resume process from run, state should update from StateManagement, verify"
//         );
//     }
// }

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
