#![forbid(unsafe_code)]
#![feature(vec_push_within_capacity)]

use bufferfish::Bufferfish;

use futures_lite::StreamExt;

use futures_util::SinkExt;
use harp::{action::Action, Result};
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::RwLock,
    time::interval,
};
use tokio_util::codec::LengthDelimitedCodec;

type SharedQueue = Arc<RwLock<Vec<Action>>>;

const POSTGRES_BIND_LIMIT: usize = 65535;
const PROCESS_QUEUE_INTERVAL_SECS: u64 = 1;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // TODO: Config file to set DATABASE_URL
    let pg = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://harp:harp@localhost:5432/harp")
        .await?;

    // TODO: The migration files need to be embed in the binary at build time.
    sqlx::migrate!().run(&pg).await?;

    // TODO: Config file to set hostname:port
    let port_env = std::env::var("PORT").unwrap_or_else(|_| "7777".to_string());
    let port = port_env.parse::<u16>().unwrap_or(7777);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    // Attempt to connect to the harpd server
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("harpd listening on {addr}");

    // Create a shared queue for actions; we clone it immediately as we have to
    // move it across threads for the queue processor.
    //
    // Initially, we will allocate space for 100 Actions. This will be resized
    // as needed in the queue processor.
    let shared_queue = Arc::new(RwLock::new(Vec::with_capacity(1)));
    let mut queue = Arc::clone(&shared_queue);

    tokio::task::spawn(async move {
        let mut interval = interval(Duration::from_secs(PROCESS_QUEUE_INTERVAL_SECS));
        let pg = Arc::new(pg);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = process_queue(&mut queue, Arc::clone(&pg)).await {
                        tracing::error!("Error processing queue: {}", e);
                    }
                }
            };
        }
    });

    // Accept connections from external services; each of these connections also
    // needs a reference to the shared queue.
    loop {
        tokio::select! {
            Ok((stream, addr)) = listener.accept() => {
                tracing::info!("Service connected: {}", addr);

                let queue = Arc::clone(&shared_queue);
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(addr, stream, queue).await {
                        tracing::error!("Error handling connection: {}", e);
                    }
                })
            }
        };
    }
}

/// Iterates over the shared queue, building a batch query of actions to be
/// executed in a single transaction on the database.
async fn process_queue(queue: &mut SharedQueue, pg: Arc<PgPool>) -> Result<()> {
    let mut queue = queue.write().await;

    // If the queue is empty, we don't need to do anything.
    if queue.is_empty() {
        return Ok(());
    }

    let mut query_builder: sqlx::QueryBuilder<Postgres> = QueryBuilder::new(
        "INSERT INTO harp.actions (unique_id, ip_address, kind, detail, created)",
    );

    // It's unlikely, but we need to make sure we never have more than the
    // postgres bind limit / struct fields in a single query.
    let limit = POSTGRES_BIND_LIMIT / 5;
    let queue = if queue.len() > limit { queue.drain(..limit) } else { queue.drain(..) };

    tracing::debug!("Logging {} actions", queue.len());
    query_builder.push_values(queue, |mut b, action| {
        b.push_bind(i64::from(action.id))
            .push_bind(action.addr)
            .push_bind(action.kind)
            .push_bind(action.detail)
            .push_bind(action.created);
    });
    let query = query_builder.build();
    query.execute(&*pg).await?;

    Ok(())
}

/// Handles a single connection from an external service. Responsible for
/// parsing incoming messages, converting them into `Action`s, and adding them
/// to the shared queue.
async fn handle_connection(addr: SocketAddr, stream: TcpStream, queue: SharedQueue) -> Result<()> {
    let mut frame = LengthDelimitedCodec::builder().length_field_type::<u16>().new_framed(stream);

    loop {
        tokio::select! {
            result = frame.next() => match result {
                Some(Ok(bytes)) => {
                    let bf = Bufferfish::from(bytes);
                    let action = Action::try_from(bf)?;
                    let mut queue = queue.write().await;

                    // We utilize the `push_within_capacity` and `try_reserve`
                    // to avoid panicking if we would exceed system memory.
                    if let Err(action) = queue.push_within_capacity(action) {
                        tracing::debug!("Queue is full; attempting to resize");

                        if let Err(e) = queue.try_reserve(100) {
                            tracing::error!("Cannot resize queue: {}", e);

                            // We'll reconstruct the Bufferfish from the failing
                            // Action and send it back to the service where it
                            // will be stored in a reserve queue to resend
                            // later.
                            let bf = Bufferfish::try_from(action)?;
                            frame.send(bf.into()).await?;
                        }
                    };

                }
                Some(Err(e)) => {
                    tracing::error!("Error reading from service stream: {}", e);
                    break;
                }
                None => {
                    tracing::info!("Service disconnected: {}", addr);
                    break;
                }
            }
        }
    }

    Ok(())
}
