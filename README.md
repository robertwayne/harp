# Harp

Harp is a database logging library and daemon.

The library allows for any Rust application to become a Harp service, running an
"action" processor off-thread which communicates with a designated `harpd`
service.

The `harpd` service provides a resilient, message-style queue for logging
"actions" to a PostgreSQL database via drip-fed, batched transactions.

Harp operates on "actions", which are basically just highly structured messages
with unique IDs and IP addresses.

## Motivation

Harp was designed for use with online games, and thus has a few specific
requirements. Specifically, items to be logged _(known as targets)_ must
implement the `Loggable` trait, which requires that the item has an IP address
and a unique ID.

The original use-case was for saving player actions to a database; as I maintain
a few game servers, I wanted it to be easy to integrate this kind of logging
across them all without having to write a bunch of boilerplate code.

## Usage

Not yet.

## Configuration

Not yet.

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

Harp is meant for logging what I call "Actions", which require a structured
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

## License

Harp source code is dual-licensed under either

- **[MIT License](/docs/LICENSE-MIT)**
- **[Apache License, Version 2.0](/docs/LICENSE-APACHE)**

at your option.
