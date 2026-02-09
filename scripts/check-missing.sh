#!/bin/bash
# Report missing test files
# For every .yay basename, there should be a corresponding file in every language dir.
# For every .nay basename, there should be a corresponding .error file.

set -ueo pipefail
IFS=$'\t\n'

HERE=$(dirname "${BASH_SOURCE[0]}")
TEST_ROOT="$HERE/../test"

if [[ ! -d "$TEST_ROOT" ]]; then
	echo "Test directory not found: $TEST_ROOT" >&2
	exit 1
fi

exit_code=0

# Check that every .yay basename has a file in each language directory
yay_basenames() {
	find "$TEST_ROOT/yay" -maxdepth 1 -name '*.yay' -exec basename {} .yay \; | sort -u
}

nay_basenames() {
	find "$TEST_ROOT/nay" -maxdepth 1 -name '*.nay' -exec basename {} .nay \; | sort -u
}

# Check .nay/.error parity
echo "Checking nay/error parity..."
while IFS= read -r base; do
	if [[ ! -f "$TEST_ROOT/nay/$base.error" ]]; then
		echo "MISSING: nay/$base.error"
		exit_code=1
	fi
done < <(nay_basenames)

# Check language coverage for valid fixtures
LANG_DIRS=(c go java js py rs scm)
echo "Checking language coverage for .yay fixtures..."
while IFS= read -r base; do
	for lang in "${LANG_DIRS[@]}"; do
		if [[ ! -f "$TEST_ROOT/$lang/$base.$lang" ]]; then
			echo "MISSING: $lang/$base.$lang"
			exit_code=1
		fi
	done
done < <(yay_basenames)

# Check for missing CI workflows
WORKFLOW_DIR="$HERE/../.github/workflows"
missing_workflows=()
for lang in "${LANG_DIRS[@]}"; do
	case "$lang" in
	js) workflow="js.yml" ;;
	go) workflow="go.yml" ;;
	py) workflow="python.yml" ;;
	rs) workflow="rust.yml" ;;
	scm) workflow="guile.yml" ;;
	c) workflow="c.yml" ;;
	java) workflow="java.yml" ;;
	*) workflow="$lang.yml" ;;
	esac
	if [[ ! -f "$WORKFLOW_DIR/$workflow" ]]; then
		missing_workflows+=("$workflow")
	fi
done

if [[ ${#missing_workflows[@]} -gt 0 ]]; then
	echo "Missing CI workflows:"
	printf '  %s\n' "${missing_workflows[@]}"
	exit_code=1
fi

if [[ $exit_code -eq 0 ]]; then
	echo "All files present."
fi

exit $exit_code
