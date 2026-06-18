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
# build-essential: full C/C++ toolchain (gcc, g++, libc6-dev, libstdc++-14-dev,
#   make).  libc6-dev is critical — it provides the CRT objects (Scrt1.o,
#   crti.o, crtn.o) and link stubs (libc.so, libm.so, libpthread.so …) that
#   rust-lld needs even for the earliest build-script crates.
# pkg-config + libssl-dev: openssl-sys (native-tls ← ureq ← ort-sys).
# The symlink /usr/lib/x86_64-linux-gnu/libstdc++.so → .so.6 is required
# because Rust 1.90+ uses its bundled rust-lld which does NOT search
# GCC-internal paths and does NOT parse GCC linker scripts.
# libstdc++-14-dev (pulled in by g++) puts libstdc++.so in
# /usr/lib/gcc/x86_64-linux-gnu/14/ as a linker script; lld rejects it.
# A real symlink in the standard lib dir is the fix.
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        libssl-dev \
    && ln -sf /usr/lib/x86_64-linux-gnu/libstdc++.so.6 \
              /usr/lib/x86_64-linux-gnu/libstdc++.so \
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
