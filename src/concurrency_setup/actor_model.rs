use crate::types::*;
use crate::utils::*;
use actix::prelude::*;
use anyhow::Result;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt;
use std::fmt::Display;
use std::sync::Arc;
use std::time::{Duration, Instant};

// TODO: Fix the clones tomorrow

/// TradeStream from deserialized exchange specific type to generalized TakerTrades type
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

pub struct TradeStreamActor {
    pub sequencer_addr: Addr<SequencerActor>,
}

impl Actor for TradeStreamActor {
    type Context = Context<Self>;
}

impl<T: ToTakerTrades + Display + 'static> Handler<TradeStreamMessage<T>> for TradeStreamActor {
    type Result = Result<TakerTrades>;
    fn handle(&mut self, msg: TradeStreamMessage<T>, _ctx: &mut Self::Context) -> Self::Result {
        let tt = msg.data.to_trades_type()?;
        self.sequencer_addr
            .do_send(SequencerMessage::TakerTrade(tt.clone()));

        // println!("data {}", tt);
        return Ok(tt);
    }
}
/// OB_UPDATE Message/Actor  
pub struct BookModelStreamMessage<T>
where
    T: ToBookModels,
{
    pub data: T,
}
//
// impl<T> fmt::Display for BookModelStreamMessage<T>
// where
//     T: ToTakerTrades + Display,
// {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "message: {}", self.data)
//     }
// }

impl<T> Message for BookModelStreamMessage<T>
where
    T: ToBookModels + 'static,
{
    type Result = Result<BookModel>;
}

impl<T> fmt::Display for BookModelStreamMessage<T>
where
    T: ToBookModels + Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.data)
    }
}

pub struct BookModelStreamActor {
    pub sequencer_addr: Addr<SequencerActor>,
}

impl Actor for BookModelStreamActor {
    type Context = Context<Self>;
}

impl<T: ToBookModels + 'static> Handler<BookModelStreamMessage<T>> for BookModelStreamActor {
    type Result = Result<BookModel>;
    fn handle(&mut self, msg: BookModelStreamMessage<T>, _ctx: &mut Self::Context) -> Self::Result {
        let book = msg.data.to_book_model()?;
        self.sequencer_addr
            .do_send(SequencerMessage::BookModelUpdate(book.clone()));

        // println!("data {}", tt);
        return Ok(book);
    }
}

/// Going to build a Sequencer as a middleman between the OB_update/trade streams and the matching engine
/// to ensure proper sequential ordering of the messages, using exchange timestamps
/// How the Sequencer works: OB update data comes in every 250ms, trade stream is theoretically
/// fifo from the exchange. we are going to queue messages in blocks of 250ms, so 1 ob update per
/// outgoing message block. any trade stream updates that are prior to the next 250ms interval go
/// out immediately, the rest
/// Sequencer Actor

#[derive(Debug)]
pub struct SequencerActor {
    pub queue: BinaryHeap<SequencerMessage>,
    pub matching_engine_addr: Addr<MatchingEngineActor>,
    pub last_ob_update: Instant,
    pub is_processing_paused: bool,
}
impl SequencerActor {
    fn new(matching_engine_addr: Addr<MatchingEngineActor>) -> Self {
        SequencerActor {
            queue: BinaryHeap::new(),
            matching_engine_addr,
            last_ob_update: Instant::now(),
            is_processing_paused: false,
        }
    }
    fn start_periodic_check(ctx: &mut Context<Self>) {
        ctx.notify_later(CheckAndForward, Duration::from_millis(10));
    }
}
impl Actor for SequencerActor {
    type Context = Context<Self>;
}

impl Handler<SequencerMessage> for SequencerActor {
    type Result = ();

    fn handle(&mut self, msg: SequencerMessage, ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            SequencerMessage::BookModelUpdate(ref book) => {
                self.last_ob_update = Instant::now();
                self.is_processing_paused = false;
                self.matching_engine_addr.try_send(book.clone());
            }
            SequencerMessage::TakerTrade(ref trade) => {
                if self.is_processing_paused
                    || Instant::now().duration_since(self.last_ob_update)
                        > Duration::from_millis(1000)
                {
                    self.queue.push(msg.clone());
                    log::debug!("processing paused, time since last update > 1s (1000ms)");
                } else {
                    self.matching_engine_addr.do_send(trade.clone());
                }
            }
        }
        self.queue.push(msg.clone());
    }
}
#[derive(Debug, Clone)]
pub enum SequencerMessage {
    TakerTrade(TakerTrades),
    BookModelUpdate(BookModel),
}
impl SequencerMessage {
    fn timestamp(&self) -> i64 {
        match self {
            SequencerMessage::TakerTrade(trade) => trade.transaction_timestamp,
            SequencerMessage::BookModelUpdate(book) => book.timestamp,
        }
    }
}

impl Message for SequencerMessage {
    type Result = ();
}

impl Message for TakerTrades {
    type Result = ();
}

impl Ord for SequencerMessage {
    fn cmp(&self, other: &Self) -> Ordering {
        other.timestamp().cmp(&self.timestamp())
    }
}
impl PartialOrd for SequencerMessage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Eq for SequencerMessage {}

impl PartialEq for SequencerMessage {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp() == other.timestamp()
    }
}
impl Message for BookModel {
    type Result = ();
}
struct CheckAndForward;
impl Message for CheckAndForward {
    type Result = ();
}

impl Handler<CheckAndForward> for SequencerActor {
    type Result = ();

    fn handle(&mut self, _msg: CheckAndForward, ctx: &mut Context<Self>) -> Self::Result {
        if !self.is_processing_paused {
            while let Some(message) = self.queue.pop() {
                if let SequencerMessage::TakerTrade(trade) = message {
                    if trade.transaction_timestamp
                        <= self.last_ob_update.elapsed().as_millis() as i64
                    {
                        self.matching_engine_addr.do_send(trade);
                    }
                }
            }
        }
        ctx.notify_later(CheckAndForward, Duration::from_millis(10));
    }
}
/// Matching engine Arbiter/Actor. Takes in data from OB_updates and Trade stream
///

#[derive(Debug, Clone)]
pub struct MatchingEngineActor {
    pub data: i64,
}

impl Actor for MatchingEngineActor {
    type Context = Context<Self>;
}

impl Handler<BookModel> for MatchingEngineActor {
    type Result = ();

    fn handle(&mut self, msg: BookModel, ctx: &mut Context<Self>) -> Self::Result {
        println!(" {:?}", msg);
    }
}

impl Handler<TakerTrades> for MatchingEngineActor {
    type Result = ();
    fn handle(&mut self, msg: TakerTrades, ctx: &mut Context<Self>) -> Self::Result {
        println!("{:?}", msg);
    }
}
