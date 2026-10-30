#!/bin/bash
# Test script for the yay CLI transcoder
#
# Tests:
#   - Idempotence: YAY -> YAY produces identical output
#   - Reformat: MEH (loose YAY) -> YAY normalization
#   - Round-trip: YAY -> YSON -> YAY data integrity
#   - Output: YAY -> JS/Go/C code generation
#   - Error: Invalid .nay files are rejected
#   - Transcode: YAY -> YAML/TOML/CBOR against golden fixtures
#   - Ingest: YAML/TOML/CBOR -> YAY against golden fixtures
#   - Markdown: front matter normalized, body untouched; md <-> {...head, content}
#   - Check: --check without -t (valid) and with -t (unchanged by -t)
#
# Exit codes: 0=pass, 1=fail, 2=skip

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
YAY="$ROOT_DIR/rust/target/release/yay"
TEST_DIR="$ROOT_DIR/test"
YAY_DIR="$TEST_DIR/yay"
NAY_DIR="$TEST_DIR/nay"
MEH_DIR="$TEST_DIR/meh"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

PASSED=0
FAILED=0
SKIPPED=0

# Check if yay binary exists
if [[ ! -x "$YAY" ]]; then
  echo "yay binary not found at $YAY"
  echo "Skipping: run 'cd rust && cargo build --release' first"
  exit 2
fi

echo "Using yay: $YAY"
echo ""

# =============================================================================
# Test Functions
# =============================================================================

pass() {
  echo -e "${GREEN}PASS${NC}: $1"
  ((PASSED++))
}

fail() {
  echo -e "${RED}FAIL${NC}: $1"
  if [[ -n "${2:-}" ]]; then
    echo "  $2"
  fi
  ((FAILED++))
}

skip() {
  echo -e "${YELLOW}SKIP${NC}: $1"
  ((SKIPPED++))
}

# =============================================================================
# 1. Idempotency Tests
# =============================================================================

run_idempotence_tests() {
  echo "=== Idempotence Tests (YAY → YAY) ==="
  echo ""

  local tmp1 tmp2 name
  tmp1=$(mktemp)
  tmp2=$(mktemp)
  trap 'rm -f "$tmp1" "$tmp2"' RETURN

  for yay_file in "$YAY_DIR"/*.yay; do
    [[ -f "$yay_file" ]] || continue
    name=$(basename "$yay_file")

    # First pass
    if ! "$YAY" -t yay "$yay_file" >"$tmp1" 2>/dev/null; then
      fail "idempotence/$name (first pass failed)"
      continue
    fi

    # Second pass
    if ! "$YAY" -t yay "$tmp1" >"$tmp2" 2>/dev/null; then
      fail "idempotence/$name (second pass failed)"
      continue
    fi

    # Compare
    if diff -q "$tmp1" "$tmp2" >/dev/null 2>&1; then
      pass "idempotence/$name"
    else
      fail "idempotence/$name (output differs after second pass)"
    fi
  done

  echo ""
}

# =============================================================================
# 2. Reformatter Tests
# =============================================================================

run_reformat_tests() {
  echo "=== Reformatter Tests (MEH → YAY) ==="
  echo ""

  local tmp name base expected_file
  tmp=$(mktemp)
  trap 'rm -f "$tmp"' RETURN

  for meh_file in "$MEH_DIR"/*.meh; do
    [[ -f "$meh_file" ]] || continue
    name=$(basename "$meh_file")

    # Extract base name (e.g., "comment-alignment" from "comment-alignment.unaligned.meh")
    base=${name%.*}     # remove .meh
    base=${base%.*}.yay # remove variant, add .yay
    expected_file="$MEH_DIR/$base"

    if [[ ! -f "$expected_file" ]]; then
      skip "reformat/$name (no expected .yay file)"
      continue
    fi

    # Run formatter
    if ! "$YAY" -t yay "$meh_file" >"$tmp" 2>/dev/null; then
      fail "reformat/$name (formatter failed)"
      continue
    fi

    # Compare
    if diff -q "$tmp" "$expected_file" >/dev/null 2>&1; then
      pass "reformat/$name"
    else
      fail "reformat/$name (output differs from expected)"
      echo "  Expected: $expected_file"
      echo "  Got:"
      diff "$expected_file" "$tmp" | head -10 | sed 's/^/    /'
    fi

    # --check -t yay refuses a document the formatter would change
    if cmp -s "$meh_file" "$expected_file"; then
      continue
    fi
    if "$YAY" --check -f meh -t yay "$meh_file" >/dev/null 2>&1; then
      fail "formatted/$name (should have been refused)"
    else
      pass "formatted/$name (refused)"
    fi
  done

  # --check -t yay accepts every expected output
  for expected_file in "$MEH_DIR"/*.yay; do
    [[ -f "$expected_file" ]] || continue
    name=$(basename "$expected_file")
    if "$YAY" --check -t yay "$expected_file" >/dev/null 2>&1; then
      pass "formatted/$name"
    else
      fail "formatted/$name (refused)"
      "$YAY" --check -t yay "$expected_file" 2>&1 | head -3 | sed 's/^/    /'
    fi
  done

  # Both forms of a block string read the same: a blank line after it, or
  # none, or a run of them.
  for meh_file in "$MEH_DIR"/block-string-*.meh; do
    [[ -f "$meh_file" ]] || continue
    name=$(basename "$meh_file")
    base=${name%.*}
    base=${base%.*}.yay
    if cmp -s <("$YAY" -f meh -t yson "$meh_file" 2>&1) <("$YAY" -f yay -t yson "$MEH_DIR/$base" 2>&1); then
      pass "same-value/$name"
    else
      fail "same-value/$name (reads differently from $base)"
    fi
  done

  echo ""
}

# =============================================================================
# 3. Round-Trip Tests
# =============================================================================

run_roundtrip_tests() {
  echo "=== Round-Trip Tests (YAY ↔ YSON) ==="
  echo ""

  local tmp_yson tmp_yay name
  tmp_yson=$(mktemp)
  tmp_yay=$(mktemp)
  trap 'rm -f "$tmp_yson" "$tmp_yay"' RETURN

  for yay_file in "$YAY_DIR"/*.yay; do
    [[ -f "$yay_file" ]] || continue
    name=$(basename "$yay_file")

    # YAY → YSON
    if ! "$YAY" -t yson "$yay_file" >"$tmp_yson" 2>/dev/null; then
      fail "roundtrip/$name (YAY→YSON failed)"
      continue
    fi

    # YSON → YAY
    if ! "$YAY" -f yson -t yay "$tmp_yson" >"$tmp_yay" 2>/dev/null; then
      fail "roundtrip/$name (YSON→YAY failed)"
      continue
    fi

    # We can't compare text directly (formatting differs)
    # Just verify it parses successfully
    if "$YAY" --check "$tmp_yay" 2>/dev/null; then
      pass "roundtrip/$name"
    else
      fail "roundtrip/$name (result invalid)"
    fi
  done

  echo ""
}

# =============================================================================
# 4. Output Format Tests
# =============================================================================

run_output_format_tests() {
  echo "=== Output Format Tests (YAY → Language) ==="
  echo ""

  local tmp base expected_file
  tmp=$(mktemp)
  trap 'rm -f "$tmp"' RETURN

  for ext in js go c; do
    for yay_file in "$YAY_DIR"/*.yay; do
      [[ -f "$yay_file" ]] || continue
      base=$(basename "$yay_file" .yay)
      expected_file="$TEST_DIR/$ext/$base.$ext"

      [[ -f "$expected_file" ]] || continue

      # Generate output
      if ! "$YAY" -t "$ext" "$yay_file" >"$tmp" 2>/dev/null; then
        fail "output/$base.$ext (generation failed)"
        continue
      fi

      # Compare (normalize whitespace)
      if diff -q <(tr -s '[:space:]' ' ' <"$tmp") <(tr -s '[:space:]' ' ' <"$expected_file") >/dev/null 2>&1; then
        pass "output/$base.$ext"
      else
        fail "output/$base.$ext (output differs)"
      fi
    done
  done

  echo ""
}

# =============================================================================
# 5. Error Tests
# =============================================================================

run_error_tests() {
  echo "=== Error Tests (NAY files) ==="
  echo ""

  local name
  for nay_file in "$NAY_DIR"/*.nay; do
    [[ -f "$nay_file" ]] || continue
    name=$(basename "$nay_file")

    # Should fail to parse with strict YAY mode
    if "$YAY" --from yay --check "$nay_file" 2>/dev/null; then
      fail "error/$name (should have failed)"
    else
      pass "error/$name"
    fi
  done

  echo ""
}

# =============================================================================
# 6. Transcode Tests (YAY → YAML/TOML/CBOR against golden fixtures)
# =============================================================================

run_transcode_tests() {
  echo "=== Transcode Tests (YAY → YAML/TOML/CBOR) ==="
  echo ""

  local tmp base

  # --- YAML ---
  for yay_file in "$YAY_DIR"/*.yay; do
    [[ -f "$yay_file" ]] || continue
    base=$(basename "$yay_file" .yay)

    local expected_yaml="$TEST_DIR/yaml/$base.yaml"
    local expected_err="$TEST_DIR/yaml/$base.error"

    if [[ -f "$expected_err" ]]; then
      # Expect failure
      if "$YAY" -t yaml "$yay_file" >/dev/null 2>&1; then
        fail "transcode/yaml/$base (should have failed)"
      else
        pass "transcode/yaml/$base (expected error)"
      fi
    elif [[ -f "$expected_yaml" ]]; then
      tmp=$(mktemp)
      if "$YAY" -t yaml "$yay_file" >"$tmp" 2>/dev/null; then
        if diff -q "$tmp" "$expected_yaml" >/dev/null 2>&1; then
          pass "transcode/yaml/$base"
        else
          fail "transcode/yaml/$base (output differs)"
          diff "$expected_yaml" "$tmp" | head -5 | sed 's/^/    /'
        fi
      else
        fail "transcode/yaml/$base (conversion failed)"
      fi
      rm -f "$tmp"
    fi
  done

  # --- TOML ---
  for yay_file in "$YAY_DIR"/*.yay; do
    [[ -f "$yay_file" ]] || continue
    base=$(basename "$yay_file" .yay)

    local expected_toml="$TEST_DIR/toml/$base.toml"
    local expected_err="$TEST_DIR/toml/$base.error"

    if [[ -f "$expected_err" ]]; then
      # Expect failure
      if "$YAY" -t toml "$yay_file" >/dev/null 2>&1; then
        fail "transcode/toml/$base (should have failed)"
      else
        pass "transcode/toml/$base (expected error)"
      fi
    elif [[ -f "$expected_toml" ]]; then
      tmp=$(mktemp)
      if "$YAY" -t toml "$yay_file" >"$tmp" 2>/dev/null; then
        if diff -q "$tmp" "$expected_toml" >/dev/null 2>&1; then
          pass "transcode/toml/$base"
        else
          fail "transcode/toml/$base (output differs)"
          diff "$expected_toml" "$tmp" | head -5 | sed 's/^/    /'
        fi
      else
        fail "transcode/toml/$base (conversion failed)"
      fi
      rm -f "$tmp"
    fi
  done

  # --- CBOR ---
  for yay_file in "$YAY_DIR"/*.yay; do
    [[ -f "$yay_file" ]] || continue
    base=$(basename "$yay_file" .yay)

    local expected_cbor="$TEST_DIR/cbor/$base.cbor"
    local expected_err="$TEST_DIR/cbor/$base.error"

    if [[ -f "$expected_err" ]]; then
      # Expect failure
      if "$YAY" -t cbor "$yay_file" -o /dev/null 2>/dev/null; then
        fail "transcode/cbor/$base (should have failed)"
      else
        pass "transcode/cbor/$base (expected error)"
      fi
    elif [[ -f "$expected_cbor" ]]; then
      tmp=$(mktemp)
      if "$YAY" -t cbor "$yay_file" -o "$tmp" 2>/dev/null; then
        if cmp -s "$tmp" "$expected_cbor"; then
          pass "transcode/cbor/$base"
        else
          fail "transcode/cbor/$base (output differs)"
        fi
      else
        fail "transcode/cbor/$base (conversion failed)"
      fi
      rm -f "$tmp"
    fi
  done

  echo ""
}

# =============================================================================
# 7. Ingest Tests (YAML/TOML/CBOR → YAY against golden fixtures)
# =============================================================================

run_ingest_tests() {
  echo "=== Ingest Tests (YAML/TOML/CBOR → YAY) ==="
  echo ""

  local tmp base

  # --- from-yaml ---
  for input_file in "$TEST_DIR"/from-yaml/*.yaml; do
    [[ -f "$input_file" ]] || continue
    base=$(basename "$input_file" .yaml)

    local expected_yay="$TEST_DIR/from-yaml/$base.yay"
    local expected_err="$TEST_DIR/from-yaml/$base.error"

    if [[ -f "$expected_err" ]]; then
      if "$YAY" -f yaml -t yay "$input_file" >/dev/null 2>&1; then
        fail "ingest/from-yaml/$base (should have failed)"
      else
        pass "ingest/from-yaml/$base (expected error)"
      fi
    elif [[ -f "$expected_yay" ]]; then
      tmp=$(mktemp)
      if "$YAY" -f yaml -t yay "$input_file" >"$tmp" 2>/dev/null; then
        if diff -q "$tmp" "$expected_yay" >/dev/null 2>&1; then
          pass "ingest/from-yaml/$base"
        else
          fail "ingest/from-yaml/$base (output differs)"
          diff "$expected_yay" "$tmp" | head -5 | sed 's/^/    /'
        fi
      else
        fail "ingest/from-yaml/$base (decode failed)"
      fi
      rm -f "$tmp"
    else
      skip "ingest/from-yaml/$base (no expected output)"
    fi
  done

  # --- from-toml ---
  for input_file in "$TEST_DIR"/from-toml/*.toml; do
    [[ -f "$input_file" ]] || continue
    base=$(basename "$input_file" .toml)

    local expected_yay="$TEST_DIR/from-toml/$base.yay"
    local expected_err="$TEST_DIR/from-toml/$base.error"

    if [[ -f "$expected_err" ]]; then
      if "$YAY" -f toml -t yay "$input_file" >/dev/null 2>&1; then
        fail "ingest/from-toml/$base (should have failed)"
      else
        pass "ingest/from-toml/$base (expected error)"
      fi
    elif [[ -f "$expected_yay" ]]; then
      tmp=$(mktemp)
      if "$YAY" -f toml -t yay "$input_file" >"$tmp" 2>/dev/null; then
        if diff -q "$tmp" "$expected_yay" >/dev/null 2>&1; then
          pass "ingest/from-toml/$base"
        else
          fail "ingest/from-toml/$base (output differs)"
          diff "$expected_yay" "$tmp" | head -5 | sed 's/^/    /'
        fi
      else
        fail "ingest/from-toml/$base (decode failed)"
      fi
      rm -f "$tmp"
    else
      skip "ingest/from-toml/$base (no expected output)"
    fi
  done

  # --- from-cbor ---
  for input_file in "$TEST_DIR"/from-cbor/*.cbor; do
    [[ -f "$input_file" ]] || continue
    base=$(basename "$input_file" .cbor)

    local expected_yay="$TEST_DIR/from-cbor/$base.yay"
    local expected_err="$TEST_DIR/from-cbor/$base.error"

    if [[ -f "$expected_err" ]]; then
      if "$YAY" -f cbor -t yay "$input_file" >/dev/null 2>&1; then
        fail "ingest/from-cbor/$base (should have failed)"
      else
        pass "ingest/from-cbor/$base (expected error)"
      fi
    elif [[ -f "$expected_yay" ]]; then
      tmp=$(mktemp)
      if "$YAY" -f cbor -t yay "$input_file" >"$tmp" 2>/dev/null; then
        if diff -q "$tmp" "$expected_yay" >/dev/null 2>&1; then
          pass "ingest/from-cbor/$base"
        else
          fail "ingest/from-cbor/$base (output differs)"
          diff "$expected_yay" "$tmp" | head -5 | sed 's/^/    /'
        fi
      else
        fail "ingest/from-cbor/$base (decode failed)"
      fi
      rm -f "$tmp"
    else
      skip "ingest/from-cbor/$base (no expected output)"
    fi
  done

  # --- from-json: strings keep the prefixes YSON reads as tags ---
  local text out
  for text in '# Hello' '#' '#12' '#NaN' '*cafe' '*' '!x' '!*x' '/x'; do
    if out=$(printf '{"t": "%s"}' "$text" | "$YAY" -f json -t yay 2>&1); then
      if [[ "$out" == "{t: \"$text\"}" ]]; then
        pass "ingest/from-json/$text"
      else
        fail "ingest/from-json/$text (output differs)" "$out"
      fi
    else
      fail "ingest/from-json/$text (decode failed)" "$out"
    fi
  done

  echo ""
}

# =============================================================================
# CLI Examples Tests (verify CLI.md examples work as documented)
# =============================================================================

run_cli_examples_tests() {
  echo "--- CLI Examples Tests ---"

  local examples_script="$SCRIPT_DIR/test-cli-examples.sh"
  if [[ ! -x "$examples_script" ]]; then
    echo -e "${YELLOW}SKIP${NC}: test-cli-examples.sh not found"
    ((SKIPPED++))
    return
  fi

  # Run the examples test script and capture output
  local output
  local status=0
  output=$("$examples_script" 2>&1) || status=$?

  # Display the output
  echo "$output"

  # Extract counts from the summary line (format: "Passed: N")
  local example_passed
  local example_failed
  example_passed=$(echo "$output" | grep "^Passed:" | sed 's/.*: //' | tr -d '[:space:]')
  example_failed=$(echo "$output" | grep "^Failed:" | sed 's/.*: //' | tr -d '[:space:]')

  # Add to our totals (handle empty values)
  PASSED=$((PASSED + ${example_passed:-0}))
  FAILED=$((FAILED + ${example_failed:-0}))

  if [[ $status -ne 0 ]]; then
    echo -e "${RED}CLI examples tests failed${NC}"
    # A run that stops before its summary still fails.
    if [[ ${example_failed:-0} -eq 0 ]]; then
      FAILED=$((FAILED + 1))
    fi
  fi

  echo ""
}

# =============================================================================
# Markdown Tests (markdown with YAY front matter)
# =============================================================================

run_markdown_tests() {
  echo "=== Markdown Tests (front matter and body ↔ {...head, content}) ==="
  echo ""

  local tmp name base expected_file input_file
  tmp=$(mktemp)
  trap 'rm -f "$tmp"' RETURN

  # Normalize: test/md/<name>.<variant>.md → test/md/<name>.md, and the
  # expected file is a strict fixed point.
  for input_file in "$TEST_DIR"/md/*.*.md; do
    [[ -f "$input_file" ]] || continue
    name=$(basename "$input_file")
    base=${name%%.*}
    expected_file="$TEST_DIR/md/$base.md"
    if [[ ! -f "$expected_file" ]]; then
      skip "markdown/normalize/$name (no expected .md file)"
      continue
    fi
    if "$YAY" -f md -t md "$input_file" >"$tmp" 2>/dev/null && cmp -s "$tmp" "$expected_file"; then
      pass "markdown/normalize/$name"
    else
      fail "markdown/normalize/$name (output differs from expected)"
      diff "$expected_file" "$tmp" | head -10 | sed 's/^/    /'
    fi
    if "$YAY" -f yd -t md "$expected_file" >"$tmp" 2>/dev/null && cmp -s "$tmp" "$expected_file"; then
      pass "markdown/idempotent/$base.md"
    else
      fail "markdown/idempotent/$base.md (not a strict fixed point)"
    fi
    if "$YAY" -f md -t yd "$input_file" >"$tmp" 2>/dev/null && cmp -s "$tmp" "$expected_file"; then
      pass "markdown/to-yd/$name"
    else
      fail "markdown/to-yd/$name (-t yd differs from -t md)"
    fi
    if "$YAY" --check -f yd -t yd "$expected_file" >/dev/null 2>&1; then
      pass "markdown/formatted/$base.md"
    else
      fail "markdown/formatted/$base.md (refused)"
    fi
    if ! cmp -s "$input_file" "$expected_file"; then
      if "$YAY" --check -f md -t md "$input_file" >/dev/null 2>&1; then
        fail "markdown/formatted/$name (should have been refused)"
      else
        pass "markdown/formatted/$name (refused)"
      fi
    fi
  done

  # Read as an object: test/from-md/<name>.md → <name>.yay, or <name>.error;
  # and the object writes back to markdown that reads as the same object.
  for input_file in "$TEST_DIR"/from-md/*.md; do
    [[ -f "$input_file" ]] || continue
    base=$(basename "$input_file" .md)
    if [[ -f "$TEST_DIR/from-md/$base.error" ]]; then
      if "$YAY" -f md -t yay "$input_file" >/dev/null 2>&1; then
        fail "markdown/from-md/$base (should have failed)"
      else
        pass "markdown/from-md/$base (expected error)"
      fi
      continue
    fi
    if "$YAY" -f md -t yay "$input_file" >"$tmp" 2>/dev/null && cmp -s "$tmp" "$TEST_DIR/from-md/$base.yay"; then
      pass "markdown/from-md/$base"
    else
      fail "markdown/from-md/$base (output differs)"
      diff "$TEST_DIR/from-md/$base.yay" "$tmp" | head -5 | sed 's/^/    /'
    fi
    if "$YAY" -f md -t yson "$input_file" | "$YAY" -f yson -t md >"$tmp" 2>/dev/null &&
      cmp -s <("$YAY" -f md -t yson "$tmp") <("$YAY" -f md -t yson "$input_file"); then
      pass "markdown/round-trip/$base"
    else
      fail "markdown/round-trip/$base (object → markdown → object changed)"
    fi
  done

  # Write from an object: test/to-md/<name>.yay → exactly <name>.md, and
  # <name>.md reads back as exactly <name>.yay; or <name>.error, refused.
  for input_file in "$TEST_DIR"/to-md/*.yay; do
    [[ -f "$input_file" ]] || continue
    base=$(basename "$input_file" .yay)
    if [[ -f "$TEST_DIR/to-md/$base.error" ]]; then
      if "$YAY" -f yay -t md "$input_file" >/dev/null 2>&1; then
        fail "markdown/to-md/$base (should have failed)"
      else
        pass "markdown/to-md/$base (expected error)"
      fi
      continue
    fi
    expected_file="$TEST_DIR/to-md/$base.md"
    if "$YAY" -f yay -t md "$input_file" >"$tmp" 2>/dev/null && cmp -s "$tmp" "$expected_file"; then
      pass "markdown/to-md/$base"
    else
      fail "markdown/to-md/$base (output differs)"
      diff "$expected_file" "$tmp" | head -5 | sed 's/^/    /'
    fi
    if "$YAY" -f yd -t yay "$expected_file" >"$tmp" 2>/dev/null && cmp -s "$tmp" "$input_file"; then
      pass "markdown/to-md/$base (reads back)"
    else
      fail "markdown/to-md/$base (does not read back as $base.yay)"
      diff "$input_file" "$tmp" | head -5 | sed 's/^/    /'
    fi
  done

  # Strict front matter: test/nmd/*.md must be refused by --from yd, which
  # also wants exactly one blank line between the front matter and a body.
  for input_file in "$TEST_DIR"/nmd/*.md; do
    [[ -f "$input_file" ]] || continue
    name=$(basename "$input_file")
    if "$YAY" -f yd --check "$input_file" >/dev/null 2>&1; then
      fail "markdown/strict/$name (should have been refused)"
    else
      pass "markdown/strict/$name (refused)"
    fi
    # Mehdown reads a document whose only fault is the blank line after the
    # front matter, and writes it with one.
    if [[ $name == *blank-line* ]]; then
      if "$YAY" -f md --check "$input_file" >/dev/null 2>&1 &&
        "$YAY" -f md -t yd "$input_file" >"$tmp" 2>/dev/null &&
        "$YAY" --check -f yd -t yd "$tmp" >/dev/null 2>&1; then
        pass "markdown/lenient/$name"
      else
        fail "markdown/lenient/$name (-f md refused it, or wrote it without one blank line)"
      fi
    fi
  done

  echo ""
}

# =============================================================================
# Check Tests (--check without -t: valid; with -t: unchanged by -t)
# =============================================================================

# expect_check <name> <want-exit> <stdin> <args...>
expect_check() {
  local name=$1 want=$2 input=$3 got
  shift 3
  printf '%s' "$input" | "$YAY" "$@" >/dev/null 2>&1
  got=$?
  if [[ $got -eq $want ]]; then
    pass "check/$name"
  else
    fail "check/$name (exit $got, wanted $want)"
  fi
}

# expect_message <name> <pattern> <stdin> <args...>: stderr matches
expect_message() {
  local name=$1 pattern=$2 input=$3 err
  shift 3
  err=$(printf '%s' "$input" | "$YAY" "$@" 2>&1 >/dev/null)
  if [[ $err == *$pattern* ]]; then
    pass "check/$name"
  else
    fail "check/$name (message)" "got: $err"
  fi
}

run_check_tests() {
  echo "=== Check Tests ==="
  local yay=$'a: 1\n' meh=$'a:   1\n' yd=$'---\nk: "v"\n---\n\nx\n' md=$'---\nk: v\n---\nx\n'

  # Without -t, --check checks validity only, as before
  expect_check "valid/yay" 0 "$yay" --check
  expect_check "invalid/yay" 1 "$meh" --check
  expect_check "valid/meh" 0 "$meh" --check -f meh
  expect_check "valid/ignores-formatting" 0 $'a: 1' --check
  expect_check "invalid/strict" 1 "$md" --check -f yd

  # Allowed pairs: unchanged input passes, changed input fails
  expect_check "pair/yay-yay" 0 "$yay" --check -t yay
  expect_check "pair/yay-yay/changed" 1 "$meh" --check -t yay
  expect_check "pair/meh-yay" 0 "$yay" --check -f meh -t yay
  expect_check "pair/meh-yay/changed" 1 "$meh" --check -f meh -t yay
  expect_check "pair/yd-yd" 0 "$yd" --check -f yd -t yd
  expect_check "pair/yd-md" 0 "$yd" --check -f yd -t md
  expect_check "pair/md-md" 0 "$yd" --check -f md -t md
  expect_check "pair/md-yd" 0 "$yd" --check -f md -t yd
  expect_check "pair/md-md/changed" 1 "$md" --check -f md -t md
  expect_check "pair/md-yd/changed" 1 "$md" --check -f md -t yd

  # With -t, -f still defaults to strict yay: invalid strict input is a parse
  # error, while -f meh reads it and reports the difference
  expect_message "pair/default-strict" 'Unexpected space' "$meh" --check -t yay
  expect_message "pair/meh-lenient" 'not formatted at line 1' "$meh" --check -f meh -t yay

  # The first difference, and a missing final newline, are reported plainly
  expect_message "message/line" 'line 1: expected "a: 1", found "a:   1"' "$meh" --check -f meh -t yay
  expect_message "message/final-newline" 'line 1: the last line must end with a newline' $'a: 1' --check -t yay

  # Refused pairs: output that can never be the same bytes as the input
  local pair f t
  for pair in json:yay yson:yay yaml:yay toml:yay json:json yson:yson yay:json yay:yson yay:md yay:yd md:yay yd:yay meh:md; do
    f=${pair%%:*} t=${pair#*:}
    expect_check "refused/$f-$t" 1 "$yay" --check -f "$f" -t "$t"
    expect_message "refused/$f-$t/message" "--check -f $f -t $t:" "$yay" --check -f "$f" -t "$t"
  done

  # --canonical is gone
  expect_check "canonical-removed" 1 "$yay" --canonical

  # -t without --check converts, as always
  local out
  out=$(printf '%s' "$yay" | "$YAY" -t yson 2>/dev/null)
  if [[ $out == $'{\n  "a": "#1"\n}' ]]; then
    pass "check/convert-without-check"
  else
    fail "check/convert-without-check" "got: $out"
  fi
  out=$(printf '%s' "$meh" | "$YAY" -t yay 2>/dev/null)
  if [[ $out == "a: 1" ]]; then
    pass "check/format-without-check"
  else
    fail "check/format-without-check" "got: $out"
  fi

  echo ""
}

# =============================================================================
# Main
# =============================================================================

# Parse arguments
TESTS_TO_RUN=""
while [[ $# -gt 0 ]]; do
  case $1 in
  idempotence | idem)
    TESTS_TO_RUN="$TESTS_TO_RUN idempotence"
    ;;
  reformat | ref)
    TESTS_TO_RUN="$TESTS_TO_RUN reformat"
    ;;
  roundtrip | rt)
    TESTS_TO_RUN="$TESTS_TO_RUN roundtrip"
    ;;
  output | out)
    TESTS_TO_RUN="$TESTS_TO_RUN output"
    ;;
  error | err)
    TESTS_TO_RUN="$TESTS_TO_RUN error"
    ;;
  transcode | tc)
    TESTS_TO_RUN="$TESTS_TO_RUN transcode"
    ;;
  ingest | ing)
    TESTS_TO_RUN="$TESTS_TO_RUN ingest"
    ;;
  examples | ex)
    TESTS_TO_RUN="$TESTS_TO_RUN examples"
    ;;
  markdown | md)
    TESTS_TO_RUN="$TESTS_TO_RUN markdown"
    ;;
  check | chk)
    TESTS_TO_RUN="$TESTS_TO_RUN check"
    ;;
  all | "")
    TESTS_TO_RUN="idempotence reformat roundtrip output error transcode ingest examples markdown check"
    ;;
  *)
    echo "Unknown test category: $1"
    echo "Usage: $0 [idempotence|reformat|roundtrip|output|error|transcode|ingest|examples|markdown|check|all]"
    exit 1
    ;;
  esac
  shift
done

# Default to all tests
if [[ -z "$TESTS_TO_RUN" ]]; then
  TESTS_TO_RUN="idempotence reformat roundtrip output error transcode ingest examples markdown check"
fi

# Run selected tests
for test in $TESTS_TO_RUN; do
  case $test in
  idempotence) run_idempotence_tests ;;
  reformat) run_reformat_tests ;;
  roundtrip) run_roundtrip_tests ;;
  output) run_output_format_tests ;;
  error) run_error_tests ;;
  transcode) run_transcode_tests ;;
  ingest) run_ingest_tests ;;
  examples) run_cli_examples_tests ;;
  markdown) run_markdown_tests ;;
  check) run_check_tests ;;
  esac
done

# Summary
echo "=== Summary ==="
echo -e "${GREEN}Passed${NC}: $PASSED"
echo -e "${RED}Failed${NC}: $FAILED"
echo -e "${YELLOW}Skipped${NC}: $SKIPPED"

if [[ $FAILED -gt 0 ]]; then
  exit 1
fi
exit 0
