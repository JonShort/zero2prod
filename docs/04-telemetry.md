## Unknown unknowns

- Test suites are not _proof_ of the correctness of our application
- e.g. there are _known unknowns_ we haven't covered:
  - lose connection to the DB
  - attacker passes malicious payload/s
- but also _unknown unknowns_, cuased from the randomness of "the outside world"
  - system pushed outside of usual operating conditions (e.g. spike of traffic)
  - multiple components experience failures at the same time (e.g. SQL transaction is left hanging while db is going through a master-replica failover)
  - change introduced which moves the system equlibrium
  - no changes for a long time (e.g. memory leak)
  - etc.

## Observability

To prepare for these we need to assume we won't be there when an unknown unknown arises.

We can't debug in prod, so the only thing we can rely on is _telemetry data_

[Honeycomb observability guide](https://www.honeycomb.io/what-is-observability/)

## Logging

### The log crate

The go-to crate for logging in rust is [log](https://docs.rs/log)

`log` provides four macros - [trace](https://docs.rs/log/0.4.11/log/macro.trace.html), [debug](https://docs.rs/log/0.4.11/log/macro.debug.html), [warn](https://docs.rs/log/0.4.11/log/macro.warn.html), [error](https://docs.rs/log/0.4.11/log/macro.error.html)

### actix-web's Logger middleware

Adds a log record for every incoming request - [link](https://docs.rs/actix-web/4.0.1/actix_web/middleware/struct.Logger.html)

### The facade pattern

What should we do with these logs? File? Terminal ? Remote system over HTTP?

The `log` crate uses the [facade pattern](https://en.wikipedia.org/wiki/Facade_pattern) to deal with this:
- Provides a [Log](https://docs.rs/log/0.4.11/log/trait.Log.html) trait
- Consumer calls [set_logger](https://docs.rs/log/0.4.11/log/fn.set_logger.html) and provides an implementation of the `Log` trait
- Every time a log record is emitted `Log::log` will be called

We will use [env_logger](https://docs.rs/env_logger) for now (prints logs to terminal)

## Instrumenting POST /subscriptions

### Interactions with external systems

- Add `log::info` calls at start & end of successful handling; `log::error` at unsuccessful handling
- Use `std::fmt::Debug` formatting within error log - e.g. `log {:?}`

### Think like a user

We need a system which is _sufficiently_ observable. How do we do this?

Re-frame as a user, what does the error look like e.g:
"I tried subscribing to your newsletter with my email XYZ but the website failed with a weird error"

We can search the DB to see if XYZ exists, but we can't see the logs for this issue without additional info from the customer.

In this example we add email + name to the info log starting the request

### Logs must be easy to correlate

Add a correlation id - in this case a UUID

Easy enough for individual route logs, but `actix_web`'s `Logger` middleware doesn't include it

## Structured Logging

Manully adding the correlation id cannot scale

What do we have:
- Overarching task (HTTP request)
- sub-tasks (e.g. parse input, make query, etc.)
- each unit of work has a duration
- each unit of work has a context

Logs are the wrong abstraction

### The tracing crate

link - https://docs.rs/tracing

### Migrating from log to tracing

- Add crate with the `log` feature active
- Convert any `log::` to `tracing::`

### Tracing's Span

[Span](https://docs.rs/tracing/0.1.19/tracing/span/index.html) allows us to better capture the structure of the program

- Accepts key:value pairs instead of a formatted string
  - explicitly name with `a = %a`
  - implictly name with `%a`
- `%` before vars tells `tracing` to use the `Display` implementation for logging
  - [other options](https://docs.rs/tracing/0.1.19/tracing/#recording-fields)

After initialisation, using `.enter()` within sync code makes all logs / spans associate with the span

> This is the rust pattern "Resource Acquisition Is Initialization" (RAII) - more info on this pattern [here](https://doc.rust-lang.org/stable/rust-by-example/scope/raii.html)

When the guard is dropped, then no more association takes place - this is due to a custom `Drop` implemention on the guard

Spans can be entered / exited multiple times - only dropping the guard is final

### Instrumenting Futures

Because each `Future` is polled to completion we can:
- "enter" the span when the future is polled
- "exit" the span when the future is parked

The `tracing` crate exposes [Instrument](https://docs.rs/tracing/latest/tracing/trait.Instrument.html) as an extension trait for futures

### tracing's Subscriber

We need to replace `env_logger` with a `tracing`-native solution

[Subscriber](https://docs.rs/tracing/0.1.19/tracing/trait.Subscriber.html) is the tracing equivalent of `log`'s `Log`, `Subscriber` exposes methods for every stage in the lifecycle of a `Span`

### tracing-subscriber

[tracing-subscriber](https://docs.rs/tracing-subscriber) is another crate maintained by the `tracing` project, providing basic subscribers

`tracing-subscriber` introduces [Layer](https://docs.rs/tracing-subscriber/0.2.12/tracing_subscriber/layer/trait.Layer.html) which allows us to build a processing pipeline for our spans data

We can combine multiple smaller layers to obtain the processing pipeline we need

The layering approach uses [Registry](https://docs.rs/tracing-subscriber/0.2.12/tracing_subscriber/struct.Registry.html), which implements the `Subscriber` trait

### tracing-bunyan-formatter

JSON formatted logs similar to express-bunyan-logger in node land
