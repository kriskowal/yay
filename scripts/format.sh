#!/bin/bash
# Run code formatters for all languages.
#
# Usage:
#   scripts/format.sh          # format everything
#   scripts/format.sh rust     # format only Rust
#   scripts/format.sh go js    # format Go and JavaScript

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT=$(cd "$HERE/.." && pwd)

# Track what ran
ran=0
skipped=()

# ── Helpers ──────────────────────────────────────────────────────────────────

should_run() {
	local lang="$1"
	if [[ $# -eq 0 ]] || [[ "${TARGETS[*]}" == "all" ]]; then
		return 0
	fi
	for t in "${TARGETS[@]}"; do
		if [[ "$t" == "$lang" ]]; then
			return 0
		fi
	done
	return 1
}

run_formatter() {
	local name="$1"
	shift
	echo "=== $name ==="
	if "$@"; then
		echo "  done"
	else
		echo "  FAILED (exit $?)"
		return 1
	fi
	ran=$((ran + 1))
}

skip_formatter() {
	local name="$1"
	local reason="$2"
	echo "=== $name ==="
	echo "  skipped: $reason"
	skipped+=("$name")
}

# ── Parse arguments ─────────────────────────────────────────────────────────

if [[ $# -eq 0 ]]; then
	TARGETS=(all)
else
	TARGETS=("$@")
fi

# ── Shell (shfmt) ───────────────────────────────────────────────────────────

if should_run shell; then
	if command -v shfmt &>/dev/null; then
		run_formatter "Shell (shfmt)" shfmt -w "$HERE"/*.sh
	else
		skip_formatter "Shell (shfmt)" "shfmt not found"
	fi
fi

# ── Rust (rustfmt) ──────────────────────────────────────────────────────────

if should_run rust; then
	if command -v rustfmt &>/dev/null; then
		run_formatter "Rust (rustfmt)" cargo fmt --all --manifest-path "$ROOT/rust/Cargo.toml"
	else
		skip_formatter "Rust (rustfmt)" "rustfmt not found"
	fi
fi

# ── Go (gofmt) ──────────────────────────────────────────────────────────────

if should_run go; then
	if command -v gofmt &>/dev/null; then
		run_formatter "Go (gofmt)" gofmt -w "$ROOT/go/"
	else
		skip_formatter "Go (gofmt)" "gofmt not found"
	fi
fi

# ── JavaScript (prettier) ──────────────────────────────────────────────────

if should_run js; then
	if npx prettier --version &>/dev/null; then
		js_files=$(find "$ROOT/js" -name '*.js' \
			-not -path '*/coverage/*' \
			-not -path '*/node_modules/*')
		# shellcheck disable=SC2086
		run_formatter "JavaScript (prettier)" npx prettier --write $js_files
	else
		skip_formatter "JavaScript (prettier)" "prettier not found"
	fi
fi

# ── Python (ruff) ───────────────────────────────────────────────────────────

if should_run python; then
	if command -v ruff &>/dev/null; then
		run_formatter "Python (ruff)" ruff format "$ROOT/python/"
	else
		skip_formatter "Python (ruff)" "ruff not found"
	fi
fi

# ── Java (google-java-format) ──────────────────────────────────────────────

if should_run java; then
	if command -v google-java-format &>/dev/null; then
		run_formatter "Java (google-java-format)" \
			google-java-format --replace "$ROOT"/java/src/main/java/com/kriskowal/yay/*.java "$ROOT"/java/src/test/java/com/kriskowal/yay/*.java
	else
		skip_formatter "Java" "google-java-format not found (install: brew install google-java-format)"
	fi
fi

# ── Summary ─────────────────────────────────────────────────────────────────

echo ""
echo "========================================="
echo "Formatted: $ran"
if [[ ${#skipped[@]} -gt 0 ]]; then
	echo "Skipped: ${skipped[*]}"
fi
