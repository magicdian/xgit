#!/usr/bin/env bash
set -euo pipefail

# Simple release script: build for host and linux (requires toolchains installed)
# Usage: ./scripts/release.sh ./dist
OUT=${1:-./dist}
mkdir -p "$OUT"

echo "Building host release"
cargo build --release
HOST_BIN=target/release/xgit
cp "$HOST_BIN" "$OUT/xgit-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)"

# Build linux x86_64 using cross (requires `cross` installed)
# Set Docker platform to ensure correct architecture when using docker-based cross
if command -v cross >/dev/null 2>&1; then
  echo "Building x86_64-unknown-linux-gnu via cross"
  export DOCKER_DEFAULT_PLATFORM=linux/amd64
  cross build --release --target x86_64-unknown-linux-gnu
  cp target/x86_64-unknown-linux-gnu/release/xgit "$OUT/xgit-linux-x86_64"
else
  echo "'cross' not installed. To install: cargo install cross"
  echo "Or install cross: https://github.com/cross-rs/cross#installation"
fi

# Optionally, build macos (if on macOS or cross-toolchain available)
if [[ "$(uname -s)" == "Darwin" ]]; then
  echo "Already built macOS host binary"
fi

echo "Done. Binaries in: $OUT"
