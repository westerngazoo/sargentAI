# syntax=docker/dockerfile:1.7

ARG RUST_VERSION=1.95

FROM rust:${RUST_VERSION}-slim-bookworm AS builder
WORKDIR /src
# pkg-config + libssl-dev: required by openssl-sys (pulled in by ort's download-binaries feature).
# ca-certificates: required for ort to download the ONNX Runtime binary from GitHub.
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --release -p fitai-api --locked
# Harvest the ONNX Runtime shared library from wherever ort placed it after download.
RUN mkdir /ort-libs && \
    find target/release/build -name 'libonnxruntime*.so*' -exec cp {} /ort-libs/ \; && \
    test "$(ls -A /ort-libs)" || (echo "ERROR: libonnxruntime not found after build" >&2 && exit 1)

FROM debian:bookworm-slim
# libgomp1: OpenMP runtime required by ONNX Runtime at startup.
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        libgomp1 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 10001 app
COPY --from=builder /ort-libs/ /usr/local/lib/
COPY --from=builder /src/target/release/fitai-api /usr/local/bin/fitai-api
RUN ldconfig
USER app
EXPOSE 8080
ENV PORT=8080
CMD ["fitai-api"]
