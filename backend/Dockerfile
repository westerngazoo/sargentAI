# syntax=docker/dockerfile:1.7

ARG RUST_VERSION=1.95

FROM rust:${RUST_VERSION}-slim-bookworm AS builder
WORKDIR /src
# pkg-config + libssl-dev: required by openssl-sys (native-tls ← ureq ← ort-sys).
# ca-certificates: required for ort-sys to download the ONNX Runtime binary from GitHub.
# libstdc++-12-dev: the final link step needs libstdc++ because ONNX Runtime is a C++ library.
RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        ca-certificates \
        libstdc++-12-dev \
    && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --release -p fitai-api --locked
# Harvest the ONNX Runtime shared library.
# ort-sys caches the download under /root/.cache/ort.pyke.io/dfbin/ (not target/).
RUN mkdir /ort-libs && \
    find /root/.cache/ort.pyke.io -name 'libonnxruntime*.so*' -exec cp -L {} /ort-libs/ \; 2>/dev/null || true; \
    find target/release/build -name 'libonnxruntime*.so*' -exec cp -L {} /ort-libs/ \; 2>/dev/null || true; \
    test "$(ls -A /ort-libs)" || (echo "ERROR: libonnxruntime not found after build" >&2 && exit 1)

FROM debian:bookworm-slim
# libgomp1: OpenMP runtime required by ONNX Runtime.
# libstdc++6: C++ runtime required by libonnxruntime.so at startup.
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        libgomp1 \
        libstdc++6 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 10001 app
COPY --from=builder /ort-libs/ /usr/local/lib/
COPY --from=builder /src/target/release/fitai-api /usr/local/bin/fitai-api
RUN ldconfig
USER app
EXPOSE 8080
ENV PORT=8080
CMD ["fitai-api"]
