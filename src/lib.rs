#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

pub mod action;
pub mod error;
pub mod macros;

use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use action::Action;
use bufferfish::Bufferfish;
use error::HarpError;
use futures_lite::StreamExt;
use futures_util::SinkExt;
use stubborn_io::{tokio::StubbornIo, ReconnectOptions, StubbornTcpStream};
use tokio::{
    net::TcpStream,
    time::{interval, MissedTickBehavior},
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub type Result<T> = std::result::Result<T, HarpError>;

/// The maximum amount of times this service will attempt to reconnect to the
/// Harp server.
const RETRY_CONNECT_LIMIT: u32 = 15;
/// The amount of time in seconds, multiplied by the retry count, to wait before
/// attempting to reconnect to the Harp server.
const RETRY_CONNECT_INTERVAL_SECS: u32 = 3;
/// The amount of time in seconds to wait before attempting to resend actions in
/// the reserve queue.
const RETRY_RESERVE_INTERVAL_SECS: u64 = 3;
/// The maximum amount of actions to send from the reserve queue each tick.
const RETRY_RESERVE_BATCH_SIZE: usize = 10;

/// Structs which implement the `Loggable` trait are able to be identified by a
/// pair of IP and ID - generally a specific player / account or an unidentified
/// connection.
pub trait Loggable {
    fn identifier(&self) -> HarpId;
}

pub type HarpId = (IpAddr, u32);

pub struct Harp {
    stream: Framed<StubbornIo<TcpStream, SocketAddr>, LengthDelimitedCodec>,
    rx: flume::Receiver<Action>,
    tx: flume::Sender<Action>,
    reserve_queue: Vec<Bufferfish>,
}

impl Harp {
    /// Attempts to connect to the default Harp server. If the connection fails,
    /// an exponential backoff will be used to retry the connection.
    pub async fn connect() -> Result<Self> {
        let addr = Harp::create_addr(None, None);
        Self::connect_with_address(addr).await
    }

    /// Attempts to connect to the designated Harp server. If the connection
    /// fails, an exponential backoff will be used to retry the connection.
    pub async fn connect_with_options(hostname: IpAddr, port: u16) -> Result<Self> {
        let addr = Harp::create_addr(Some(hostname.to_string()), Some(port));
        Self::connect_with_address(addr).await
    }

    async fn connect_with_address(addr: SocketAddr) -> Result<Self> {
        let mut interval = interval(Duration::from_millis(1000));
        // TODO: This could result in massive bursts of actions if the server is
        // disconnected for a long time. This should be configurable, but also
        // probably have a different default.
        interval.set_missed_tick_behavior(MissedTickBehavior::Burst);

        let options = ReconnectOptions::new().with_retries_generator(backoff_generator);

        // TODO: Expand retries to include fresh connections. Currently, if a
        //service fails to connect to the server (received a ConnectionRefused
        // error), it just closes out. Ideally, we attempt to reconnect to the
        // server.
        let stream = StubbornTcpStream::connect_with_options(addr, options).await?;
        stream.set_nodelay(true)?;

        let stream = LengthDelimitedCodec::builder().length_field_type::<u16>().new_framed(stream);

        let (tx, rx) = flume::unbounded::<Action>();

        tracing::info!("Service connected to Harp on {addr}");

        Ok(Self { stream, rx, tx, reserve_queue: Vec::with_capacity(10) })
    }

    /// Convert a provided host and port into a `SocketAddr`. If no host or port
    /// are provided, defaults to "127.0.0.1:7777".
    fn create_addr(host: Option<String>, port: Option<u16>) -> SocketAddr {
        let host = host
            .unwrap_or_else(|| "127.0.0.1".to_string())
            .parse::<IpAddr>()
            .unwrap_or_else(|_| [127, 0, 0, 1].into());
        let port = port.unwrap_or(7777);

        SocketAddr::new(host, port)
    }

    /// Returns a reference to the write half of the channel. Users can pass
    /// `Action`s into this channel, and they will be processed by Harp.
    pub fn get_sender(&self) -> flume::Sender<Action> {
        self.tx.clone()
    }

    /// Starts a new Harp service. This will listen for incoming `Action`s on
    /// the channel, convert them into `Bufferfish` packets, and send them to
    /// the Harp server.
    pub async fn run(&mut self) -> Result<()> {
        let mut interval = interval(Duration::from_secs(RETRY_RESERVE_INTERVAL_SECS));

        loop {
            tokio::select! {
                Some(Ok(bytes)) = self.stream.next() => {
                    // If we ever receive a message from the Harp server, it is
                    // because an action was not able to be processed and has
                    // been returned. The Bufferfish will be stored in the
                    // reserve queue and retried later.
                    let bf = Bufferfish::from(bytes);
                    self.reserve_queue.push(bf);
                },
                Ok(action) = self.rx.recv_async() => {
                    let bf: Bufferfish = action.try_into()?;
                    if let Err(e) = self.stream.send(bf.into()).await {
                        tracing::error!("Failed to send action: {e}");
                    }
                }
                _ = interval.tick() => {
                    // If we have any actions in the reserve queue, we should
                    // attempt to send them again.
                    if !self.reserve_queue.is_empty() {
                        tracing::debug!("Attempting to resend {} actions", self.reserve_queue.len());

                        // As the reserve queue is only used due to a serious
                        // server error, we will drip feed the actions back in
                        // case the server is still suffering from backpressure.
                        for bf in self.reserve_queue.drain(..RETRY_RESERVE_BATCH_SIZE) {
                            if let Err(e) = self.stream.send(bf.into()).await {
                                tracing::error!("Failed to send action: {e}");
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn backoff_generator() -> impl Iterator<Item = std::time::Duration> {
    let mut v = Vec::with_capacity(15);
    for i in 0..RETRY_CONNECT_LIMIT {
        v.push(std::time::Duration::from_secs(u64::from(RETRY_CONNECT_INTERVAL_SECS * i)));
    }

    v.into_iter()
}
