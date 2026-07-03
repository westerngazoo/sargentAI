# syntax=docker/dockerfile:1.7
#
# ONNX Runtime (used by `ort` for pose estimation) is installed into the image
# from Microsoft's official release tarball and linked dynamically. `ort` is
# built with `default-features = false` (see `backend/Cargo.toml`), so its
# `download-binaries` feature is off and the build NEVER contacts `cdn.pyke.io`.
# `ort-sys` links against the library we provide via `ORT_LIB_LOCATION` +
# `ORT_PREFER_DYNAMIC_LINK`.
#
# Ubuntu 24.04 (glibc 2.39) comfortably satisfies both the Rust toolchain and
# the official ONNX Runtime build.

# ---------------------------------------------------------------------------
# ONNX Runtime: fetch the official shared library for the target architecture.
# ---------------------------------------------------------------------------
FROM ubuntu:24.04 AS onnxruntime
# Must match the ONNX Runtime 1.24 ABI that `ort-sys` 2.0.0-rc.12 targets
# (see the `api-24` feature in backend/Cargo.toml). Keep this in lockstep with
# `ONNXRUNTIME_VERSION` in the `rust` job of .github/workflows/ci.yml.
ARG ONNXRUNTIME_VERSION=1.24.2
# Provided automatically by BuildKit: "amd64" or "arm64".
ARG TARGETARCH
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
        curl \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*
# Map Docker's TARGETARCH to ONNX Runtime's release-asset architecture name.
RUN case "${TARGETARCH}" in \
        amd64) ort_arch=x64 ;; \
        arm64) ort_arch=aarch64 ;; \
        *) echo "unsupported TARGETARCH: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    pkg="onnxruntime-linux-${ort_arch}-${ONNXRUNTIME_VERSION}"; \
    curl --proto '=https' --tlsv1.2 -fsSL \
        "https://github.com/microsoft/onnxruntime/releases/download/v${ONNXRUNTIME_VERSION}/${pkg}.tgz" \
        -o /tmp/ort.tgz; \
    mkdir -p /opt/onnxruntime; \
    tar -xzf /tmp/ort.tgz -C /opt/onnxruntime --strip-components=1 "${pkg}/lib" "${pkg}/include"; \
    rm /tmp/ort.tgz; \
    test -e /opt/onnxruntime/lib/libonnxruntime.so

# ---------------------------------------------------------------------------
# Rust toolchain (cached independently of source changes).
# ---------------------------------------------------------------------------
FROM ubuntu:24.04 AS rust-toolchain
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

# ---------------------------------------------------------------------------
# Build the API, linking `ort-sys` against the ONNX Runtime we just fetched.
# ---------------------------------------------------------------------------
FROM rust-toolchain AS builder
WORKDIR /src
# build-essential: full C/C++ toolchain (gcc, g++, libc6-dev, libstdc++-14-dev,
#   make).  libc6-dev provides the CRT objects (Scrt1.o, crti.o, crtn.o) and
#   link stubs that rust-lld needs even for the earliest build-script crates.
# pkg-config: retained for any *-sys crate that consults it.
# The symlink /usr/lib/*/libstdc++.so -> .so.6 is required because Rust 1.90+
#   uses bundled rust-lld, which does not parse GCC's libstdc++.so linker script.
RUN apt-get update && apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
    && arch="$(dpkg-architecture -qDEB_HOST_MULTIARCH)" \
    && ln -sf "/usr/lib/${arch}/libstdc++.so.6" "/usr/lib/${arch}/libstdc++.so" \
    && rm -rf /var/lib/apt/lists/*
# The ONNX Runtime shared library `ort-sys` links against at build time.
COPY --from=onnxruntime /opt/onnxruntime/lib/ /opt/onnxruntime/lib/
ENV ORT_LIB_LOCATION=/opt/onnxruntime/lib \
    ORT_PREFER_DYNAMIC_LINK=1
COPY . .
RUN cargo build --release -p fitai-api --locked

# ---------------------------------------------------------------------------
# Runtime image.
# ---------------------------------------------------------------------------
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
# Ship the ONNX Runtime shared library the binary loads at runtime. The tarball
# ships `.so`, `.so.1` and `.so.1.x.y` as three identical real files; keep only
# the versioned one and recreate the SONAME/dev symlinks so `ldconfig` is happy.
COPY --from=onnxruntime /opt/onnxruntime/lib/libonnxruntime.so.[0-9]*.[0-9]* /usr/local/lib/
RUN cd /usr/local/lib \
    && real="$(ls libonnxruntime.so.[0-9]*.[0-9]*)" \
    && ln -sf "${real}" libonnxruntime.so.1 \
    && ln -sf "${real}" libonnxruntime.so
COPY --from=builder /src/target/release/fitai-api /usr/local/bin/fitai-api
RUN ldconfig
USER app
EXPOSE 8080
ENV PORT=8080
CMD ["fitai-api"]
