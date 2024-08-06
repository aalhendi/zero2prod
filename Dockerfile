# Rust stable as base image
FROM rust:1.78.0
# change working dir to `app`. Docker will create if not exist
WORKDIR /app
# linker system dependencies 
RUN apt update && apt install lld clang -y
# Copy all files from our working environment to our Docker image
COPY . .
ENV SQLX_OFFLINE true
# Build
RUN cargo build --release
ENV APP_ENVIRONMENT production
# On `docker run`, launch binary
ENTRYPOINT ["./target/release/zero2prod"]