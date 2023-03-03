#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

pub mod action;
pub mod sender;

use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use action::Action;
use bufferfish::Bufferfish;
use futures_util::{SinkExt, StreamExt};
use sender::Sender;
use stubborn_io::{tokio::StubbornIo, ReconnectOptions, StubbornTcpStream};
use tokio::{
    net::TcpStream,
    time::{interval, MissedTickBehavior},
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub type HarpId = (IpAddr, u32);

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
///
/// # Examples
///
/// ```
/// # use harp::Loggable;
/// # use std::net::IpAddr;
/// struct Player {
///     ip: IpAddr,
///     id: u32,
/// }
///
/// impl Loggable for Player {
///     fn identifier(&self) -> (IpAddr, u32) {
///         (self.ip, self.id)
///     }
/// }
/// ```
pub trait Loggable {
    /// Returns an (IP, ID) pair which uniquely identifies this struct.
    fn identifier(&self) -> HarpId;
}

pub struct HarpError {}

pub struct Harp {
    stream: Framed<StubbornIo<TcpStream, SocketAddr>, LengthDelimitedCodec>,
    rx: flume::Receiver<Action>,
    tx: flume::Sender<Action>,
    reserve_queue: Vec<Bufferfish>,
}

impl Harp {
    /// This is a helper function to simplify the initial setup of a Harp
    /// service. It will attempt to connect to the Harp server and, if
    /// successful, will spawn a new task via Tokio to run the service.
    ///
    /// The returned `flume::Sender` is cheaply cloneable, so you can pass it
    /// among threads and tasks.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use harp::{Harp, action::{Action, Kind}, Loggable, HarpId};
    /// # use std::net::{IpAddr, Ipv4Addr};
    /// #
    /// # pub struct MyAction {}
    /// #
    /// # impl Loggable for MyAction {
    /// #     fn identifier(&self) -> HarpId {
    /// #         (IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0)}
    /// # }
    /// #
    /// # pub enum MyKind {
    /// #     A
    /// # }
    /// #
    /// # impl Kind for MyKind {
    /// #     fn key(&self) -> &'static str {
    /// #         match self {
    /// #           MyKind::A => "a"
    /// #         }
    /// #     }
    /// # }
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // The return value here is `flume::Sender<harp::action::Action>`
    /// let harp = Harp::create_service().await?;
    ///
    /// // We can then create an action...
    /// // See the `action::Action` documentation for more information on
    /// // constructing actions and implementing the Loggable trait.
    /// let action = Action::new(MyKind::A, &MyAction{});
    ///
    /// // ...and send it to the Harp server.
    /// harp.send(action)?;
    /// #
    /// #     Ok(())
    /// # }
    /// ```
    ///
    /// See `create_service_with_options` for more information on specifying a
    /// custom hostname and port.
    #[inline(always)]
    pub async fn create_service() -> Result<Sender> {
        let mut harp = Harp::connect().await?;
        let tx = harp.get_sender();

        tokio::spawn(async move {
            let _ = harp.run().await;
        });

        Ok(Sender(tx))
    }

    /// This is a helper function to simplify the initial setup of a Harp
    /// service. Takes a custom hostname and port to connect to the Harp server.
    ///
    /// See `create_service` for more information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use harp::{Harp, action::Action, Loggable, HarpId};
    /// # use std::net::{IpAddr, Ipv4Addr};
    /// #
    /// # pub struct MyAction {}
    /// #
    /// # impl Loggable for MyAction {
    /// #     fn identifier(&self) -> HarpId {
    /// #         (IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0)}
    /// # }
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let harp = Harp::create_service_with_options("127.0.0.1", 7777).await?;
    /// # Ok(())
    /// # }
    #[inline(always)]
    pub async fn create_service_with_options(hostname: &str, port: u16) -> Result<Sender> {
        let mut harp = Harp::connect_with_options(hostname, port).await?;
        let tx = harp.get_sender();

        tokio::spawn(async move {
            let _ = harp.run().await;
        });

        Ok(Sender(tx))
    }

    /// Attempts to connect to the default Harp server. If the connection fails,
    /// an exponential backoff will be used to retry the connection.
    ///
    /// You will need to manually call `run` on the returned `Harp` instance, as
    /// well as move it into a new Tokio task.
    ///
    /// Prefer to use `create_service` or `create_service_with_options` instead,
    /// which handles all of this for you.
    pub async fn connect() -> Result<Self> {
        let addr = Harp::create_addr(None, None);
        Self::raw_connect(addr).await
    }

    /// Attempts to connect to the designated Harp server. If the connection
    /// fails, an exponential backoff will be used to retry the connection.
    ///
    /// You will need to manually call `run` on the returned `Harp` instance, as
    /// well as well as move it into a new Tokio task.
    ///
    /// Prefer to use `create_service` or `create_service_with_options` instead,
    /// which handles all of this for you.
    pub async fn connect_with_options(hostname: &str, port: u16) -> Result<Self> {
        let addr = Harp::create_addr(Some(hostname), Some(port));
        Self::raw_connect(addr).await
    }

    async fn raw_connect(addr: SocketAddr) -> Result<Self> {
        let mut interval = interval(Duration::from_millis(1000));
        // TODO: This could result in massive bursts of actions if the server is
        // disconnected for a long time. This should be configurable, but also
        // probably have a different default.
        interval.set_missed_tick_behavior(MissedTickBehavior::Burst);

        // TODO: Should accept custom backoff generators.
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
    fn create_addr(host: Option<&str>, port: Option<u16>) -> SocketAddr {
        let host =
            host.unwrap_or("127.0.0.1").parse::<IpAddr>().unwrap_or_else(|_| [127, 0, 0, 1].into());
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

fn backoff_generator() -> impl Iterator<Item = std::time::Duration> {
    let mut v = Vec::with_capacity(15);
    for i in 0..RETRY_CONNECT_LIMIT {
        v.push(std::time::Duration::from_secs(u64::from(RETRY_CONNECT_INTERVAL_SECS * i)));
    }

    v.into_iter()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_return_valid_addr() {
        // Invalid host, default port
        let addr = super::Harp::create_addr(Some("hello, world!"), None);
        assert_eq!(addr, SocketAddr::new([127, 0, 0, 1].into(), 7777));

        // Default host and port
        let addr = super::Harp::create_addr(None, None);
        assert_eq!(addr, SocketAddr::new([127, 0, 0, 1].into(), 7777));

        // Valid, custom host and port
        let addr = super::Harp::create_addr(Some("255.255.255.255"), Some(7000));
        assert_eq!(addr, SocketAddr::new([255, 255, 255, 255].into(), 7000));
    }
}
