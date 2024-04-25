#[allow(unused)]
#[allow(unused_must_use)]
use crate::types::*;
use anyhow::Result;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};
use std::{fmt::Debug, sync::Arc, time::Duration};
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::instrument;

///
/// Trade Stream Actor
///
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
    fn new(
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
        self.sequencer_sender
            .send(sequencer_message.clone())
            .await?;

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
    pub fn new(sequencer_sender: mpsc::Sender<SequencerMessage>) -> Result<(Self, JoinHandle<()>)> {
        let (sender, receiver) = mpsc::channel(32);
        let actor = TradeStreamActor::new(receiver, sequencer_sender);
        let trade_stream_handle = tokio::spawn(async move {
            tracing::info!("tokio::spawn, run trade actor");
            run_trade_actor(actor).await;
        });

        Ok((Self { sender }, trade_stream_handle))
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
        let _send_message = actor.handle_message(message).await?;
    }
    Ok(())
}

///
/// OrderBook Actor
///
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
    fn new(
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
    pub fn new(sequencer_sender: mpsc::Sender<SequencerMessage>) -> Result<(Self, JoinHandle<()>)> {
        let (sender, receiver) = mpsc::channel(32);
        let actor = OrderBookActor::new(receiver, sequencer_sender);
        let order_book_handle = tokio::spawn(async move {
            tracing::info!("tokio::spawn, run ob actor");
            run_book_actor(actor).await;
        });

        Ok((Self { sender }, order_book_handle))
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
        let _send_message = actor.handle_message(message).await?;
    }
    Ok(())
}

// if the ob updates are late or cut off for some reason, then the matching engine
// state is no longer accurate and the sequencer thread is paused. The state
// actor and timer actor look at incoming ob update messages and time their arrival.
// If the arrival is late, in this case, arrival > 1000ms then the
// system is paused. If ob previous update > 1000ms the sequencer
// state changes to paused = true and all incoming trade stream messages are rerouted to a queue,
// where they are sorted by timestamp. On new ob_update the associated timestamp is logged and
// only the messages that are queued after the ob update timestamp are sent to the matching engine
// along with the ob_update

///
/// SEQUENCER
///
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
    pub sequencer_state: SequencerState,
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
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                 Some(sequencer_msg) = self.receiver.recv() => {
                     match self.sequencer_state {
                        SequencerState::Processing => {
                            match sequencer_msg {
                                SequencerMessage::BookModelUpdate(ref book_update) => {

                                     self.timer_sender.send(SequencerMessage::BookModelUpdate(book_update.clone())).await?;
                                     tracing::info!("SequencerActor::run(): {:?}", sequencer_msg.clone());
                                    self.handle_message(SequencerMessage::BookModelUpdate(book_update.clone())).await;
                                }
                                _ => {
                                    tracing::info!("SequencerActor::run(): {:?}", sequencer_msg.clone());
                                    self.handle_message(sequencer_msg.clone()).await;
                                    },

                            }
                        }
                        SequencerState::PauseProcessing => match sequencer_msg {
                            SequencerMessage::TakerTrade(sequencer_msg) => {
                                self.queue.push_back(sequencer_msg);
                                tracing::info!("processing pause trade message sent to queue");

                            }
                            SequencerMessage::BookModelUpdate(sequencer_msg) => {
                                 self.timer_sender.send(SequencerMessage::BookModelUpdate(sequencer_msg));
                                 self.state_update
                                    .send(StateManagementMessage::ResumeProcessing).await;
                                tracing::info!("sequencer run: book_model_update")
                            }
                        },
                        SequencerState::ResumeProcessing => {

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
        // Ok(())
    }

    pub async fn process_queue(&mut self, ob_update_timestamp: SequencerMessage) {
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

        tracing::info!(
            "**queue** {}, process {:?}",
            self.queue.clone().len(),
            self.sequencer_state
        );

        for message in self.queue.drain(..index) {
            let matching_engine_message = MatchingEngineMessage::TakerTrade(Arc::new(message));
            self.matching_engine_actor
                .send(matching_engine_message)
                .await;
        }
        if self.queue.len() == 0 {
            let _ = self.sequencer_state = SequencerState::Processing;
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

impl TimerActor {
    pub async fn run_timer(&mut self) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                Some(msg) = self.receiver.recv() => {
                    match msg {
                        SequencerMessage::BookModelUpdate(_msg) => {
                            interval.reset();
                            tracing::info!("interval reset");
                        }

                        _ => log::debug!("Incorrect SequencerMessage type being passed to timer"),

                    }
                }
                _ = interval.tick() => {
                    self.state_management_sender
                        .send(StateManagementMessage::PauseProcessing)
                        .await?;
                    tracing::info!("Pause Processing From TimerActor");
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct SequencerHandler;

impl SequencerHandler {
    pub fn new(
        matching_engine_sender: mpsc::Sender<MatchingEngineMessage>,
    ) -> Result<(
        mpsc::Sender<SequencerMessage>,
        JoinHandle<()>,
        JoinHandle<()>,
    )> {
        let (sequencer_sender, sequencer_receiver) = mpsc::channel::<SequencerMessage>(32);
        let (state_sender, state_receiver) = mpsc::channel::<StateManagementMessage>(32);
        let (timer_sender, timer_receiver) = mpsc::channel::<SequencerMessage>(32);

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
        let sequencer_actor_handle = tokio::spawn(async move {
            match sequencer_actor.run().await {
                Ok(_) => tracing::info!("spawn SequencerActor successfully"),
                Err(_) => tracing::debug!("Error spawning SequencerActor"),
            }
        });

        let timer_handle = tokio::spawn(async move {
            match timer_actor.run_timer().await {
                Ok(_) => tracing::info!("spawn TimerActor successfully"),
                Err(_) => tracing::debug!("error spawning TimerActor"),
            }
        });
        Ok((sequencer_sender, sequencer_actor_handle, timer_handle))
    }
}

///
/// MATCHING ENGINE
///
#[derive(Debug)]
pub struct MatchingEngineActor {
    pub receiver: mpsc::Receiver<MatchingEngineMessage>,
    pub testing_counter: AtomicU32,
}

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

impl MatchingEngineActor {
    pub async fn handle_message(&mut self, msg: MatchingEngineMessage) {
        println!("MATCHING ENGINE MSG {:?}", msg);
        self.testing_counter = NEXT_ID.fetch_add(1, Ordering::Relaxed).into();
        println!("counter {:?}", self.testing_counter);
        tracing::info!(
            "msg reached end of pipeline, counter {:?}",
            self.testing_counter
        );
    }
    pub async fn run(&mut self) -> Result<()> {
        while let Some(matching_engine_message) = self.receiver.recv().await {
            self.handle_message(matching_engine_message).await;
        }
        Ok(())
    }
}
#[derive(Debug)]
pub enum MatchingEngineMessage {
    TakerTrade(Arc<TakerTrades>),
    BookModelUpdate(BookModel),
}
#[derive(Debug)]
pub struct MatchingEngineHandler;
impl MatchingEngineHandler {
    pub fn new() -> Result<(mpsc::Sender<MatchingEngineMessage>, JoinHandle<()>)> {
        let (sender, receiver) = mpsc::channel(32);
        let mut mea = MatchingEngineActor {
            receiver,
            testing_counter: 0.into(),
        };

        let matching_engine_handle = tokio::spawn(async move {
            match mea.run().await {
                Ok(_) => tracing::info!("MatchingEngineActor spawn success"),
                Err(_) => tracing::debug!("Error spawning MatchingEngineActor"),
            }
        });

        Ok((sender, matching_engine_handle))
    }
}
