#!/bin/bash
# Run Python tests with coverage reporting and enforce minimums

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."
DIR="$ROOT/python"

# Coverage thresholds
# Note: dumper.py (serialization) is not tested by fixtures, only parsing is.
# Excluding dumper.py, parser coverage is ~86%.
MIN_COVERAGE=78

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

# Check if coverage is installed
if ! $PYTHON_CMD -m coverage --version >/dev/null 2>&1; then
  echo "Skipping: coverage.py not installed"
  echo "Install with: pip install coverage"
  exit 2
fi

cd "$DIR"

echo "Running tests with coverage..."
echo "Minimum threshold: ${MIN_COVERAGE}%"
echo

# Run tests with coverage
$PYTHON_CMD -m coverage run test_yay.py
status=$?

if [[ $status -ne 0 ]]; then
  echo ""
  echo "Tests failed"
  exit 1
fi

echo ""

# Show coverage report for libyay module only
$PYTHON_CMD -m coverage report --include="libyay/*"

# Extract total coverage percentage
coverage=$($PYTHON_CMD -m coverage report --include="libyay/*" | grep "^TOTAL" | awk '{print $NF}' | tr -d '%')

if [[ -z "$coverage" ]]; then
  echo "Failed to extract coverage percentage"
  exit 1
fi

# Compare coverage
if command -v bc >/dev/null 2>&1; then
  if (($(echo "$coverage < $MIN_COVERAGE" | bc -l))); then
    echo ""
    echo "ERROR: Coverage (${coverage}%) is below minimum (${MIN_COVERAGE}%)"
    exit 1
  fi
else
  # Fallback: truncate to integer comparison
  coverage_int=${coverage%.*}
  if [[ "$coverage_int" -lt "$MIN_COVERAGE" ]]; then
    echo ""
    echo "ERROR: Coverage (${coverage}%) is below minimum (${MIN_COVERAGE}%)"
    exit 1
  fi
fi

echo ""
echo "Coverage check passed: ${coverage}% >= ${MIN_COVERAGE}%"
