# task_ba

A simple Rust application that processes Solana on-chain data and publishes token launch events via RabbitMQ.

## Requirements

- Rust
- Docker & Docker Compose (for services defined in `docker-compose.yml`)

## Quick Start

1. Start dependencies with Docker Compose:
   ```bash
   docker compose up -d
   ```
2. Run the main application:
   ```bash
   cargo run --bin task_ba
   ```
3. (Optional) Run the RabbitMQ consumer in a separate terminal:
   ```bash
   cargo run --bin rabbit_consumer
   ```

## Configuration

Application settings can be adjusted in `config.jsonc` and the Rust modules under `src/config/`.

## References

This project takes inspiration and guidance from the following resources:

- [ValidatorsDAO/solana-stream](https://github.com/ValidatorsDAO/solana-stream) – open-source Solana Stream SDK used to better understand Geyser client patterns.
- [ERPC – Solana Geyser gRPC Quickstart](https://erpc.global/en/doc/geyser-grpc/quickstart/) – official documentation followed to configure and connect to the Geyser gRPC endpoint.
