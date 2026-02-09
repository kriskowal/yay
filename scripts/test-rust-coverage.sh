#!/bin/bash
# Run Rust tests with coverage reporting and enforce minimums

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."
DIR="$ROOT/rust"

# Coverage thresholds
# Note: Some internal helper functions (normalize_hex_spaces, wrap_comment_text, etc.)
# are tested via unit tests rather than fixtures. This is intentional as these
# functions are not directly exercised by the YAY/MEH file format.
MIN_LINES=82

if [[ ! -d "$DIR" ]]; then
	echo "Rust directory not found"
	exit 2
fi

if ! command -v cargo >/dev/null 2>&1; then
	echo "Skipping: cargo not installed"
	exit 2
fi

# Check if cargo-llvm-cov is installed
if ! cargo llvm-cov --version >/dev/null 2>&1; then
	echo "Skipping: cargo-llvm-cov not installed"
	echo "Install with: cargo install cargo-llvm-cov"
	exit 2
fi

cd "$DIR"

echo "Running tests with coverage..."
echo "Minimum threshold: lines=${MIN_LINES}%"
echo

# Run cargo llvm-cov and capture output (covers entire workspace)
output=$(cargo llvm-cov --workspace --summary-only 2>&1)
status=$?

echo "$output"

if [[ $status -ne 0 ]]; then
	echo "Coverage run failed"
	exit 1
fi

# Extract line coverage from TOTAL line
# Format: TOTAL ... 64.70% (line coverage is the last percentage before branch columns)
line_coverage=$(echo "$output" | grep "^TOTAL" | awk '{for(i=1;i<=NF;i++) if($i ~ /%$/) last=$i} END{print last}' | tr -d '%')

if [[ -z "$line_coverage" ]]; then
	echo "Failed to extract line coverage"
	exit 1
fi

# Compare coverage (using bc for floating point)
if ! command -v bc >/dev/null 2>&1; then
	# Fallback: truncate to integer comparison
	line_int=${line_coverage%.*}
	if [[ "$line_int" -lt "$MIN_LINES" ]]; then
		echo ""
		echo "ERROR: Line coverage (${line_coverage}%) is below minimum (${MIN_LINES}%)"
		exit 1
	fi
else
	if (($(echo "$line_coverage < $MIN_LINES" | bc -l))); then
		echo ""
		echo "ERROR: Line coverage (${line_coverage}%) is below minimum (${MIN_LINES}%)"
		exit 1
	fi
fi

echo ""
echo "Coverage check passed: ${line_coverage}% >= ${MIN_LINES}%"
