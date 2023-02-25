# Harp

Harp is a database logging library and service daemon.

The library allows for any Rust application to become a Harp service, running an
"action" processor off-thread which communicates with a designated `harpd`
service.

The `harpd` service provides a resilient, message-style queue for logging
"actions" to a PostgreSQL database via drip-fed, batched transactions.

Harp operates on "actions", which are basically just highly structured messages
with unique IDs and IP addresses.

## Usage

Not yet.

## Configuration

```toml
# harp.toml
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
