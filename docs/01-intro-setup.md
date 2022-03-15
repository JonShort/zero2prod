## Preamble - Cloud-native applications
Cloud-native applications are chosen to:
- To achieve high-availability while running in fault-prone environments;
- To allow us to continuously release new versions with zero downtime;
- To handle dynamic workloads (e.g. request volumes).

Running a replicated application influences our approach to data persistence. Avoid using the local filesystem as primary storage solution, instead rely on databases.

## Installing the Rust toolchain
- rustup [https://rustup.rs/]
- compilation targets [https://forge.rust-lang.org/release/platform-support.html](list of targets here)
- release channels
  - stable (6-weekly release)
  - beta (candidate)
    - Testing on beta is a good way to support the rust project
  - nightly (built from rust master branch) (unstable)

See installed toolchains:
```bash
rustup toolchain list
```

## Project setup
Check rust compiler version:
```bash
rustc --version
```

Interface for dealing with the rust compiler, `cargo`:
```bash
cargo --version
```

Create new project
```bash
cargo new zero2prod
```

## Inner development loop
Loop is:
1. Make a change
1. Compile the application
1. Run tests
1. Run the application

Speed of inner development loop dictates how many loops we can complete in a given amount of time

## Faster Linking
> Linking - assembling the actual binary given the outputs of the earlier compilation stages

Default linker is good, but there are faster alternatives depending on the OS:

- `lld` on Windows & Linux (developed by LLVM project)
- `zld` on MacOS

Installed then configured per-project with the following code:
```toml
# .cargo/config.toml

# On Windows
# ```
# cargo install -f cargo-binutils
# rustup component add llvm-tools-preview
# ```
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
[target.x86_64-pc-windows-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
# On Linux:
# - Ubuntu, `sudo apt-get install lld clang`
# - Arch, `sudo pacman -S lld clang`
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "linker=clang", "-C", "link-arg=-fuse-ld=lld"]
# On MacOS, `brew install michaeleisel/zld/zld`
[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=/usr/local/bin/zld"]
[target.aarch64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=/usr/local/bin/zld"]
```

> JS note - I'm not doing this, `zld` doesn't seem as well-maintained as `lld` (e.g. being debug-only)

## cargo-watch
Watch source code, run chosen commands (faster perceived working)

Install [cargo-watch](https://crates.io/crates/cargo-watch):
```bash
cargo install cargo-watch
```

Run with a single check arg:
```bash
cargo watch -x check
```

Run with chained args (full development loop):
```bash
cargo watch -i docs -x check -x test -x run
```

## Continuous Integration Steps

### Tests
If CI had a single step, it should be testing:
```bash
cargo test
```

`cargo test` also takes care of building the project before running tests, hence you do not need to run `cargo build` beforehand (even though most pipelines will invoke `cargo build` before running tests to cache dependencies).

### Code coverage
Easiest way to measure is to use [tarpaulin](https://github.com/xd009642/tarpaulin):
```bash
# At the time of writing tarpaulin only supports
# x86_64 CPU architectures running Linux.
cargo install cargo-tarpaulin
```

Coverage gathered with:
```bash
cargo tarpaulin --ignore-tests
```

### Linting
Clippy is the official linter, installed with the `rustup` default profile.

Sometimes CI environments use `rustup` minimal profile, which doesn't include clippy - add with:
```bash
rustup component add clippy
```

Run with:
```bash
cargo clippy
```

Fail on warnings (useful for CI):
```bash
cargo clippy -- -D warnings
```

### Formatting
Rustfmt handles automatic code-formatting - also included in `rustup`

Include if missing:
```bash
rustup component add rustfmt
```

Format entire project:
```bash
cargo fmt
```

CI can add a formatting step (fails when a commit contains unformatted code):
```bash
cargo fmt -- --check
```

### Security Vulnerabilities
[Rust Secure Code working group](https://github.com/RustSec) maintain an advisory database of reported vulns published to crates.io

They also provide cargo-audit:
```bash
cargo install cargo-audit
```

Run with:
```bash
cargo audit
```
