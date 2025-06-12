FROM rust:1.87-alpine AS chef
USER root
# Add cargo-chef to cache dependencies
RUN apk add --no-cache musl-dev & cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
# Capture info needed to build dependencies
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --bin node-handshake --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin node-handshake


FROM debian:buster-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/node-handshake /usr/local/bin
ENTRYPOINT ["/usr/local/bin/node-handshake"]