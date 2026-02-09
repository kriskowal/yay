#!/bin/bash
# Generate transcoding fixtures from test/yay/*.yay sources.
#
# For each .yay file, attempts conversion to YAML, TOML, and CBOR.
# On success: writes the output to test/<format>/<basename>.<ext>
# On failure: writes the stderr to test/<format>/<basename>.error
#
# Usage: scripts/gen-transcode-fixtures.sh

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
YAY="$ROOT_DIR/rust/target/release/yay"
YAY_DIR="$ROOT_DIR/test/yay"

if [[ ! -x "$YAY" ]]; then
  echo "yay binary not found at $YAY"
  echo "Run 'cd rust && cargo build --release' first"
  exit 1
fi

generate_text_format() {
  local format="$1"
  local ext="$2"
  local out_dir="$ROOT_DIR/test/$format"

  mkdir -p "$out_dir"

  local ok=0
  local err=0

  for yay_file in "$YAY_DIR"/*.yay; do
    [[ -f "$yay_file" ]] || continue
    local base
    base=$(basename "$yay_file" .yay)

    local out_file="$out_dir/$base.$ext"
    local err_file="$out_dir/$base.error"

    # Remove stale pair files
    rm -f "$out_file" "$err_file"

    local tmp_out tmp_err
    tmp_out=$(mktemp)
    tmp_err=$(mktemp)

    if "$YAY" -t "$format" "$yay_file" >"$tmp_out" 2>"$tmp_err"; then
      mv "$tmp_out" "$out_file"
      rm -f "$tmp_err"
      ((ok++))
    else
      # Write the error message (strip the filename prefix for portability)
      cat "$tmp_err" >"$err_file"
      rm -f "$tmp_out" "$tmp_err"
      ((err++))
    fi
  done

  echo "  $format: $ok ok, $err errors"
}

generate_cbor_format() {
  local out_dir="$ROOT_DIR/test/cbor"

  mkdir -p "$out_dir"

  local ok=0
  local err=0

  for yay_file in "$YAY_DIR"/*.yay; do
    [[ -f "$yay_file" ]] || continue
    local base
    base=$(basename "$yay_file" .yay)

    local out_file="$out_dir/$base.cbor"
    local diag_file="$out_dir/$base.diag"
    local err_file="$out_dir/$base.error"

    # Remove stale pair files
    rm -f "$out_file" "$diag_file" "$err_file"

    local tmp_err
    tmp_err=$(mktemp)

    if "$YAY" -t cbor "$yay_file" -o "$out_file" 2>"$tmp_err"; then
      rm -f "$tmp_err"
      # Generate diagnostic notation companion
      "$YAY" -f cbor -t cbor-diag "$out_file" >"$diag_file" 2>/dev/null
      ((ok++))
    else
      rm -f "$out_file"
      cat "$tmp_err" >"$err_file"
      rm -f "$tmp_err"
      ((err++))
    fi
  done

  echo "  cbor: $ok ok, $err errors (with .diag companions)"
}

echo "Generating transcode fixtures from test/yay/*.yay ..."
echo ""

generate_text_format "yaml" "yaml"
generate_text_format "toml" "toml"
generate_cbor_format

echo ""
echo "Done."
