#!/bin/bash
# Verify README examples match test fixtures
#
# This script verifies that:
# 1. All .yay code blocks in README.md match the corresponding test/ fixtures
# 2. All language-specific output blocks match their test/ fixtures
# 3. Language READMEs have the same .yay references as root README, in order
#
# The test/ fixtures are the source of truth. If this script fails,
# run ./scripts/sync-readmes.sh to update the READMEs.

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."
TEST_DIR="$ROOT/test"

exit_code=0

# Extract filename from link (handles both relative and full URL patterns)
extract_link_filename() {
	sed 's|.*test/||; s|).*||'
}

# Iterate over code blocks in a README, calling a handler for each
# Handler receives: filename content readme_path
# shellcheck disable=SC2094  # We read from readme but handler doesn't write to it
foreach_code_block() {
	local readme="$1"
	local handler="$2"
	local in_code_block=false
	local current_file=""
	local content=""

	while IFS= read -r line || [[ -n "$line" ]]; do
		if [[ "$in_code_block" == true ]]; then
			if [[ "$line" == '```' ]]; then
				if [[ -n "$current_file" ]]; then
					"$handler" "$current_file" "$content" "$readme"
				fi
				in_code_block=false
				current_file=""
				content=""
			else
				if [[ -z "$content" ]]; then
					content="$line"
				else
					content="$content"$'\n'"$line"
				fi
			fi
		elif [[ "$line" =~ ^\[.*\]\(.*test/.*\)$ ]]; then
			current_file=$(echo "$line" | extract_link_filename)
		elif [[ "$line" =~ ^\`\`\` && -n "$current_file" ]]; then
			in_code_block=true
			content=""
		else
			if [[ ! "$line" =~ ^[[:space:]]*$ ]]; then
				current_file=""
			fi
		fi
	done <"$readme"
}

# Handler: verify code block matches fixture
# shellcheck disable=SC2317
handle_verify() {
	local filename="$1"
	local content="$2"
	local readme="$3"

	local target="$TEST_DIR/$filename"
	local readme_rel="${readme#"$ROOT"/}"
	if [[ -f "$target" ]]; then
		local expected
		expected=$(cat "$target")
		# Remove trailing newline from fixture for comparison
		expected="${expected%$'\n'}"
		if [[ "$content" != "$expected" ]]; then
			echo "FAIL: $readme_rel: $filename content mismatch"
			echo "  Expected (from test/):"
			echo "$expected" | head -3 | sed 's/^/    /'
			echo "  Actual (in README):"
			echo "$content" | head -3 | sed 's/^/    /'
			exit_code=1
		fi
	else
		echo "FAIL: $readme_rel: $filename not found in test/"
		exit_code=1
	fi
}

# Find language directories
find_lang_dirs() {
	for dir in "$ROOT"/*/; do
		local name
		name=$(basename "$dir")
		case "$name" in
		test | scripts | npm | homebrew | .github | docs | vim | vscode | www) continue ;;
		esac
		if [[ -f "$dir/README.md" ]]; then
			echo "$name"
		fi
	done
}

# Extract ordered list of .yay references from a README
extract_yay_refs() {
	local readme="$1"
	grep -oE '\[.*\.yay\]\(.*test/.*\.yay\)' "$readme" 2>/dev/null | extract_link_filename || true
}

# Verify language READMEs have same .yay refs as root README in same order
verify_order() {
	local root_refs
	root_refs=$(extract_yay_refs "$ROOT/README.md")

	for dir in $(find_lang_dirs); do
		# Skip rust and js - they may have different README structure
		case "$dir" in
		rust | js) continue ;;
		esac

		local lang_readme="$ROOT/$dir/README.md"
		local lang_refs
		lang_refs=$(extract_yay_refs "$lang_readme")
		if [[ "$lang_refs" != "$root_refs" ]]; then
			echo "FAIL: $dir/README.md: .yay references differ from root README"
			local missing extra
			missing=$(comm -23 <(echo "$root_refs" | sort) <(echo "$lang_refs" | sort) 2>/dev/null || true)
			extra=$(comm -13 <(echo "$root_refs" | sort) <(echo "$lang_refs" | sort) 2>/dev/null || true)
			if [[ -n "$missing" ]]; then
				echo "  missing:"
				echo "$missing" | while read -r f; do [[ -n "$f" ]] && echo "    $f"; done
			fi
			if [[ -n "$extra" ]]; then
				echo "  extra:"
				echo "$extra" | while read -r f; do [[ -n "$f" ]] && echo "    $f"; done
			fi
			if [[ -z "$missing" && -z "$extra" ]]; then
				echo "  order differs from root README"
			fi
			exit_code=1
		fi
	done
}

# Main
main() {
	echo "Verifying README examples match test fixtures..."

	# Verify root README
	foreach_code_block "$ROOT/README.md" handle_verify

	# Verify language READMEs
	for dir in $(find_lang_dirs); do
		foreach_code_block "$ROOT/$dir/README.md" handle_verify
	done

	# Verify library READMEs
	foreach_code_block "$ROOT/js/libyay/README.md" handle_verify
	foreach_code_block "$ROOT/rust/libyay/README.md" handle_verify

	# Verify language READMEs have same refs as root
	verify_order

	if [[ $exit_code -eq 0 ]]; then
		echo "All README examples match fixtures"
	else
		echo ""
		echo "To fix mismatches, run: ./scripts/sync-readmes.sh"
	fi

	exit $exit_code
}

main
