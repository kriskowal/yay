#!/bin/bash
# Run tests for Scheme (Guile) implementation

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."
DIR="$ROOT/scm"

if [[ ! -d "$DIR" ]]; then
	echo "Scheme directory not found"
	exit 2
fi

if ! command -v guile >/dev/null 2>&1; then
	echo "Skipping: guile not installed"
	exit 2
fi

cd "$DIR"

echo "Running tests..."
guile -L . --no-auto-compile run-tests.scm
