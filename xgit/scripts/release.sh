#!/usr/bin/env bash
set -euo pipefail

# Usage: ./scripts/release.sh ./dist
OUT=${1:-./dist}
BIN_NAME=${BIN_NAME:-xgit}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Read package version from Cargo.toml [package] section.
VERSION="$(
  awk -F'"' '
    /^\[package\]/ { in_pkg=1; next }
    /^\[/ && !/^\[package\]/ { in_pkg=0 }
    in_pkg && $0 ~ /^version[[:space:]]*=[[:space:]]*"/ { print $2; exit }
  ' "$PROJECT_DIR/Cargo.toml"
)"
if [[ -z "$VERSION" ]]; then
  VERSION="unknown"
fi

# Clean output directory before each release run.
if [[ -d "$OUT" ]]; then
  rm -rf "$OUT"
fi
mkdir -p "$OUT"

release_targets=()
release_target_count=0

if ! command -v zip >/dev/null 2>&1; then
  echo "'zip' not found. Please install zip utility first."
  exit 1
fi

add_release_target() {
  local target_triple="$1"
  local t
  for t in "${release_targets[@]+"${release_targets[@]}"}"; do
    if [[ "$t" == "$target_triple" ]]; then
      return
    fi
  done
  release_targets+=("$target_triple")
  release_target_count=$((release_target_count + 1))
}

copy_artifact() {
  local src="$1"
  local target_triple="$2"
  local out_dir="$OUT/$target_triple"
  mkdir -p "$out_dir"
  cp "$src" "$out_dir/"
  add_release_target "$target_triple"
}

host_target="$(rustc -vV | awk '/^host:/ {print $2}')"
host_bin="target/release/$BIN_NAME"
if [[ "$host_target" == *windows* ]]; then
  host_bin="${host_bin}.exe"
fi

echo "Building host release ($host_target)"
cargo build --release
copy_artifact "$host_bin" "$host_target"

# Build linux x86_64 using cross (requires `cross` installed)
linux_target="x86_64-unknown-linux-gnu"
if command -v cross >/dev/null 2>&1; then
  echo "Building $linux_target via cross"
  export DOCKER_DEFAULT_PLATFORM=linux/amd64
  cross build --release --target "$linux_target"
  copy_artifact "target/$linux_target/release/$BIN_NAME" "$linux_target"
else
  echo "Skipping $linux_target: 'cross' not installed."
fi

# Build windows x86_64 using cargo (requires target/toolchain installed)
windows_target="x86_64-pc-windows-gnu"
echo "Building $windows_target via cargo"
cargo build --release --target "$windows_target"
copy_artifact "target/$windows_target/release/${BIN_NAME}.exe" "$windows_target"

timestamp="$(date '+%Y%m%d-%H%M%S')"
archive_name="${BIN_NAME}-release-${VERSION}-${timestamp}.zip"

if [[ "$release_target_count" -eq 0 ]]; then
  echo "No release artifacts found to archive."
  exit 1
fi

(
  cd "$OUT"
  zip -r "$archive_name" "${release_targets[@]+"${release_targets[@]}"}" >/dev/null
)

echo "Done. Binaries in: $OUT"
echo "Release archive: $OUT/$archive_name"
