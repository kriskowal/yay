#!/bin/bash
# Run JavaScript tests with coverage reporting and enforce minimums

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."
DIR="$ROOT/js/libyay"

# Coverage thresholds
MIN_STATEMENTS=87
MIN_BRANCHES=77
MIN_FUNCTIONS=93
MIN_LINES=87

if [[ ! -d "$DIR" ]]; then
  echo "JavaScript directory not found"
  exit 2
fi

if ! command -v node >/dev/null 2>&1; then
  echo "Skipping: node not installed"
  exit 2
fi

cd "$DIR"

# Install dependencies if needed
if [[ ! -d "node_modules" ]]; then
  echo "Installing dependencies..."
  npm install
fi

echo "Running tests with coverage..."
echo "Minimum thresholds: statements=${MIN_STATEMENTS}% branches=${MIN_BRANCHES}% functions=${MIN_FUNCTIONS}% lines=${MIN_LINES}%"
echo

# Run c8 with check-coverage to enforce thresholds
npx c8 \
  --all \
  --include yay.js \
  --check-coverage \
  --statements "$MIN_STATEMENTS" \
  --branches "$MIN_BRANCHES" \
  --functions "$MIN_FUNCTIONS" \
  --lines "$MIN_LINES" \
  --reporter=text \
  node --test yay.test.js
