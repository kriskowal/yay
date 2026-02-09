#!/bin/bash
# Run tests for Rust implementation

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."
DIR="$ROOT/rust"

if [[ ! -d "$DIR" ]]; then
  echo "Rust directory not found"
  exit 2
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "Skipping: cargo not installed"
  exit 2
fi

cd "$DIR"

echo "Building release binary..."
cargo build --release

echo "Running tests..."
cargo test
