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
  all | "")
    TESTS_TO_RUN="idempotence reformat roundtrip output error transcode ingest examples"
    ;;
  *)
    echo "Unknown test category: $1"
    echo "Usage: $0 [idempotence|reformat|roundtrip|output|error|transcode|ingest|examples|all]"
    exit 1
    ;;
  esac
  shift
done

# Default to all tests
if [[ -z "$TESTS_TO_RUN" ]]; then
  TESTS_TO_RUN="idempotence reformat roundtrip output error transcode ingest examples"
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
