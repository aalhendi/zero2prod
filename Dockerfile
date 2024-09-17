FROM rust:1.81.0-alpine AS base
# ca-certificates - needed to verify TLS certificates when establishing HTTPS connections 
RUN apk add --no-cache musl-dev ca-certificates 

# change working dir to `app`. Docker will create if not exist
WORKDIR /app
RUN cargo install cargo-chef

# [STAGE]: Planner
FROM base AS planner
# Copy all files from our working environment to our Docker image
# NOTE(aalhendi): `COPY . .` will invalidate the cache for the planner container,
# but will not invalidate cache for builder container as long as checksum of recipe.json returned by `cargo chef prepare` does not change
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare --recipe-path recipe.json

# [STAGE]: BUILDER
# Rust stable as base image
FROM base AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
# Up to this point, if our dependency tree stays the same,
# all layers should be cached.

COPY . .
ENV SQLX_OFFLINE=true
# Build
RUN cargo build --release

# [STAGE]: RUNTIME
FROM scratch AS runtime

WORKDIR /app
# Copy SSL certificates
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
# Copy compiled binary from builder to workdir
COPY --from=builder /app/target/release/zero2prod zero2prod
COPY configuration configuration

ENV APP_ENVIRONMENT=production

# On `docker run`, launch binary
ENTRYPOINT ["./zero2prod"]