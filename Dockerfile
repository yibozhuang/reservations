FROM rust:1.87-slim-bullseye AS builder
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    protobuf-compiler \
    libssl-dev \
    libprotobuf-dev \
    pkg-config

WORKDIR /app
ENV PROTOC_INCLUDE=/usr/include

COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/reservations /app/reservations
COPY --from=builder /app/db /app/db

ENV RUST_LOG=info
EXPOSE 50051
CMD ["./reservations"]
