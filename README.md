# Newsletter Email Service

A Rust-based newsletter email service with PostgreSQL backend and OpenTelemetry support.

## Quick Start

1. Start PostgreSQL and Valkey, then run the database migrations:

   ```sh
   docker compose up --detach --wait
   sqlx migrate run
   ```

2. Run the application:

   ```sh
   TEST_LOG=true RUST_LOG=debug cargo run | bunyan
   ```

Stop the local services without deleting PostgreSQL data:

```sh
docker compose down
```

To discard the local database as well, run `docker compose down --volumes`.

## Features

- PostgreSQL database backend
- Valkey-backed session storage
- OpenTelemetry support via OpenObserve (optional)
- Comprehensive test coverage
- CI/CD pipeline with security audit

## Documentation

- [Development Guide](DEVELOPMENT.md) - Detailed setup instructions and development guidelines
- [CI/CD Pipeline](docs/CI_CD.md) - Information about our continuous integration and deployment process

## Building for Production

Build a Docker image:

```sh
docker build --tag zero2prod --file Dockerfile .
```

## License & Notes

- Haven't decided on license yet...
- This project was developed following the principles and practices taught in ["Zero To Production In Rust"](https://www.zero2prod.com/) by Luca Palmieri.
