#!/bin/bash
# Fill each js/binyay-<platform>/bin/ with the yay binary for its platform,
# check every package, and print the publish commands.
#
# Usage:
#   ./scripts/build-npm-packages.sh                       # build with cargo
#   ./scripts/build-npm-packages.sh --from-release v1.1.1 # take a release's assets
#
# Building cross-compiles each Rust target, which must be installed
# (`rustup target add <target>`) along with its linker. A missing target or
# a failed build stops the script.
#
# --from-release takes the binaries from the GitHub release's assets,
# yay-<target>.tar.gz (yay-<target>.zip for Windows), as
# .github/workflows/release.yml makes them. The tag must match
# js/binyay/package.json. YAY_RELEASE_URL overrides the download base.
#
# Either way the script ends with scripts/check-npm-packages.sh, and prints
# the publish commands only when every package passes.

set -ueo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
ROOT_DIR=$(dirname "$HERE")
JS_DIR="$ROOT_DIR/js"
RUST_DIR="$ROOT_DIR/rust"
RELEASE_URL="${YAY_RELEASE_URL:-https://github.com/kriskowal/yay/releases/download}"

# npm platform:Rust target
PLATFORMS=(
  darwin-arm64:aarch64-apple-darwin
  darwin-x64:x86_64-apple-darwin
  linux-arm64:aarch64-unknown-linux-gnu
  linux-x64:x86_64-unknown-linux-gnu
  win32-x64:x86_64-pc-windows-msvc
)

die() {
  echo "error: $1" >&2
  exit 1
}

release=""
while [[ $# -gt 0 ]]; do
  case "$1" in
  --from-release)
    [[ $# -ge 2 ]] || die "--from-release needs a tag, such as v1.1.1"
    release="$2"
    shift 2
    ;;
  -h | --help)
    sed -n '2,19p' "$0" | sed 's/^# \{0,1\}//'
    exit 0
    ;;
  *) die "unknown argument: $1" ;;
  esac
done

version=$(node -p 'require(process.argv[1]).version' "$JS_DIR/binyay/package.json")
if [[ -n "$release" && "$release" != "v$version" ]]; then
  die "--from-release $release, but js/binyay/package.json is at $version"
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

for mapping in "${PLATFORMS[@]}"; do
  platform="${mapping%%:*}"
  target="${mapping##*:}"
  pkg_dir="$JS_DIR/binyay-$platform"
  bin_name=yay
  archive=tar.gz
  if [[ "$platform" == win32-* ]]; then
    bin_name=yay.exe
    archive=zip
  fi

  [[ -f "$pkg_dir/package.json" ]] || die "no package at $pkg_dir"
  mkdir -p "$pkg_dir/bin"
  rm -f "$pkg_dir/bin/yay" "$pkg_dir/bin/yay.exe"

  if [[ -n "$release" ]]; then
    asset="yay-$target.$archive"
    echo "Fetching $asset from $release for binyay-$platform..."
    curl -fsSL -o "$work/$asset" "$RELEASE_URL/$release/$asset" ||
      die "could not fetch $RELEASE_URL/$release/$asset"
    mkdir "$work/$target"
    if [[ "$archive" == zip ]]; then
      unzip -q "$work/$asset" -d "$work/$target"
    else
      tar -xzf "$work/$asset" -C "$work/$target"
    fi
    src="$work/$target/$bin_name"
  else
    if ! rustup target list --installed | grep -qx "$target"; then
      die "Rust target $target is not installed; run: rustup target add $target"
    fi
    echo "Building binyay-$platform ($target)..."
    (cd "$RUST_DIR" && cargo build --release --target "$target" -p binyay) ||
      die "cargo build failed for $target"
    src="${CARGO_TARGET_DIR:-$RUST_DIR/target}/$target/release/$bin_name"
  fi

  [[ -f "$src" ]] || die "no $bin_name at $src"
  cp "$src" "$pkg_dir/bin/$bin_name"
  chmod +x "$pkg_dir/bin/$bin_name"
  echo "  copied to js/binyay-$platform/bin/$bin_name"
done

echo ""
"$HERE/check-npm-packages.sh"

echo ""
echo "To publish binyay $version, platform packages first and binyay last:"
for mapping in "${PLATFORMS[@]}"; do
  echo "  (cd js/binyay-${mapping%%:*} && npm publish --access public)"
done
echo "  (cd js/binyay && npm publish --access public)"
