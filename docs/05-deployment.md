## A container for our application

### Dockerfiles
- Start with base image (usually OS w/ lang toolchain)

Simplest possible Dockerfile for a Rust project:
```dockerfile
# We use the latest Rust stable release as base image
FROM rust:1.59.0
# Let's switch our working directory to `app` (equivalent to `cd app`)
# The `app` folder will be created for us by Docker in case it does not
# exist already.
WORKDIR /app
# Install the required system dependencies for our linking configuration
RUN apt update && apt install lld clang -y
# Copy all files from our working environment to our Docker image
COPY . .
# Let's build our binary!
# We'll use the release profile to make it faaaast
RUN cargo build --release
 # When `docker run` is executed, launch the binary!
ENTRYPOINT ["./target/release/zero2prod"]
```

Build image:
```bash
docker build --tag zero2prod --file Dockerfile .
```

### Build context

- The only point of contact between the image and local machine are commands like `COPY` or `ADD`
- Using `.` we are telling docker to use the current directory as the build context for this image

### Sqlx offline mode

Running the build command inside the Docker container doesn't work at present because we have no DB running at the port in .env (for sqlx to make compile-time queries)

Options to fix:
- Allow image to talk to a database running on local machine at build time (w/ `--network` flag)
  - Complicated, and affects how reproducable our builds are
- Use "offline mode" for sqlx (saves outcome to `sqlx-data.json`)
  - Add `"offline"` feature to `Cargo.toml`
  - Use sqlx cli `cargo sqlx prepare -- --lib` (`--lib` is piped to cargo command rather than `sqlx`)

### Networking

Map port `8000` from host to port `8000` within container:
```bash
docker run -p 8000:8000 zero2prod
```

This still doesn't work, because we are listening on `127.0.0.1:8000` within the container. Using `0.0.0.0` is ideal, but we don't want to do this during local development (security concerns)

### Hierarchical Configuration

The [config](https://crates.io/crates/config) crate supports reading configuration from a directory rather than a single file

Process:
- Resolve the configuration directory (currentdir + /configuration)
- Add the "base" source
- Add the "[env]" source (based on `APP_ENVIRONMENT` env var)
- Set host to `0.0.0.0` in the production configuration file

### Database connectivity

We still can't connect to the DB at this stage - configure the postgres pool to timeout after 2s rather than 30, and use a "lazy" connection

### Optimising our docker image

Two optimisations:
- Smaller image size
- Layer caching

**Docker image size**
Currently our image is double the size of the base image

- Add .dockerignore (reduce build context)
- Remove source code (2 stages)
  - `builder` stage - generate binary (discarded after build)
  - `runtime` stage - run the binary
- Switch the runtime image
  - Switch to using `rust:1.59.0-slim` (600mb)
    - We don't need the rust toolchain / machinary to run the binary
  - Switch to using `debian:bullseye-slim` (88mb)
- Other useful optimisations - https://github.com/johnthagen/min-sized-rust

**Caching for rust docker builds**

Install deps first, then copy source code over (take advantage of layer caching)
