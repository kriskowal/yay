#!/bin/bash
# Run tests for Go implementation

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."
DIR="$ROOT/go"

if [[ ! -d "$DIR" ]]; then
  echo "Go directory not found"
  exit 2
fi

if ! command -v go >/dev/null 2>&1; then
  echo "Skipping: go not installed"
  exit 2
fi

cd "$DIR"

echo "Running code generation..."
go generate

echo "Running tests..."
go test -v ./...
