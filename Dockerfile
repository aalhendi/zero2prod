FROM lukemathwalker/cargo-chef:latest-rust-1.80.0 as chef
# change working dir to `app`. Docker will create if not exist
WORKDIR /app
# linker system dependencies
RUN apt update && apt install lld clang -y

# [STAGE]: Planner
FROM chef as planner
# Copy all files from our working environment to our Docker image
# NOTE(aalhendi): `COPY . .` will invalidate the cache for the planner container,
# but will not invalidate cache for builder container as long as checksum of recipe.json returned by `cargo chef prepare` does not change
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare --recipe-path recipe.json

# [STAGE]: BUILDER
# Rust stable as base image
FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
# Up to this point, if our dependency tree stays the same,
# all layers should be cached.

COPY . .
ENV SQLX_OFFLINE true
# Build
RUN cargo build --release

# [STAGE]: RUNTIME
FROM debian:bookworm-slim as runtime
# OpenSSL - dynamically linked by some of our dependencies
# ca-certificates - needed to verify TLS certificates when establishing HTTPS connections 
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
# Copy compiled binary from builder to workdir
COPY --from=builder /app/target/release/zero2prod zero2prod
COPY configuration configuration
ENV APP_ENVIRONMENT production
# On `docker run`, launch binary
ENTRYPOINT ["./zero2prod"]