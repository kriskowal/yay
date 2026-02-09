#!/bin/bash
# Verify binyay READMEs link to CLI.md and their examples are valid
#
# Checks:
# 1. Each binyay README contains a link to CLI.md
# 2. Each `yay` command example in binyay READMEs appears in CLI.md

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."

exit_code=0

# Extract yay command examples from a file (lines starting with yay or containing | yay)
extract_yay_commands() {
  grep -oE '(^|[|] *)yay [^#]*' "$1" 2>/dev/null | sed 's/^[| ]*//' | sort -u || true
}

# Check a binyay README
check_readme() {
  local readme="$1"
  local name="$2"

  if [[ ! -f "$readme" ]]; then
    echo "SKIP: $name not found"
    return 0
  fi

  # Check for CLI.md link
  if ! grep -qE 'CLI\.md|CLI documentation' "$readme"; then
    echo "FAIL: $name missing link to CLI.md"
    exit_code=1
  else
    echo "PASS: $name links to CLI.md"
  fi

  # Extract yay commands from this README
  local readme_commands
  readme_commands=$(extract_yay_commands "$readme")

  if [[ -z "$readme_commands" ]]; then
    return 0
  fi

  # Check each command appears in CLI.md
  local cli_content
  cli_content=$(cat "$ROOT/CLI.md")

  while IFS= read -r cmd; do
    [[ -z "$cmd" ]] && continue
    if ! echo "$cli_content" | grep -qF "$cmd"; then
      # Try a looser match - just the flags
      local flags
      flags=$(echo "$cmd" | grep -oE -- '-[a-zA-Z]+' | head -3 | tr '\n' ' ')
      if [[ -n "$flags" ]] && echo "$cli_content" | grep -qE "yay.*$flags"; then
        : # Found with looser match
      else
        echo "WARN: $name has example not in CLI.md: $cmd"
      fi
    fi
  done <<<"$readme_commands"
}

# Main
echo "Checking binyay READMEs..."

check_readme "$ROOT/js/binyay/README.md" "js/binyay/README.md"
check_readme "$ROOT/rust/binyay/README.md" "rust/binyay/README.md"

if [[ $exit_code -eq 0 ]]; then
  echo "All binyay READMEs OK"
fi

exit $exit_code
