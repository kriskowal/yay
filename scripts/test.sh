#!/bin/bash
# Run tests for all targets (language implementations, CLI, coverage checks)
#
# This script orchestrates running individual test scripts.
# Each target has its own test-<target>.sh script that can be run independently.

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")

# Return codes
RC_PASS=0
RC_FAIL=1
RC_SKIP=2

# Colors for output (if terminal supports it)
if [[ -t 1 ]]; then
	RED='\033[0;31m'
	GREEN='\033[0;32m'
	YELLOW='\033[0;33m'
	BLUE='\033[0;34m'
	NC='\033[0m' # No Color
else
	RED=''
	GREEN=''
	YELLOW=''
	BLUE=''
	NC=''
fi

# Test targets in order (linting/readme checks first, then language parsers, CLI, coverage)
TARGETS=(shellcheck readmes readme-footer cli-readmes c go java js python rust scm cli go-coverage js-coverage python-coverage rust-coverage)

# Run a single test target
# Returns: 0=pass, 1=fail, 2=skip
run_test() {
	local target="$1"
	local script="$HERE/test-${target}.sh"

	if [[ ! -f "$script" ]]; then
		echo -e "${YELLOW}=== ${target} ===${NC}"
		echo -e "${YELLOW}  Skipping: no test script found${NC}"
		return $RC_SKIP
	fi

	# Map short names to display names
	local display_name
	case "$target" in
	c) display_name="C" ;;
	cli) display_name="CLI (yay transcoder)" ;;
	cli-readmes) display_name="CLI READMEs" ;;
	go) display_name="Go" ;;
	go-coverage) display_name="Go (coverage)" ;;
	java) display_name="Java" ;;
	js) display_name="JavaScript" ;;
	js-coverage) display_name="JavaScript (coverage)" ;;
	python) display_name="Python" ;;
	python-coverage) display_name="Python (coverage)" ;;
	readme-footer) display_name="README footer" ;;
	readmes) display_name="README examples" ;;
	shellcheck) display_name="Shell scripts" ;;
	rust-coverage) display_name="Rust (coverage)" ;;
	rust) display_name="Rust" ;;
	scm) display_name="Scheme (Guile)" ;;
	*) display_name="$target" ;;
	esac

	echo -e "${BLUE}=== ${display_name} ===${NC}"

	set +e
	bash "$script" 2>&1 | sed 's/^/  /'
	local result=$?
	set -e

	case $result in
	0)
		echo -e "${GREEN}  PASSED${NC}"
		return $RC_PASS
		;;
	2)
		echo -e "${YELLOW}  SKIPPED${NC}"
		return $RC_SKIP
		;;
	*)
		echo -e "${RED}  FAILED${NC}"
		return $RC_FAIL
		;;
	esac
}

# Main
main() {
	local exit_code=0
	local passed=0
	local failed=0
	local skipped=0

	# If arguments provided, run only those targets; otherwise run all
	local targets_to_run=()
	if [[ $# -gt 0 ]]; then
		targets_to_run=("$@")
	else
		targets_to_run=("${TARGETS[@]}")
	fi

	echo "Running ${#targets_to_run[@]} test target(s)..."
	echo

	for target in "${targets_to_run[@]}"; do
		set +e
		run_test "$target"
		local result=$?
		set -e

		case $result in
		"$RC_PASS") ((passed++)) || true ;;
		"$RC_FAIL")
			((failed++)) || true
			exit_code=1
			;;
		"$RC_SKIP") ((skipped++)) || true ;;
		esac
		echo
	done

	# Summary
	echo "========================================="
	echo "Summary: $passed passed, $failed failed, $skipped skipped"
	if [[ $exit_code -eq 0 ]]; then
		echo -e "${GREEN}All tests passed!${NC}"
	else
		echo -e "${RED}Some tests failed.${NC}"
	fi

	exit $exit_code
}

main "$@"
