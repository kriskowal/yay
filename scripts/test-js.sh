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

cd "$DIR"

echo "Running tests..."
node --test yay.test.js
