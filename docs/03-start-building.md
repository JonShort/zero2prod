## What are we implementing?

**As a** blog visitor  
**I want** to subscribe to the newsletter  
**So that** I can receive email updates when new content is published on the blog

1. input email in form
1. API call to backend server `/subscriptions` POST endpoint
1. process info & store
1. response from server

## Strategy

1. Choose web framework
1. Testing strategy
1. Crate to interact with DB
1. DB migrations
1. Write queries

We will start by implementing a `/health_check` endpoint - relying on the CI pipeline to keep us in check

## Choosing web framework

Choices:

- `actix-web`
- `tide`
- `rocket`
- `warp`

Choice is `actix-web` (prod-tested, built on tokio)

- [actix-web’s website](https://actix.rs/)
- [actix-web’s documentation](https://docs.rs/actix-web/4.0.1/actix_web/index.html)
- [actix-web’s examples collection](https://github.com/actix/examples)

## First endpoint

GET `/health_check` should return 200 OK (no body)

We can use to verify API is up and ready to receive requests. Can pair with SaaS service e.g. [pingdom](https://www.pingdom.com/) to monitor

Also useful for container orchestration, detect unresponsive API

### Wiring up actix-web

- Use hello world example from [actix-web’s website](https://actix.rs/)
- Hit with curl

### Anatomy of an actix-web application

- `HttpServer`, handles transport level concerns
  - where should API be listening for incoming requests?
  - Max number of connections
  - Enable TLS?
  - etc...
- `App`, logic
  - routing
  - middlewares
  - request handlers
  - etc...
- Endpoint - `.route()`
  - arg1 - path can be dynamic e.g. `/{name}`
  - arg2 - route instance of `Route` struct, combines handler & guards
    - Guards are implementors of the `Guard` trait, must satisfy in order to allow request to be passed to handler
  - example: `web::get().to(greet)`
    - `web::get()` short for `Route::new().guard(guard::Get())` aka. only GET requests are allowed
    - handler (`greet`) takes `HttpRequest` and returns _something_ which implements `Responder` trait
      - Although `actix-web` does allow different func signatures for handlers
- Runtime (`tokio`)
  - Rust doesn't include async runtime, must be included as a dependency
  - `Future` trait has `poll` method, e.g. "is value here yet"
  - `cargo-expand` can show what macros are actually doing, in this case it gives us the boilerplate to run synchronous main which really provides our async main to tokio

### Implementing health check

- Ensure returned value implements `Responder`
- Can use `.route(path, web::get().to(factory))` or `.service(factory)`
- check with: `curl -v http://127.0.0.1:8080/health_check`

## Integration testing

### How do you test an endpoint

_Black box testing_ is effective for APIs, helps prevent breaking changes leaking to prod. Verify the behaviour of a system by examining its output given set of inputs (no internal implementation info available)

We will launch our API and interact using off-the-shelf HTTP client (e.g. [reqwest](https://docs.rs/reqwest/0.11.0/reqwest/))

### Where should I put my tests?

- embedded test module (next to code)
  - e.g. `mod tests`
  - Privileged access to the code living beside
- in external `tests/` folder
  - Compiled to separate binary
- as part of public docs
  - e.g. `assert!()` in doc comment above method
  - Compiled to separate binary

Decision for this project - `tests/` folder

### Changing project structure for easier testing

Refactor project into library and binary:

- Library - all logic
- Binary - entrypoint with slim `main` function

### Implementing the integration test

We need:

- Method to start the server
- Decoupled assertions on actual calls to the API

We will start the server with a `spawn_app` method; which requires a refactor to the `zero2prod::run` signature.

- Return a actix_web `Server` (or std::io::Error)
- No longer awaits the `Server` setup
- Synchronous

Now start the server as a background task with a synchronous `spawn_app()`, that starts the server with a `tokio::spawn(server)`

### Polishing

**check that tests teardown properly**
The server startup is not leaky because when the `tokio` runtime is shut down all the spawned tasks are dropped (`tokio::test` spins up a new runtime at the start of each test case)

**what if port 8080 is in-use?**

- Could fail if we're running something else on that port
- Could fail if tests are ran in parallel

Tests should run on a random, available, port (alter `zero2prod::run` to accept address as an arg, then use port 0)

## Working with HTML Forms

### Refining our requirements

- What info? Email address
- Body of POST request
- `application/x-www-form-urlencoded` fits our use-case

Requirements summary

- Valid name + email -> 200 OK
- name or email missing -> 400 BAD REQUEST

### Capturing our requirements as tests

- success test just involves setting the data for a valid request
- failure test is an example of a [table-driven test](https://github.com/golang/go/wiki/TableDrivenTests)
  - Multiple failure conditions are tested, preventing us from duplicating logic
  - Must have a good failure message (understand what is failing)

### Parsing form data from a POST request

Similar to the health_check endpoint, but this time using the `actix_web::post` macro

- Same signature as the GET
- Requires serde (serializer / deserializer for Rust data structures) to handle the JSON
  - Article on serde - [here](https://www.joshmcguigan.com/blog/understanding-serde/)

### Extractors

Info an extractors - [here](https://actix.rs/docs/extractors/)

- Used to "extract" information from the incoming request
- Included is a Form helper which:
  - Checks the `Content-Type` header (must be `application/x-www-form-urlencoded`)
  - Checks that the request body includes expected values + types (struct of expectations is provided)
  - By default returns 400 if these guards do not pass (can be customised)

## Storing data - Databases

Author's rule-of-thumb around DBs:

> If you are uncertain about your persistence requirements, use a relational database. If you have no reason to expect massive scale, use [PostgreSQL](https://www.postgresql.org/).

[PostgreSQL](https://www.postgresql.org/) - a battle-tested piece of technology, widely supported across all cloud providers if you need a managed offering, opensource, exhaustive documentation, easy to run locally and in CI via Docker, well-supported within the Rust ecosystem.

### Choosing a DB crate

As of August 2020, three top-of-mind choices for PostgreSQL:

- [tokio-postgres](https://docs.rs/tokio-postgres/)
- [sqlx](https://docs.rs/sqlx/)
- [diesel](https://docs.rs/diesel/)

How to pick one:

- compile-time safety
- SQL-first vs a DSL for query building
- async vs sync interface

**Compile-time safety**  
When do we realise we've made a mistake (e.g. misspelling in column or table name / expecting a field to exist in returned data)

- `tokio-postgres` handles this at _runtime_
- `sqlx` handles this at _compile time_
  - Uses macros to connect to DB at compile-time and check the provided query is ok
- `diesel` handles this at _compile time_
  - Uses CLI to build a Rust representation of the DB schema - [info](https://docs.diesel.rs/diesel/macro.table.html)

**Query interface**

- `tokio-postgres` expects direct SQL for queries
- `sqlx` expects direct SQL for queries
- `diesel` provides a "query builder" DSL (Domain Specific Language)

- Direct SQL is portable, non-lang-specific
- `diesel` DSL costs upfront, but offers more reusability assuming `diesel` continues to be used

**Async support**

> Threads are for working in parallel, async is for waiting in parallel.

async won't _reduce_ the time it takes to process a query, but it will allow extra work to be done in the meantime.

Separate threadpool should be more then enough for most use-cases, but if your framework is already async it may reduce "headaches" to use async

- `sqlx` - async option
- `tokio-postgres` - async option
- `diesel - sync

> aside - `tokio-postgres` also supports [query pipelining](https://docs.rs/tokio-postgres/0.5.5/tokio_postgres/index.html#pipelining)

**Summary**

| Crate            | Compile-time safety | Query interface | Async |
| ---------------- | ------------------- | --------------- | ----- |
| `tokio-postgres` | No                  | SQL             | Yes   |
| `sqlx`           | Yes                 | SQL             | Yes   |
| `diesel`         | Yes                 | DSL             | No    |

Choice for zero2prod - `sqlx`

### Integration testing with side-effects

- We need to know that the data is being persisted in a success to the POST subscriptions endpoint
  - Option 1: call other public endpoint (ideal)
  - Option 2: directly query the DB

We will go with Option 2 for now as Option 1 requires an additional feature

### Database Setup
- Docker, official postgres image
- bash script written for easier spin-up

**Migrations**

To store the subscribers' details we need to create a table; to do this we need to change the [DB schema](https://www.postgresql.org/docs/9.5/ddl-schemas.html) (commonly referred to as a DB migration)

`sqlx` provides [sqlx-cli](https://crates.io/crates/sqlx-cli) for managing DB migrations

Install with:
```bash
cargo install --version=0.5.11 sqlx-cli --no-default-features --features native-tls,postgres
```

Create DB (not required for our docker, but needed in CI / env)
```bash
sqlx database create
```

Requires `DATABASE_URL` env var, formatted as so:
```
postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}
```

Create migration with name "create_subscriptions_table":
```bash
sqlx migrate add create_subscriptions_table
```

This adds a `migrations/` folder, and includes the stubbed migration

We write SQL to create the table here, using uuid as primary key

>Author's opinion is that synthetic key (no business meaning) is default unless there's a compelling reason

Other rules:
- keeping track of when user subscribed with `subscribed_at`
- `email` must be unique
- all fields must be populated (`NOT NULL`)
- `email` and `name` are `TEXT` because there's no restriction on maximum length

FYI - these rules come at a performance cost, but given our domain it's unlikely we'd feel pain from this

Running migrations:
```bash
sqlx migrate run
```

### Writing our first DB query

Add sqlx as a dependency, include many feature flags:
- `runtime-actix-rustls` tells sqlx to use the actix runtime for its futures and rustls as TLS backend;
- `macros` gives us access to sqlx::query! and sqlx::query_as!, which we will be using exten- sively;
- `postgres` unlocks Postgres-specific functionality (e.g. non-standard SQL types);
- `uuid` adds support for mapping SQL UUIDs to the Uuid type from the uuid crate. We need it
to work with our id column;
- `chrono` adds support for mapping SQL timestamptz to the DateTime<T> type from the chrono
crate. We need it to work with our subscribed_at column;
- `migrate` gives us access to the same functions used under the hood by sqlx-cli to manage
migrations. It will turn out to be useful for our test suite.

**Configuration management**

The [config](https://docs.rs/config/) crate is powerful for managing complex configuration, however not required by us at this stage


