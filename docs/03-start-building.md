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


