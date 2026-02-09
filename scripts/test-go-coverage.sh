#!/bin/bash
# Run Go tests with coverage reporting and enforce minimums

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."
DIR="$ROOT/go"

# Coverage thresholds (do not lower these)
MIN_COVERAGE=82

if [[ ! -d "$DIR" ]]; then
	echo "Go directory not found"
	exit 2
fi

if ! command -v go >/dev/null 2>&1; then
	echo "Skipping: go not installed"
	exit 2
fi

cd "$DIR"

echo "Running tests with coverage..."
echo "Minimum threshold: ${MIN_COVERAGE}%"
echo

# Run go test with coverage
output=$(go test -cover ./... 2>&1)
status=$?

echo "$output"

if [[ $status -ne 0 ]]; then
	echo ""
	echo "Tests failed"
	exit 1
fi

# Extract coverage percentage from output
# Format: "coverage: 83.7% of statements"
coverage=$(echo "$output" | grep -o 'coverage: [0-9.]*%' | grep -o '[0-9.]*' | head -1)

if [[ -z "$coverage" ]]; then
	echo "Failed to extract coverage percentage"
	exit 1
fi

# Compare coverage (using bc for floating point, or integer fallback)
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
