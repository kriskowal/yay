#!/bin/bash
# Run tests for C implementation

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."
DIR="$ROOT/c"

if [[ ! -d "$DIR" ]]; then
	echo "C directory not found"
	exit 2
fi

if ! command -v make >/dev/null 2>&1; then
	echo "Skipping: make not installed"
	exit 2
fi

cd "$DIR"

echo "Building and running tests..."
make test
