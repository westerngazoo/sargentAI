#!/usr/bin/env bash
# Lay out ONNX Runtime from Microsoft's official NuGet package
# (Microsoft.ML.OnnxRuntime) the way the GitHub release tarball did:
#
#   <dest>/lib/libonnxruntime.so.<version>      the library
#   <dest>/lib/libonnxruntime.so.1              SONAME link (the .so's DT_SONAME)
#   <dest>/lib/libonnxruntime.so                dev link (what ort-sys links)
#   <dest>/include/*.h                          the C/C++ headers
#
# so CI and the Docker image keep consuming /opt/onnxruntime unchanged.
# The caller verifies the package's SHA-256 before calling this.
#
# Usage: ort-from-nupkg.sh <package.nupkg> <x64|arm64> <version> <dest>
set -euo pipefail

nupkg="${1:?usage: ort-from-nupkg.sh <package.nupkg> <x64|arm64> <version> <dest>}"
arch="${2:?arch: x64 or arm64}"
version="${3:?version, e.g. 1.24.2}"
dest="${4:?dest directory}"

case "$arch" in
  x64 | arm64) ;;
  *) echo "unsupported arch: $arch (x64 or arm64)" >&2; exit 1 ;;
esac

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

unzip -q "$nupkg" \
  "runtimes/linux-${arch}/native/libonnxruntime.so" \
  'build/native/include/*' \
  -d "$work"

mkdir -p "$dest/lib" "$dest/include"
install -m 0644 "$work/runtimes/linux-${arch}/native/libonnxruntime.so" \
  "$dest/lib/libonnxruntime.so.${version}"
ln -sf "libonnxruntime.so.${version}" "$dest/lib/libonnxruntime.so.1"
ln -sf "libonnxruntime.so.${version}" "$dest/lib/libonnxruntime.so"
cp "$work"/build/native/include/*.h "$dest/include/"

test -e "$dest/lib/libonnxruntime.so"
