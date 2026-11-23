#!/bin/bash
# Run tests for JavaScript implementation

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."
DIR="$ROOT/js/libyay"

if [[ ! -d "$DIR" ]]; then
  echo "JavaScript directory not found"
  exit 2
fi

if ! command -v node >/dev/null 2>&1; then
  echo "Skipping: node not installed"
  exit 2
fi

# smallcaps.test.js runs live binyay output; build it when cargo is here
if [[ -z "${YAY_BIN:-}" ]] && command -v cargo >/dev/null 2>&1; then
  echo "Building binyay..."
  (cd "$ROOT/rust" && cargo build --quiet -p binyay)
fi

cd "$DIR"

echo "Running tests..."
node --test yay.test.js tagged.test.js smallcaps.test.js references.test.js capdata.test.js codec.test.js
