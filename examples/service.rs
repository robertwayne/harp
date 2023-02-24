/// In order to replicate this example on your own, you will need to include
/// both `tokio`, `harp`, and `serde_json` in your `Cargo.toml`.
///
/// ```toml
/// [dependencies]
/// tokio = { version = "1.0", features = ["full"] }
/// harp = { git = "https://github.com/robertwayne/harp" }
/// ```
///
/// Additionally, you will need to be running a PostgreSQL database with the URL
/// `postgres://harp:harp@localhost:5432/harp`.
///
/// Finally, you will need to have an instance of `harpd` running. A pre-built
/// binary from the releases page or built from the source code can be used.
///
/// This is a WIP example, and this process will be simplified and automated in
/// the near future.
use std::{
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};

use harp::{
    action::{Action, Kind},
    Harp, HarpId, Loggable,
};
use serde_json::json;

// We'll define our action kind as an enum for type safety. A kind can be
// represented by any string type, however.
pub enum ActionKind {
    PlayerJoin,
    PlayerLeave,
}

// We also need to implement the `Kind` trait for our enum. This requires that
// we implement the `key()` method, which will return a string representation of
// the action. This string will be stored in the database, so think about how
// you'd like to have your action kinds represented.
impl Kind for ActionKind {
    fn key(&self) -> &'static str {
        match self {
            ActionKind::PlayerJoin => "player_join",
            ActionKind::PlayerLeave => "player_leave",
        }
    }
}

// We'll define a simple struct to represent a player.
pub struct Player {
    pub id: u32,
    pub ip: IpAddr,
}

// We'll implement the `Loggable` trait for our `Player` struct. This trait
// requires the `identifier()` method, which means we must return a tuple
// containing the IP address and some unique ID - in this case, we have a player
// ID.
//
// Currently, `Loggable` requires the ID be represented as a u32. This will
// ideally be changed to be more generic in the future.
impl Loggable for Player {
    fn identifier(&self) -> HarpId {
        (self.ip, self.id)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // We'll create a fake player. In a real application, you'd assign the IP
    // from the underlying stream. Additionally, you'd want unique IDs.
    let player = Player { id: 1, ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)) };

    // Create and connect to a Harp server using the default hostname and port
    // of "127.0.0.1:7777". The returned value from `create_service!()` macro is
    // the send half of an MPMC channel. This can be cloned cheaply. We'll use
    // this to send actions to the service as it lives in its own task thread.
    let harp = harp::create_service!();

    // We'll tick every second, just to simulate some actions quickly.
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // We can construct a couple of fake actions.
                //One without detail...
                let action = Action::new(ActionKind::PlayerJoin, &player);

                // ...and one with detail.
                // Note the detail field must be a `serde_json::Value`.
                let action2 = Action::with_detail(ActionKind::PlayerLeave, json!({ "reason": "lost connection"}), &player);

                // We can then send them using the send channel we got earlier.
                // Remember, the service is on another task, which means it
                // could be on another thread, so using this send half to pass
                // the action is required.
                harp.send(action)?;
                harp.send(action2)?;
            }
        }
    }
}
