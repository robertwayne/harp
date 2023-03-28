# Harp

Harp is a database logging library and daemon.

The library allows for any Rust application to become a Harp service, running an
"action" processor off-thread which communicates with a designated `harpd`
service.

The `harpd` service provides a resilient, message-style queue for logging
"actions" to a PostgreSQL database via drip-fed, batched transactions.

Harp operates on "actions", which are basically just highly structured messages
with unique IDs and IP addresses.

## Usage

### Daemon

Download _(or build)_ the CLI tool located in `/bin` and run it with the
`-h` flag to see the available options.

```bash
# Runs the daemon with a custom configuration file.
# See the "Configuration" section for more information.
harpd --config /my/harp/config.toml
```

### Service Node

See the [examples](/examples) directory for a complete, well-documented example.
If you're just looking for the bare minimum, here's a quick start example:

```rust no_run
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
    // of "127.0.0.1:7777". The returned value from `create_service()` is the
    // send half of an MPMC channel. This can be cloned cheaply. We'll use this
    // to send actions to the service as it lives in its own task thread.
    let harp = Harp::create_service().await?;

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

```

## Configuration

Harp is configured via a TOML file. A path can be passed via the command-line with the `-c` or `--config` flag. If no path is provided, Harp will attempt to load the configuration from `/etc/harp/config.toml`.

```toml
host = "127.0.0.1"
port = 7777

# Duration in seconds between processing the queue.
# This value cannot be lower than 1.
process_interval = 10

# Maximum packet size (in bytes) to accept per message.
# This value cannot be lower than 128.
max_packet_size = 1024

[database]
name = "harp"
user = "harp"
pass = "harp"
host = "127.0.0.1"
port = 5432

# Maximum number of connections to the database.
# This value cannot be lower than 1.
max_connections = 3
```

## Architecture

`harpd` is designed to be simple and resilient; in an ideal scenario, once you
start it, you can _(mostly)_ forget about it.

The flow of the service looks like this:

1. `harpd` starts up and connects to the database.
2. Two asyncronous tasks are created: a queue processor and a connection
   handler.
   - The connection handler will attempt to decode incoming messages as Harp `Action`s.
   - Successfully decoded messages are added to the queue.
   - The processing task will _(eventually)_ batch-process the actions in a
     single database transaction.

Some notes:

- The service can safely handle invalid messages _(size, decoding, etc.)_ without
  crashing. Connections are dropped by default on failure.
- Messages will be returned to the sender if the queue is full and/or the system
  cannot allocate more memory. The library stores these messages on a reserve
  queue and will slowly retry sending them.
  - If you are interacting with `harpd` without going through the library,
    you must manually handle this case!
- Queries are executed again if the database connection is lost once it has been
  re- established.

## FAQ

### "What about \<insert other logging tool\>?"

I know there are many Logging-as-a-Service and open-source alternatives out
there. I've never used any of them, nor have I invested any time into
researching them. I had very specific and rather simple requirements that I
wanted met, and this was much easier than adding another library and service
into my stack.

Anyway, you're probably better off using one of those in the first place - Harp
is highly opinionated.

### "What about `tracing` or other Rust logging libraries?"

Harp is meant for logging what I call "actions", which require a structured
format, along with an identifier. This is not meant for logging general
messages, errors, or other things which are not tied to a specific item under
these constraints.

You can _(and should)_ use `tracing` _(or others)_ in tandem with Harp, as it
serves a much more general purpose.

### "Why PostgreSQL? Can it support other databases?"

PostgreSQL is my go-to general database, and I like to keep my stack as simple
as possible until there is a reason to add specialized alternatives. As such,
there are no plans to support other options.

## Contributing

Not yet.

__Notes__:

- If you're working on anything in `/bin`, you'll need to enable the `bin`
  feature for cargo to check and build it.
  - If you're using `rust-analyzer`,  add the following to a local settings
file: `"rust-analyzer.cargo.features": "all"`.
- `/bin` requires the use of nightly due to the use of unstable features.

## License

Harp source code is dual-licensed under either

- __[MIT License](/docs/LICENSE-MIT)__
- __[Apache License, Version 2.0](/docs/LICENSE-APACHE)__

at your option.
