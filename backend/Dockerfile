# syntax=docker/dockerfile:1.7
#
# ort-sys (download-binaries) fetches a custom ORT build from ort.pyke.io that
# requires glibc >= 2.38.  Debian Bookworm only ships glibc 2.36, so we use
# Ubuntu 24.04 (glibc 2.39) for both the builder and the runtime stage.

FROM ubuntu:24.04 AS rust-toolchain
# Install the Rust toolchain pinned by rust-toolchain.toml.  We do this in a
# separate stage so the heavy download is cached independently of source changes.
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
        curl \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*
ARG RUST_VERSION=1.95.0
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain ${RUST_VERSION} --profile minimal --no-modify-path \
    && /root/.cargo/bin/rustup component add rustfmt clippy
ENV PATH="/root/.cargo/bin:$PATH"

FROM rust-toolchain AS builder
WORKDIR /src
# pkg-config + libssl-dev: openssl-sys (native-tls ← ureq ← ort-sys).
# libstdc++-14-dev: Ubuntu 24.04 ships GCC 14; the link step needs the unversioned
#   libstdc++.so stub because ONNX Runtime is a C++ library.
RUN apt-get update && apt-get install -y --no-install-recommends \
        gcc \
        pkg-config \
        libssl-dev \
        libstdc++-14-dev \
    && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --release -p fitai-api --locked
# ort-sys caches the ORT binary under /root/.cache/ort.pyke.io/dfbin/.
# Gather every libonnxruntime* there (and fall back to target/ for future changes).
RUN mkdir /ort-libs && \
    find /root/.cache/ort.pyke.io -name 'libonnxruntime*' -exec cp -L {} /ort-libs/ \; 2>/dev/null || true; \
    find target/release/build  -name 'libonnxruntime*' -exec cp -L {} /ort-libs/ \; 2>/dev/null || true; \
    test "$(ls -A /ort-libs)" || (echo "ERROR: libonnxruntime not found after build" >&2 && exit 1)

FROM ubuntu:24.04
# libgomp1: OpenMP runtime required by ONNX Runtime.
# libstdc++6: C++ runtime required by libonnxruntime.so.
ENV DEBIAN_FRONTEND=noninteractive
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
