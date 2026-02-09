#!/bin/bash
# shellcheck disable=SC2001  # sed is used intentionally for complex patterns
# Extract and verify examples from CLI.md
#
# This script:
# 1. Extracts bash examples from CLI.md
# 2. Runs each example and verifies the output matches any documented output
# 3. Creates temporary test files as needed
#
# Example format in CLI.md:
#   ```bash
#   yay --check input.yay
#   ```
#
# Examples with expected output:
#   ```bash
#   echo '{"a": 1}' | yay -f json
#   # Output:
#   # a: 1
#   ```

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."
CLI_MD="$ROOT/CLI.md"
YAY="$ROOT/rust/target/release/yay"

# Create temp directory for test files
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

exit_code=0
tests_run=0
tests_passed=0

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
# YELLOW='\033[0;33m'  # Unused but kept for future use
NC='\033[0m' # No Color

# Check if yay binary exists
if [[ ! -x "$YAY" ]]; then
  echo "Error: yay binary not found at $YAY"
  echo "Run 'cargo build --release' in rust/ first"
  exit 1
fi

# Create sample test files in temp directory
create_test_files() {
  # config.yay - simple config file
  cat >"$TEMP_DIR/config.yay" <<'EOF'
name: "example"
version: 1
enabled: true
EOF

  # data.yay - data file for conversion
  cat >"$TEMP_DIR/data.yay" <<'EOF'
items:
- 1
- 2
- 3
EOF

  # wide-file.yay - file with long lines for wrap testing
  cat >"$TEMP_DIR/wide-file.yay" <<'EOF'
description: "This is a configuration file with some settings"
items: ["alpha", "beta", "gamma"]
EOF

  # input.yay - generic input file
  cat >"$TEMP_DIR/input.yay" <<'EOF'
key: "value"
number: 42
EOF

  # strings.yay - file with only strings (no integers, for JSON conversion)
  cat >"$TEMP_DIR/strings.yay" <<'EOF'
key: "value"
items: ["alpha", "beta"]
EOF

  # data.json - JSON input file
  cat >"$TEMP_DIR/data.json" <<'EOF'
{"a": 1, "b": 2}
EOF

  # Create a configs directory with some .yay files
  mkdir -p "$TEMP_DIR/configs"
  echo 'a: 1' >"$TEMP_DIR/configs/one.yay"
  echo 'b: 2' >"$TEMP_DIR/configs/two.yay"
}

# Parse a bash code block and extract command and expected output
# Sets: CMD, EXPECTED_OUTPUT, HAS_EXPECTED, EXPECTS_ERROR
parse_example() {
  local content="$1"
  CMD=""
  EXPECTED_OUTPUT=""
  HAS_EXPECTED=false
  EXPECTS_ERROR=false

  local in_output=false
  while IFS= read -r line; do
    if [[ "$line" =~ ^#\ Output: ]]; then
      in_output=true
      continue
    fi
    if [[ "$line" =~ ^#\ Error: ]]; then
      in_output=true
      EXPECTS_ERROR=true
      continue
    fi

    if [[ "$in_output" == true ]]; then
      if [[ "$line" =~ ^#\  ]]; then
        # Strip "# " prefix
        local output_line="${line#\# }"
        if [[ -z "$EXPECTED_OUTPUT" ]]; then
          EXPECTED_OUTPUT="$output_line"
        else
          EXPECTED_OUTPUT="$EXPECTED_OUTPUT"$'\n'"$output_line"
        fi
        HAS_EXPECTED=true
      fi
    elif [[ -n "$line" && ! "$line" =~ ^# ]]; then
      # Command line (not a comment)
      # Strip inline comments (but preserve quoted strings)
      local cmd_part
      cmd_part=$(echo "$line" | sed 's/[[:space:]]*#[^"'"'"']*$//')
      if [[ -z "$CMD" ]]; then
        CMD="$cmd_part"
      fi
    fi
  done <<<"$content"
}

# Check if a command is testable (uses yay, not a loop or complex construct)
is_testable() {
  local cmd="$1"

  # Skip loops and complex constructs
  if [[ "$cmd" =~ ^for\  || "$cmd" =~ ^while\  || "$cmd" =~ ^if\  ]]; then
    return 1
  fi

  # Skip commands that are just setting up (cd, cargo, brew, npm)
  if [[ "$cmd" =~ ^cd\  || "$cmd" =~ ^cargo\  || "$cmd" =~ ^brew\  || "$cmd" =~ ^npm\  ]]; then
    return 1
  fi

  # Must involve yay
  if [[ ! "$cmd" =~ yay ]]; then
    return 1
  fi

  return 0
}

# Transform command for testing
# - Replace 'yay' with full path
# - Replace file references with temp dir paths
# - Handle redirections
transform_command() {
  local cmd="$1"

  # Replace file paths with temp dir FIRST (before yay replacement)
  # Order matters - replace more specific patterns first
  cmd="${cmd//\.\/configs\//$TEMP_DIR/configs/}"
  cmd="${cmd//config.yay/$TEMP_DIR/config.yay}"
  cmd="${cmd//config.go/$TEMP_DIR/config.go}"
  cmd="${cmd//data.yay/$TEMP_DIR/data.yay}"
  cmd="${cmd//data.yson/$TEMP_DIR/data.yson}"
  cmd="${cmd//data.json/$TEMP_DIR/data.json}"
  cmd="${cmd//wide-file.yay/$TEMP_DIR/wide-file.yay}"
  cmd="${cmd//strings.yay/$TEMP_DIR/strings.yay}"
  cmd="${cmd//input.yay/$TEMP_DIR/input.yay}"
  cmd="${cmd//output.yay/$TEMP_DIR/output.yay}"
  # Replace standalone "." with temp dir (for "yay --check .")
  cmd=$(echo "$cmd" | sed "s| \\.| $TEMP_DIR|g")

  # Replace 'yay' command with full path (only at word boundaries)
  # Use sed to be more precise - only match 'yay' as a command, not in paths
  cmd=$(echo "$cmd" | sed "s|^yay |$YAY |")

  # Remove output redirections for testing (we capture stdout)
  cmd=$(echo "$cmd" | sed 's/ *> *[^ ]*$//')

  echo "$cmd"
}

# Run a single example and check result
run_example() {
  local original_cmd="$1"
  local expected="$2"
  local has_expected="$3"
  local line_num="$4"
  local expects_error="$5"

  ((tests_run++))

  local cmd
  cmd=$(transform_command "$original_cmd")

  # Run the command
  local output
  local status=0
  output=$(eval "$cmd" 2>&1) || status=$?

  # For --check commands, we just verify exit code
  if [[ "$original_cmd" =~ --check ]]; then
    if [[ $status -eq 0 ]]; then
      ((tests_passed++))
      echo -e "${GREEN}PASS${NC}: line $line_num: $original_cmd"
      return 0
    else
      echo -e "${RED}FAIL${NC}: line $line_num: $original_cmd"
      echo "  Command failed with exit code $status"
      echo "  Output: $output"
      exit_code=1
      return 1
    fi
  fi

  # For commands expected to fail with specific error output
  if [[ "$expects_error" == true ]]; then
    if [[ $status -ne 0 ]]; then
      # Command failed as expected, check output if specified
      if [[ "$has_expected" == true ]]; then
        if [[ "$output" == "$expected" ]]; then
          ((tests_passed++))
          echo -e "${GREEN}PASS${NC}: line $line_num: $original_cmd (expected error)"
          return 0
        else
          echo -e "${RED}FAIL${NC}: line $line_num: $original_cmd"
          echo "  Expected error output:"
          echo "$expected" | sed 's/^/    /'
          echo "  Got:"
          echo "$output" | sed 's/^/    /'
          exit_code=1
          return 1
        fi
      else
        # Just expected to fail, no specific output required
        ((tests_passed++))
        echo -e "${GREEN}PASS${NC}: line $line_num: $original_cmd (expected error)"
        return 0
      fi
    else
      echo -e "${RED}FAIL${NC}: line $line_num: $original_cmd"
      echo "  Expected command to fail, but it succeeded"
      echo "  Output: $output"
      exit_code=1
      return 1
    fi
  fi

  # For commands with expected output, verify it
  if [[ "$has_expected" == true ]]; then
    if [[ "$output" == "$expected" ]]; then
      ((tests_passed++))
      echo -e "${GREEN}PASS${NC}: line $line_num: $original_cmd"
      return 0
    else
      echo -e "${RED}FAIL${NC}: line $line_num: $original_cmd"
      echo "  Expected:"
      echo "$expected" | sed 's/^/    /'
      echo "  Got:"
      echo "$output" | sed 's/^/    /'
      exit_code=1
      return 1
    fi
  fi

  # For commands without expected output, just verify they succeed
  if [[ $status -eq 0 ]]; then
    ((tests_passed++))
    echo -e "${GREEN}PASS${NC}: line $line_num: $original_cmd"
    return 0
  else
    echo -e "${RED}FAIL${NC}: line $line_num: $original_cmd"
    echo "  Command failed with exit code $status"
    echo "  Output: $output"
    exit_code=1
    return 1
  fi
}

# Extract and run all bash examples from CLI.md
main() {
  create_test_files

  local in_code_block=false
  local is_bash_block=false
  local content=""
  local block_start_line=0
  local line_num=0

  while IFS= read -r line; do
    ((line_num++))

    if [[ "$in_code_block" == true ]]; then
      if [[ "$line" == '```' ]]; then
        # End of code block
        if [[ "$is_bash_block" == true && -n "$content" ]]; then
          parse_example "$content"
          if [[ -n "$CMD" ]] && is_testable "$CMD"; then
            run_example "$CMD" "$EXPECTED_OUTPUT" "$HAS_EXPECTED" "$block_start_line" "$EXPECTS_ERROR"
          fi
        fi
        in_code_block=false
        is_bash_block=false
        content=""
      else
        if [[ -z "$content" ]]; then
          content="$line"
        else
          content="$content"$'\n'"$line"
        fi
      fi
    elif [[ "$line" == '```bash' ]]; then
      in_code_block=true
      is_bash_block=true
      content=""
      block_start_line=$line_num
    elif [[ "$line" =~ ^\`\`\` ]]; then
      in_code_block=true
      is_bash_block=false
      content=""
    fi
  done <"$CLI_MD"

  echo ""
  echo "=== Summary ==="
  # Plain text for machine parsing, colored for human reading
  echo "Passed: $tests_passed"
  echo "Failed: $((tests_run - tests_passed))"
  echo "Total: $tests_run"

  exit $exit_code
}

main
