# Development Guide

This guide covers the setup and development process for the newsletter email service.

## Prerequisites

### Required Tools

1. **Rust and Cargo**

   - Install via [rustup](https://www.rust-lang.org/tools/install)

2. **Docker with Docker Compose**

   - Install [Docker](https://docs.docker.com/get-docker/)

3. **Mold Linker**
   - Install [mold](https://github.com/rui314/mold)
   - Note: This requirement may be removed in the future when Rust switches to `lld` by default or via changing the `.cargo/config.toml` file.

### Development Tools

Install these tools for an optimal development experience:

```sh
# Watch for file changes and rebuild
cargo install cargo-watch

# Code coverage reporting
cargo install cargo-llvm-cov

# Security auditing
cargo install cargo-audit

# Database tooling
cargo install --version="~0.8" sqlx-cli --no-default-features --features rustls,postgres

# Log formatting
cargo install --locked bunyan

# Detect unused dependencies
cargo install cargo-udeps
```

## Database Setup

### Prepare Database Queries

Run this command to generate query metadata for offline compile-time verification:

```sh
cargo sqlx prepare --workspace
```

This preparation is checked in the CI pipeline.

## Development Workflow

1. Start PostgreSQL and Redis and wait for them to become healthy:

   ```sh
   docker compose up --detach --wait
   ```

2. Apply pending database migrations:

   ```sh
   sqlx migrate run
   ```

3. Run the application with logging:

   ```sh
   TEST_LOG=true RUST_LOG=debug cargo run | bunyan
   ```

## Testing

### Running Tests

Note: On Linux systems, you might encounter file descriptor limits (default 1024) when running tests. If you see errors like:

```
thread 'actix-rt:worker' panicked at
'Can not create Runtime: Os { code: 24, kind: Other, message: "Too many open files" }',
```

Increase the limit using:

```sh
ulimit -n 10000 # or whatever number
```

### Code Coverage

Generate code coverage reports:

```sh
# Generate coverage data
cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info

# Generate HTML report
cargo llvm-cov report --html --output-dir coverage
```

## Security

Run security audits against the [RustSec Advisory Database](https://github.com/RustSec/advisory-db):

```sh
cargo audit
```

## OpenTelemetry Integration

OpenTelemetry support is available as an optional feature. To use it:

### Local Setup

1. Start OpenObserve alongside the other local services:

   ```sh
   docker compose --profile observability up --detach --wait
   ```

   OpenObserve is available at <http://localhost:5080> using the local development
   credentials in `compose.yaml`. Its data is stored in the `openobserve_data` Docker
   volume.

2. Verify that OpenObserve is ready:

   ```sh
   curl --fail http://localhost:5080/healthz
   ```

3. Run the application with OpenTelemetry enabled:

   ```sh
   cargo run --features "open-telemetry"
   ```
