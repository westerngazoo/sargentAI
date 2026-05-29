# syntax=docker/dockerfile:1.7

ARG RUST_VERSION=1.95

FROM rust:${RUST_VERSION}-slim-bookworm AS builder
WORKDIR /src
COPY . .
RUN cargo build --release -p fitai-api --locked

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 10001 app
USER app
COPY --from=builder /src/target/release/fitai-api /usr/local/bin/fitai-api
EXPOSE 8080
ENV PORT=8080
CMD ["fitai-api"]
