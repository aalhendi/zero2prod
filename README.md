# Newsletter Email something

## Developing

### Install [Rust](https://www.rust-lang.org/tools/install)

Rust and its tooling are required to build this project. Installing via `rustup` does this all in one command.

### Install [mold](https://github.com/rui314/mold)

This project uses `mold` as the linker. You may disable this by modifying the `Cargo.toml` file.

### Install [cargo-watch](https://crates.io/crates/cargo-watch)

`cargo-watch` speeds up the iteration speed by triggering commands whenever a file is changed.

### Install [tarpaulin](https://github.com/xd009642/tarpaulin)

NOTE: unsure if this supports ARM or just x86_64
`cargo tarpaulin --ignore-tests` computes code coverage for application code, ignoring test functions.

### Install [cargo-audit](https://crates.io/crates/cargo-audit)

Checks against [RustSec/advisory-db](https://github.com/RustSec/advisory-db) for any reported vunerabilities.

### Install [sqlx](https://crates.io/crates/sqlx-cli/)

```sh
cargo install --version="~0.7" sqlx-cli --no-default-features --features rustls,postgres
```

We run a prepare command to generate query metadata to support offline compile-time verification.
A check for this is automatically run within the CI pipeline.

```sh
cargo sqlx prepare --workspace
```

### Intall [PostgreSQL](https://www.postgresql.org/)

The DB of choice for this project.

### Install [Bunyan](https://crates.io/crates/bunyan)

```sh
cargo install --locked bunyan
```

## To Run The App

```sh
./scripts/init_db.sh
./scripts/init_redis.sh

TEST_LOG=true RUST_LOG=debug cargo run | bunyan
```

## Building An Image

```sh
docker build --tag zero2prod --file Dockerfile .
```

### Notes

Since tests are run as part of a single binary, you may run into some problems with too many open files in Linux (default 1024).
There is a limit to the number of maximum number of open file descriptors (including sockets) for each process.

```sh
thread 'actix-rt:worker' panicked at
'Can not create Runtime: Os { code: 24, kind: Other, message: "Too many open files" }',
```

```sh
ulimit -n <number>
```
