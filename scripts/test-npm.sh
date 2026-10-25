#!/bin/bash
# Test scripts/check-npm-packages.sh against copies of the npm packages:
# it must refuse a platform package that would publish without a working
# binary, and pass the host's package once it holds the release binary.

set -ueo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
ROOT_DIR=$(dirname "$HERE")

for tool in node npm cargo; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "Skipping: $tool not installed"
    exit 2
  fi
done

host=$(node -p 'process.platform + "-" + process.arch')
case "$host" in
linux-x64) other=linux-arm64 ;;
linux-arm64 | darwin-arm64 | darwin-x64 | win32-x64) other=linux-x64 ;;
*)
  echo "Skipping: binyay has no package for $host"
  exit 2
  ;;
esac
bin_name=yay
if [[ "$host" == win32-* ]]; then
  bin_name=yay.exe
fi

echo "Building release binary..."
(cd "$ROOT_DIR/rust" && cargo build --release -p binyay)
yay="${CARGO_TARGET_DIR:-$ROOT_DIR/rust/target}/release/$bin_name"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/js"
cp -R "$ROOT_DIR/js/binyay" "$ROOT_DIR/js/binyay-"* "$work/js/"
export JS_DIR="$work/js"
pkg="$JS_DIR/binyay-$host"
mkdir -p "$pkg/bin"
rm -f "$pkg/bin/"*

failed=0

# expect <pass|fail> <description> [platform]
expect() {
  local want="$1" desc="$2" platform="${3:-$host}" got=pass
  "$HERE/check-npm-packages.sh" "$platform" >"$work/out" 2>&1 || got=fail
  if [[ "$got" == "$want" ]]; then
    echo "PASS: $desc"
  else
    echo "FAIL: $desc (check said $got)"
    sed 's/^/    /' "$work/out"
    failed=$((failed + 1))
  fi
}

expect fail "refuses an empty package"

cp "$yay" "$pkg/bin/$bin_name"
chmod +x "$pkg/bin/$bin_name"
expect pass "passes the package holding the release binary"

printf '*\n!.gitignore\n' >"$pkg/bin/.gitignore"
expect fail "refuses a package whose binary npm leaves out"
rm "$pkg/bin/.gitignore"

chmod -x "$pkg/bin/$bin_name"
expect fail "refuses a binary that is not executable"
chmod +x "$pkg/bin/$bin_name"

cp "$pkg/package.json" "$work/package.json"
node -e '
  const fs = require("fs");
  const p = JSON.parse(fs.readFileSync(process.argv[1]));
  p.version = "0.0.0";
  fs.writeFileSync(process.argv[1], JSON.stringify(p, null, 2) + "\n");
' "$pkg/package.json"
expect fail "refuses a version binyay does not depend on"
cp "$work/package.json" "$pkg/package.json"

mkdir -p "$JS_DIR/binyay-$other/bin"
cp "$yay" "$JS_DIR/binyay-$other/bin/yay"
expect fail "refuses a binary built for another platform" "$other"

printf '#!/bin/sh\necho yay 0.0.0\n' >"$pkg/bin/$bin_name"
expect fail "refuses a stand-in that is not a binary"

if [[ $failed -gt 0 ]]; then
  echo "$failed npm package check test(s) failed"
  exit 1
fi
echo "All npm package check tests passed"
