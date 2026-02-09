#!/bin/bash
# Verify README footer sections (References and License) are present and consistent

set -ueo pipefail
IFS=$'\t\n'

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."

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

exit_code=0

# Extract from "## References" to end, trimming trailing empty lines
extract_footer() {
  awk '/^## References$/,0' "$1" | awk '
    { lines[NR] = $0 }
    END {
      # Find last non-empty line
      last = 0
      for (i = NR; i >= 1; i--) {
        if (lines[i] != "") {
          last = i
          break
        }
      }
      # Print up to last non-empty line
      for (i = 1; i <= last; i++) {
        print lines[i]
      }
    }
  '
}

# Check a README file
check_readme() {
  local readme="$1"
  local name="$2"

  if [[ ! -f "$readme" ]]; then
    return 0 # Skip if doesn't exist
  fi

  if ! grep -q "^## References$" "$readme"; then
    echo "FAIL: Missing References section in $name"
    exit_code=1
    return 1
  fi

  if ! grep -q "^## License$" "$readme"; then
    echo "FAIL: Missing License section in $name"
    exit_code=1
    return 1
  fi

  local footer
  footer=$(extract_footer "$readme")

  if [[ "$footer" != "$FOOTER" ]]; then
    echo "FAIL: Footer mismatch in $name"
    exit_code=1
    return 1
  fi

  echo "PASS: $name"
  return 0
}

# Check root README
check_readme "$ROOT/README.md" "README.md"

# Check language READMEs (those with examples)
for dir in c go java python scm; do
  check_readme "$ROOT/$dir/README.md" "$dir/README.md"
done

# Check library READMEs
check_readme "$ROOT/js/libyay/README.md" "js/libyay/README.md"
check_readme "$ROOT/rust/libyay/README.md" "rust/libyay/README.md"

if [[ $exit_code -eq 0 ]]; then
  echo "All README footers match"
fi

exit $exit_code
