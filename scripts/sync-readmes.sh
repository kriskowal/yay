#!/bin/bash
# Sync README examples and footers with source of truth
#
# The test/ fixtures are the source of truth for examples.
# The root README.md is the source of truth for the footer (References + License).
#
# Usage:
#   ./scripts/sync-readmes.sh        # Update all READMEs
#   ./scripts/sync-readmes.sh --dry  # Show what would be changed

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."
TEST_DIR="$ROOT/test"

# Expected footer text (References + License)
FOOTER='## References

Examples in this document pay homage to:

- The Hitchhiker'"'"'s Guide to the Galaxy (Douglas Adams)
- Monty Python and the Holy Grail
- Monty Python'"'"'s Flying Circus ("Dead Parrot" sketch)
- Galaxy Quest
- Spaceballs
- Tommy Tutone ("867-5309/Jenny")
- The Tau Manifesto

## License

Apache 2.0

Copyright (C) 2026 Kris Kowal'

DRY_RUN=false
if [[ "${1:-}" == "--dry" ]]; then
	DRY_RUN=true
fi

# Extract filename from link (handles both relative and full URL patterns)
extract_link_filename() {
	sed 's|.*test/||; s|).*||'
}

# Find language directories
find_lang_dirs() {
	for dir in "$ROOT"/*/; do
		local name
		name=$(basename "$dir")
		case "$name" in
		test | scripts | npm | homebrew | .github | docs) continue ;;
		esac
		if [[ -f "$dir/README.md" ]]; then
			echo "$name"
		fi
	done
}

# Update code blocks in a README to match fixtures
sync_readme() {
	local readme="$1"
	local readme_rel="${readme#"$ROOT"/}"
	local temp_file
	temp_file=$(mktemp)
	local in_code_block=false
	local current_file=""
	local skip_block=false

	while IFS= read -r line || [[ -n "$line" ]]; do
		if [[ "$in_code_block" == true ]]; then
			if [[ "$line" == '```' ]]; then
				echo '```' >>"$temp_file"
				in_code_block=false
				current_file=""
				skip_block=false
			elif [[ "$skip_block" == false ]]; then
				echo "$line" >>"$temp_file"
			fi
		elif [[ "$line" =~ ^\[.*\]\(.*test/.*\)$ ]]; then
			current_file=$(echo "$line" | extract_link_filename)
			echo "$line" >>"$temp_file"
		elif [[ "$line" =~ ^\`\`\` && -n "$current_file" ]]; then
			in_code_block=true
			echo "$line" >>"$temp_file"

			local fixture="$TEST_DIR/$current_file"
			if [[ -f "$fixture" ]]; then
				local content
				content=$(cat "$fixture")
				content="${content%$'\n'}"
				echo "$content" >>"$temp_file"
				skip_block=true
			else
				skip_block=false
			fi
		else
			if [[ ! "$line" =~ ^[[:space:]]*$ ]]; then
				current_file=""
			fi
			echo "$line" >>"$temp_file"
		fi
	done <"$readme"

	if [[ "$DRY_RUN" == true ]]; then
		if ! diff -q "$readme" "$temp_file" >/dev/null 2>&1; then
			echo "Would update: $readme_rel"
			diff -u "$readme" "$temp_file" | head -50 || true
			echo "..."
		fi
		rm "$temp_file"
	else
		if ! diff -q "$readme" "$temp_file" >/dev/null 2>&1; then
			mv "$temp_file" "$readme"
			echo "Updated: $readme_rel"
		else
			rm "$temp_file"
		fi
	fi
}

# Sync footer in a README (replace from ## References to end)
sync_footer() {
	local readme="$1"
	local readme_rel="${readme#"$ROOT"/}"

	if [[ ! -f "$readme" ]]; then
		return
	fi

	# Skip root README (it's the source of truth for footer)
	if [[ "$readme" == "$ROOT/README.md" ]]; then
		return
	fi

	local temp_file
	temp_file=$(mktemp)

	# Copy everything before ## References, then append the footer
	awk '/^## References$/{exit} {print}' "$readme" >"$temp_file"
	echo "$FOOTER" >>"$temp_file"

	if [[ "$DRY_RUN" == true ]]; then
		if ! diff -q "$readme" "$temp_file" >/dev/null 2>&1; then
			echo "Would update footer: $readme_rel"
		fi
		rm "$temp_file"
	else
		if ! diff -q "$readme" "$temp_file" >/dev/null 2>&1; then
			mv "$temp_file" "$readme"
			echo "Updated footer: $readme_rel"
		else
			rm "$temp_file"
		fi
	fi
}

# Main
main() {
	echo "Syncing READMEs with test fixtures..."

	# Sync root README (examples only, it's the source of truth for footer)
	sync_readme "$ROOT/README.md"

	# Sync language READMEs (examples and footer)
	for dir in c go java python scm; do
		local readme="$ROOT/$dir/README.md"
		if [[ -f "$readme" ]]; then
			sync_readme "$readme"
			sync_footer "$readme"
		fi
	done

	# Sync library READMEs (examples and footer)
	for readme in "$ROOT/js/libyay/README.md" "$ROOT/rust/libyay/README.md"; do
		if [[ -f "$readme" ]]; then
			sync_readme "$readme"
			sync_footer "$readme"
		fi
	done

	echo "Done"
}

main
