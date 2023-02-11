## Confirmation Emails

### Subscriber consent

We now ensure that saved emails are _syntactically_ correct, but we need consent from the email address holder.

E.g. in EU it's a legal requirement to get consent

Flow:
- Sign up
  - User added to DB with
    - status of _pending_confirmation_
    - a _subscription_token_ added to table alongside their ID
- Confirmation email sent
- User confirms via. email
  - Link within email to `https://<api-domain>/subscriptions/confirm?token=<subscription_token>`

On clicking the link browser tab opened which triggers a GET to our `/subscriptions/confirm` endpoint

**Implementation**

- Module to send email
- Adapt POST `/subscriptions` to match new specifications
- Add a GET handler for `/subscriptions/confirm`

### Email delivery

Examples of email API providers on the market - AWS SES, SendGrid, MailGun, Mailchimp, Postmark

[Postmark](https://postmarkapp.com) is choice for this course

**Designing our interface**

Some kind of async `send_email` method which accepts arguments

Argments we will need:
- Recipient email address
- Subject Line
- Email content (HTML and plaintext)

Sender address can live in the client constructor (unchanging)

### REST client with reqwest

Lots of options, but [`reqwest`](https://crates.io/crates/reqwest) is the most popular crate

Features of `reqwest`:
- Battle-tested
- Primarily async, but with synchronous option
- Relies on `tokio` as async executor (we're already using this)
- Does not depend on a system library

Create new client with `Client::new` or `Client::builder`

Connecting is an expensive operation, especially with HTTPS. HTTP clients mitigate this with connection pooling, after the first request to a remote server is executed, keep the connection open (for certain length of time) and re-use it.

We need to use the same client across multiple requests to take advantage of this.

We can do this by adding it to the application context.

Two options
- Derive the clone trait on the email client, and pass to startup
- Wrap in Arc using `actix_web::web::Data::new()`
  - This preferred because we have a few peices of data that we're initialising our client with

Of note - we create an app instance for each thread, which might be of consideration in some scenarios.

### How to test a REST client

Integration tests at this stage would be time-consuming.

Instead test `EmailClient` in isolation.

**HTTP mocking with `wiremock`**

`MockServer` - An actual HTTP server

We get the address of the mock server with `mock_server.uri()` and pass it to our `EmailClient` as `base_url`

Server returns 404 by default, consumer can chain matchers with specific responses, e.g.

```rust
Mock::given(any())
  .respond_with(ResponseTemplate::new(200))
  .expect(1)
  .mount(&mock_server)
  .await;
```

Expect is part of the mock repsonse rather than at the end of a test.

Assertions checked when `MockServer` goes out of scope.

**Adding `send_email` method**

Postmark example request:

```bash
curl "https://api.postmarkapp.com/email" \
  -X POST \
  -H "Accept: application/json" \
  -H "Content-Type: application/json" \
  -H "X-Postmark-Server-Token: server token" \
  -d '{
    "From": "sender@example.com",
    "To": "receiver@example.com",
    "Subject": "Postmark test",
    "TextBody": "Hello dear Postmark user.",
    "HtmlBody": "<html><body><strong>Hello</strong> dear Postmark user.</body></html>"
  }'
```

So we need to:
- POST request to `/email`
- JSON body (field names must be PascalCase)
- Auth header w/ token from Postmark portal

Postmark example response:

```
HTTP/1.1 200 OK
Content-Type: application/json

{
  "To": "receiver@example.com",
  "SubmittedAt": "2021-01-12T07:25:01.4178645-05:00",
  "MessageID": "0a129aee-e1cd-480d-b08d-4f48548ff48d",
  "ErrorCode": 0,
  "Message": "OK"
}
```

Added postmark auth token to email client, provided in config yaml in dev, but will be provided as a secret in prod.

Within our `send_email` abstraction we're using reqwest builder:

```rust
  self.http_client
    .post(&url)
    .header(
        "X-Postmark-Server-Token",
        self.authorization_token.expose_secret(),
    )
    .json(&request_body)
    .send()
    .await?;
```

We add a custom matcher by implementing `wiremock::Match` to our matcher struct, and adding it to the builder pattern.

Just need to ensure that the JSON body has PascalCase keys, can be handled by serde with the macro `[serde(rename_all = "PascalCase")]`.

**Dealing with failures**

Two scenarios to cover:
- non-success status codes (4xx, 5xx, etc.)
- slow responses

`reqwest` by default returns `Ok(())` for any request which receives a response (regardless of status code).

We need to use `error_for_status` to make `reqwest` return an `Err` when non-ok status codes are returned.

**Timeouts**

Any IO operation should have a timeout, for `reqwest` we can either set a per-request or client-wide timeout.

We will set a client-wide timeout:

```rust
let http_client = Client::builder()
  .timeout(std::time::Duration::from_secs(10))
  .build()
  .unwrap();
```

Hardcoded 10 seconds is not ideal for unit tests, instead we'll make the timeout configurable.

New argument on the `EmailClient::new()`

Pass via. configuration, so in `base.yaml` we configure it to 10000ms, and in the unit tests we can directly pass `std::time::Duration::from_millis(200)` for speedy unit tests.

### Skeleton and principles for a maintainable test suite

Options for sharing test helpers:

- Stand-alone module (`tests/helpers/mod.rs`)
  - Can lead to annoying "function is never used warnings" (e.g. if test only uses a subset of helpers)
- Sub-modules scoped to a single test executable (`tests/api/main.rs`)
  - Benefit that we can structure the code in the same way as a binary crate

> Note - `main` method not needed in a test binary

New structure
```
tests/
  api/
    health_check.rs
    helpers.rs
    main.rs
    subscriptions.rs
```

`main.rs` includes only

```rust
mod health_check;
mod helpers;
mod subscriptions;
```

> If seeing errors around linking with the test suite (e.g. "Too many open files"), can alter the limit of open file descriptors with `ulimit -n 10000`

**Sharing startup logic**

Startup code is duplicated in integration tests, and itself never tested. There's a danger they could diverge.

Move server setup logic in to a public `build` function in `startup.rs`.

Use this in `src/main.rs` and in `tests/api/helpers.rs`

Refactor most of the startup into chunks for better testing.

### Build an API Client

In the subscriptions tests we duplicate a lot of the calling code, we should separate this out into a method on the `TestApp` struct

```rust
impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}
```

## Zero Downtime Deployments

Options:

- Naive Deployment (downtime)
  - Turn off A
  - Spin up B
  - B now serving traffic
- Load Balancers (dynamic backends)
  - Rolling update strat

Digital Ocean App Platform offer zero-downtime deployments, not stated how but experiments seem to indicate rolling update strategy.

## Database Migrations

**State is kept outside the application**

Our applications being stateless allows load-balancer to work as all state is shared amongst instances (e.g. DB)

**Deployments and migrations**

Rolling update means both new and old code are using the same database _at the same time_.

DB schema should be understood by both versions of the application.

To add confirmation emails, we need to evolve our schema to:
- Add a new table `subscription_tokens`
- Add mandatory column, `status`, to existing table

Options:

- Update DB first
  - Existing code doesn't know about `status`, which would cause failed requests as it is mandatory
- Deploy then update
  - New version running against old schema, `status` inserts will fail

**Multi-step migrations**
We can't evolve the DB schema and change application behaviour at the same time

### DB Updates

**A new mandatory column**

Generate new migration:
```bash
sqlx migrate add add_status_to_subscriptions
```

Then add the following to the generated file:

```sql
ALTER TABLE subscriptions ADD COLUMN status TEXT NULL;
```

Run migration locally and against prod DB.

**Start using the new column**

Change insertion query to include status of 'confirmed'

**Backfill and mark as NOT NULL**

Create a new migration:

```bash
sqlx migrate add make_status_not_null_in_subscriptions
```

Add a transaction to the migration:

```sql
-- We wrap the whole migration in a transaction to make sure
-- it succeeds or fails atomically. We will discuss SQL transactions
-- in more details towards the end of this chapter!
-- `sqlx` does not do it automatically for us.
BEGIN;
  -- Backfill `status` for historical entries
  UPDATE subscriptions
    SET status = 'confirmed'
    WHERE status IS NULL;
  -- Make `status` mandatory
  ALTER TABLE subscriptions ALTER COLUMN status SET NOT NULL;
COMMIT;
```

**A new table**

Adding `subscription_tokens` table doesn't require the three-step approach, we can just add it first, then start using it

```bash
sqlx migrate add create_subscription_tokens_table
```

Then add table with mandatory fields:

```sql
-- Create Subscription Tokens Table
CREATE TABLE subscription_tokens(
  subscription_token TEXT NOT NULL,
  subscriber_id uuid NOT NULL
    REFERENCES subscriptions (id),
  PRIMARY KEY (subscription_token)
);
```

## Sending a confirmation email

DB is now ready to accomodate confirmation emails.

### A static email

Checking that _an email_ is sent

Update `TestApp` to include a mocked email server, using `wiremock::MockServer`

```rust
Mock::given(path("/email"))
    .and(method("POST"))
    .respond_with(ResponseTemplate::new(200))
    .expect(1)
    .mount(&app.email_server)
    .await;
```

### A static confirmation link

Scan body of the email for a confirmation link

**TDD**

Grab the request received by the mocked email server using the `received_requests()` method.

```rust
let email_request = &app.email_server.received_requests().await.unwrap()[0];
```

Extract links with `linkify` dev dependency.

```rust
let get_link = |s: &str| {
  let links: Vec<_> = linkify::LinkFinder::new()
    .links(s)
    .filter(|l| *l.kind() == linkify::LinkKind::Url)
    .collect();

  assert_eq!(links.len(), 1);

  links[0].as_str().to_owned()
};
```

### Skeleton of GET /subscriptions/confirm

Setup the GET endpoint under subscriptions_confirm.

Typing the query with `web::Query<T>` will automatically return a 400 if the extraction into `T` fails

Just a load of test writing and refactoring here...

### Subscription Tokens

Update `send_confirmation_email` to accept subscription token as an argument

The subscription tokens do not grant access to protected information, but need to be hard to guess

We can use a Cryptographically secure pseudo-random number generator (CSPRING)

We'll add `rand` as a dependency

```rust
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

/// Generate a random 25-characters-long case-sensitive subscription token.
fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
```

## Database Transactions

### All or nothing

Implementation at this point does two INSERT queries agains the Postgres database:
- Store details of the new subscriber
- Store the newly-generated subscription token

What happens if the application crashes between those two operations?

We have three possible states:
- a new subscriber and its token have been persisted
- a new subscriber has been persisted, without a token
- nothing has been persisted

More queries = more difficulty reasoning about the state of the database

Transactions can mitigate this issue - grouping operations as a single _unit of work_

DB guarantees that all operations within a transaction will succeed or fail together

### Transactions in Postgres

Start with a `BEGIN` statement, finalise with a `COMMIT` statement, e.g.

```sql
BEGIN;
UPDATE subscriptions SET status = 'confirmed' WHERE status IS NULL;
ALTER TABLE subscriptions ALTER COLUMN status SET NOT NULL;
COMMIT;
```

If any queries fail within the transaction the DB rolls back (can also explicitly trigger a rollback with `ROLLBACK` statement)

`isolation level` of transactions needs to be considered to fine-tune concurrency guarantees provided by the DB

[Designing-Data-Intensive-Applications](https://www.amazon.co.uk/Designing-Data-Intensive-Applications-Reliable-Maintainable/dp/1449373321) is a good resource for this knowledge

### Transactions in Sqlx

`sqlx` provides a dedicated API for transactions - started with `pool.begin()`

`begin()` returns a `Transaction` struct, this implements `sqlx`'s `Executor` trait so can run queries. All queries run using this become part of the transaction

Because transactions need to commit or rollback, by default if not committed the transaction will rollback (using Drop impl)

```rust
if transaction.commit().await.is_err() {
    return HttpResponse::InternalServerError().finish();
}
```

FINALLY FINISHED
