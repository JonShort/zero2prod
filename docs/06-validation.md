## Reject Invalid Subscribers #1

Our input validation for POST /subscriptions is too permissive

Add a new integration test to probe the API with some "troublesome" inputs

### Validating name

It's not realistic to validate the name contents, but we need some validation to ensure there's a security layer here

What we can do:
- Enforce a maximum length of 256 characters
  - We're using the TEXT type for name in Postgres, virtually unbounded (bounded by disk storage).
- Reject names containing troublesome characters
  - e.g. `/()"<>\{}` is unlikely to be included in a real-life name

### First implementation

Could add a `is_valid_name` method to the `subscribe` method, but then we'd have to repeat this validation in `insert_subscriber`.

The issue is `is_valid_name` is a _validation function_, but the information around input data is not stored anywhere, we have to do costly point-in-time checks.

We need something that accepts unstructured input, and returns structured output.

### Type-driven development

Adding new "domain" module, with struct `SubscriberName(string)`

`SubscriberName` is a tuple struct, with single unnamed string field

A completely new type, does not inherit any of the methods available on `String`

Only allow consumers to create a new `SubscriberName` using `SubscriberName::parse(String)` method

Having a mandatory type like follows the "parse, don't validate" idea, meaning that we have certain guarantees around the data that we wouldn't have with a more generic type

**Options for allowing consumers access to the inner string:**

Expose by value (consumes struct):

```rust
pub fn inner(self) -> String {
  self.0
}
```

Expose mutable reference:

```rust
pub fn inner_mut(&mut self) -> &mut str {
  &mut self.0
}
```

Expose a shared reference:

```rust
pub fn inner_ref(&self) -> &str {
    &self.0
}
```

`inner_ref` fits our use-case best

### AsRef

Rust exposes a trait which is exactly what we've implemented with `inner_ref`, so we can use that.

### Error as values - Result

Convert the `SubscriberName::parse()` method to return a `Result`

Alter the `panic!()` on error to instead return an `Err()` with string

### Handling a result

`match` clause can handle both scenarios with different logic

```rust
let name = match SubscriberName::parse(form.0.name) {
  Ok(name) => name,
  Err(_) => return HttpResponse::BadRequest().finish(),
}
```

### The email format

Fool's errand to try write our own email validation, we'll use the [`validator`](https://crates.io/crates/validator) crate

Use the same strategy as `SubscriberName`, we'll create `SubscriberEmail`

**First break apart the `domain` module**

```
domain/
  mod.rs
  subscriber_name.rs
  subscriber_email.rs
  new_subscriber.rs
```

Rely on `validator::validate_email` to handle the heavy-lifting.

Our test-cases are all sadpath so we need tests covering the happypath.

Just checking one email is validated correctly is not good enough

### Property-based testing

Instead of verifying that a certain set of inputs is correctly parsed, we could build a random generator

Verify that the implementation displays a certain _property_, e.g. "No valid email address is rejected"

If working with time (H:M:S), this would be to repeatedly sample three random integers:

- `H` between `0` and `23` (inclusive)
- `M` between `0` and `59` (inclusive)
- `S` between `0` and `59` (inclusive)

**Property-based testing for SubscriberEmail**

Use `fake` crate rather than writing our own random generator

```rust
use fake::faker::internet::en::SafeEmail;
use fake::Fake;
```

We'll use [`quickcheck`](https://crates.io/crates/quickcheck) for easy property-based testing

Includes a macro which will run a certain amount of times (100 by default)
```rust
#[quickcheck_macros::quickcheck]
fn valid_emails_are_parsed_successfully(valid_email: String) -> bool {
  SubscriberEmail::parse(valid_email).is_ok()
}
```

Cannot use above as any random string will fail validation.

`quickcheck` and `fake` don't nicely anymore, either pin versions to those mentioned in book or use new [rng-based version](https://github.com/LukeMathWalker/zero-to-production/issues/34#issuecomment-1367346680):

```rust
use rand::{rngs::StdRng, SeedableRng};

//[ ... ]

impl Arbitrary for ValidEmailFixture {
    fn arbitrary(g: &mut Gen) -> Self {
        let mut rng = StdRng::seed_from_u64(u64::arbitrary(g));
        let email = SafeEmail().fake_with_rng(&mut rng);
        Self(email)
    }
}
```

### Payload Validation

We now need to actually use the `SubscriberEmail` struct

Same process here as `SubscriberName`

**Refactoring with TryFrom**

Extract name / email retrieval into `parse_subscriber` function.

We can use the `TryFrom` trait to deal with the failable conversion between two types (that consumes input)

This gives us `try_into()` and `try_from()` methods on our struct, e.g:

```rust
let new_subscriber = match form.0.try_into() {
    Ok(ns) => ns,
    Err(_) => return HttpResponse::BadRequest().finish(),
};
```
