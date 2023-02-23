pub mod action;
pub mod error;

use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use action::Action;
use bufferfish::Bufferfish;
use error::HarpError;
use futures_util::SinkExt;
use stubborn_io::{tokio::StubbornIo, ReconnectOptions, StubbornTcpStream};
use tokio::{
    net::TcpStream,
    time::{interval, MissedTickBehavior},
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub type Result<T> = std::result::Result<T, HarpError>;

const MAX_RETIES: u32 = 15;
const BASE_RETRY_INTERVAL: u32 = 3;

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
}

impl Harp {
    /// Attempts to connect to the default Harp server. If the connection fails,
    /// an exponential backoff will be used to retry the connection.
    pub async fn connect() -> Result<Self> {
        let addr = Harp::get_socket_addr(None, None);
        Self::connect_with_address(addr).await
    }

    /// Attempts to connect to the designated Harp server. If the connection
    /// fails, an exponential backoff will be used to retry the connection.
    pub async fn connect_with_options(hostname: IpAddr, port: u16) -> Result<Self> {
        let addr = Harp::get_socket_addr(Some(hostname.to_string()), Some(port));
        Self::connect_with_address(addr).await
    }

    async fn connect_with_address(addr: SocketAddr) -> Result<Self> {
        let mut interval = interval(Duration::from_millis(1000));
        interval.set_missed_tick_behavior(MissedTickBehavior::Burst);

        let options = ReconnectOptions::new().with_retries_generator(backoff_generator);

        // TODO: Expand retries to include fresh connections.
        //Currently, if a service fails to connect to the server (eg. it is down
        // / connectionrefused error), it just closes out. Ideally, we attempt
        // to reconnect to the server.
        let stream = StubbornTcpStream::connect_with_options(addr, options).await?;
        stream.set_nodelay(true)?;

        let stream = LengthDelimitedCodec::builder().length_field_type::<u16>().new_framed(stream);

        let (tx, rx) = flume::unbounded::<Action>();

        tracing::info!("Service connected to Harp on {}", addr);

        Ok(Self { stream, rx, tx })
    }

    /// Convert a provided host and port into a SocketAddr. If no host or port
    /// are provided, defaults to "127.0.0.1:7777".
    fn get_socket_addr(host: Option<String>, port: Option<u16>) -> SocketAddr {
        let host = host
            .unwrap_or_else(|| "127.0.0.1".to_string())
            .parse::<IpAddr>()
            .unwrap_or_else(|_| [127, 0, 0, 1].into());
        let port = port.unwrap_or(7777);

        SocketAddr::new(host, port)
    }

    /// Returns a reference to the write half of the channel. Users can pass
    /// `Action`s into this channel, and they will be processed by Harp.
    pub fn get_send_channel(&self) -> flume::Sender<Action> {
        self.tx.clone()
    }

    /// Starts a new Harp service. This will listen for incoming `Action`s on
    /// the channel, convert them into `Bufferfish` packets, and send them to
    /// the Harp server.
    pub async fn run(&mut self) -> Result<()> {
        loop {
            while let Ok(action) = self.rx.try_recv() {
                tracing::debug!("Sending action: {:?}", action);

                let bf: Bufferfish = action.try_into()?;
                if let Err(e) = self.stream.send(bf.into()).await {
                    tracing::error!("Failed to send action: {}", e);
                }
            }
        }
    }
}

pub fn backoff_generator() -> impl Iterator<Item = std::time::Duration> {
    let mut v = Vec::with_capacity(15);
    for i in 0..MAX_RETIES {
        v.push(std::time::Duration::from_secs(u64::from(BASE_RETRY_INTERVAL * i)));
    }

    v.into_iter()
}
