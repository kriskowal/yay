#!/bin/bash
# Verify shell scripts pass shfmt and shellcheck

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")

# Check if tools are available
if ! command -v shfmt &>/dev/null; then
	echo "shfmt not found, skipping"
	exit 2
fi

if ! command -v shellcheck &>/dev/null; then
	echo "shellcheck not found, skipping"
	exit 2
fi

exit_code=0

# Check formatting with shfmt
echo "Checking formatting with shfmt..."
if ! shfmt -d -i 2 "$HERE"/*.sh; then
	echo "FAIL: Scripts need formatting. Run: shfmt -w -i 2 scripts/*.sh"
	exit_code=1
else
	echo "PASS: All scripts formatted correctly"
fi

# Check with shellcheck
echo "Checking with shellcheck..."
if ! shellcheck "$HERE"/*.sh; then
	echo "FAIL: shellcheck found issues"
	exit_code=1
else
	echo "PASS: shellcheck found no issues"
fi

exit $exit_code
