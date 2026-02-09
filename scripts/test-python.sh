#!/bin/bash
# Run tests for Python implementation

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."
DIR="$ROOT/python"

if [[ ! -d "$DIR" ]]; then
  echo "Python directory not found"
  exit 2
fi

# Find Python command
PYTHON_CMD=""
if command -v python3 >/dev/null 2>&1; then
  PYTHON_CMD="python3"
elif command -v python >/dev/null 2>&1; then
  PYTHON_CMD="python"
else
  echo "Skipping: python not installed"
  exit 2
fi

cd "$DIR"

echo "Running tests..."
if [[ -f "test_yay.py" ]]; then
  $PYTHON_CMD test_yay.py
else
  $PYTHON_CMD -m pytest -v
fi
