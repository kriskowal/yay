#!/bin/bash
# Check that binyay's platform packages under js/ are ready to publish.
#
# For each platform package, this checks that:
#   - its version is binyay's, and binyay's optionalDependencies name that
#     exact version;
#   - `npm pack` would ship bin/yay (bin/yay.exe for Windows), executable;
#   - the binary is built for the package's OS and CPU;
#   - the binary prints `yay <version>` when it can run here, or holds the
#     version string when it cannot.
#
# Usage: ./scripts/check-npm-packages.sh [platform...]
#   With no platforms, all five are checked. JS_DIR overrides js/.

set -ueo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
JS_DIR="${JS_DIR:-$(dirname "$HERE")/js}"

PLATFORMS=(darwin-arm64 darwin-x64 linux-arm64 linux-x64 win32-x64)
if [[ $# -gt 0 ]]; then
  PLATFORMS=("$@")
fi

for tool in node npm od; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "error: $tool is needed to check the npm packages" >&2
    exit 1
  fi
done

# json_get <file> <expression over the parsed object p>
json_get() {
  node -e 'const p = require(process.argv[1]); console.log(eval(process.argv[2]) ?? "")' \
    "$1" "$2"
}

# bytes <file> <offset> <count>: hex bytes, space-separated
bytes() {
  od -An -tx1 -j "$2" -N "$3" "$1" | tr -s ' \n' ' ' | sed 's/^ //; s/ $//'
}

# The OS and CPU a binary is built for, as an npm platform name.
binary_platform() {
  local file="$1" magic
  magic=$(bytes "$file" 0 4)
  case "$magic" in
  "7f 45 4c 46")
    case "$(bytes "$file" 18 2)" in
    "3e 00") echo linux-x64 ;;
    "b7 00") echo linux-arm64 ;;
    *) echo "linux-unknown" ;;
    esac
    ;;
  "cf fa ed fe")
    case "$(bytes "$file" 4 4)" in
    "07 00 00 01") echo darwin-x64 ;;
    "0c 00 00 01") echo darwin-arm64 ;;
    *) echo "darwin-unknown" ;;
    esac
    ;;
  "4d 5a"*)
    local word pe
    # The PE header's offset is a little-endian word at 0x3c.
    read -r -a word <<<"$(bytes "$file" 60 4)"
    pe=$((16#${word[3]}${word[2]}${word[1]}${word[0]}))
    case "$(bytes "$file" $((pe + 4)) 2)" in
    "64 86") echo win32-x64 ;;
    *) echo "win32-unknown" ;;
    esac
    ;;
  *) echo "unknown ($magic)" ;;
  esac
}

host=$(node -p 'process.platform + "-" + process.arch')
binyay_json="$JS_DIR/binyay/package.json"
version=$(json_get "$binyay_json" 'p.version')
errors=0

error() {
  echo "  error: $1" >&2
  errors=$((errors + 1))
}

for platform in "${PLATFORMS[@]}"; do
  name="binyay-$platform"
  dir="$JS_DIR/$name"
  bin_name=yay
  if [[ "$platform" == win32-* ]]; then
    bin_name=yay.exe
  fi
  echo "$name:"

  if [[ ! -f "$dir/package.json" ]]; then
    error "no package at $dir"
    continue
  fi

  pkg_version=$(json_get "$dir/package.json" 'p.version')
  if [[ "$pkg_version" != "$version" ]]; then
    error "version is $pkg_version, but binyay is $version"
  fi
  dep_version=$(json_get "$binyay_json" "p.optionalDependencies?.[\"$name\"]")
  if [[ "$dep_version" != "$version" ]]; then
    error "binyay's optionalDependencies give $name ${dep_version:-no version}, not $version"
  fi

  bin="$dir/bin/$bin_name"
  if [[ ! -s "$bin" ]]; then
    error "bin/$bin_name is missing or empty: the package would publish without a binary"
    continue
  fi
  if [[ ! -x "$bin" ]]; then
    error "bin/$bin_name is not executable"
  fi

  # What npm would publish, ignore files and all.
  if ! packed=$(cd "$dir" && npm pack --dry-run --json 2>/dev/null |
    node -e '
      let s = "";
      process.stdin.on("data", d => (s += d)).on("end", () => {
        const files = JSON.parse(s)[0].files;
        const f = files.find(f => f.path === process.argv[1]);
        console.log(f ? (f.mode & 0o111 ? "executable" : "not-executable") : "missing");
      });
    ' "bin/$bin_name"); then
    error "npm pack --dry-run failed"
  elif [[ "$packed" == missing ]]; then
    error "npm pack leaves out bin/$bin_name (an ignore file?)"
  elif [[ "$packed" != executable ]]; then
    error "npm pack ships bin/$bin_name without its executable bit"
  fi

  built_for=$(binary_platform "$bin")
  if [[ "$built_for" != "$platform" ]]; then
    error "bin/$bin_name is built for $built_for, not $platform"
    continue
  fi

  if [[ "$platform" == "$host" ]]; then
    printed=$("$bin" --version 2>&1) || true
    if [[ "$printed" != "yay $version" ]]; then
      error "bin/$bin_name --version prints '$printed', not 'yay $version'"
    else
      echo "  bin/$bin_name: $built_for, prints 'yay $version'"
    fi
  elif grep -qaF "$version" "$bin"; then
    echo "  bin/$bin_name: $built_for, holds '$version' (not run on $host)"
  else
    error "bin/$bin_name does not hold the version string '$version'"
  fi
done

if [[ $errors -gt 0 ]]; then
  echo "$errors problem(s) found; do not publish." >&2
  exit 1
fi
echo "All ${#PLATFORMS[@]} package(s) are ready for binyay $version."
