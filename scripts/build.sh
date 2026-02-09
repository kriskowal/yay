#!/bin/bash
# Build the yay CLI binary

set -e

cd "$(dirname "$0")/../rust"

echo "Building yay..."
cargo build -p binyay --release

BINARY="target/release/yay"

if [[ -f "$BINARY" ]]; then
  echo "Built: $BINARY"
  echo ""
  echo "To install to /usr/local/bin:"
  echo "  sudo cp $BINARY /usr/local/bin/yay"
  echo ""
  echo "Or add to your PATH:"
  echo "  export PATH=\"\$PATH:$(pwd)/target/release\""
else
  echo "Build failed: $BINARY not found"
  exit 1
fi
