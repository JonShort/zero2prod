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
