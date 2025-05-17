# Reservations

A service for handling time-slot based reservations.

Tech-stack:
- Rust for development
- gRPC for service APIs
- Postgres for data store

## Building
```
$ cargo build
```

## Running Locally

1. Copy `.env.example` to `.env` and properly configure database connection

2. Run with Docker
```
$ docker-compose up --build
```

3. Test with example client
```
$ cargo run --example client
```
