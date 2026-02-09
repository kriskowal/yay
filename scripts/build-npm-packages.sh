#!/usr/bin/env bash
# Build platform-specific npm packages by compiling and copying binaries into js/binyay-*/bin/
#
# This script cross-compiles the yay binary for each platform and populates
# the js/binyay-{platform}-{arch}/bin/ directories.
#
# Usage: ./scripts/build-npm-packages.sh
#
# Prerequisites:
#   - Rust toolchain with cross-compilation targets installed
#   - For Linux ARM: aarch64-linux-gnu-gcc
#   - For Linux x64 musl: musl-tools

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
JS_DIR="$ROOT_DIR/js"
RUST_DIR="$ROOT_DIR/rust"

# Platform -> Rust target mapping
# Format: npm_platform:rust_target
PLATFORMS="
darwin-arm64:aarch64-apple-darwin
darwin-x64:x86_64-apple-darwin
linux-arm64:aarch64-unknown-linux-gnu
linux-x64:x86_64-unknown-linux-gnu
win32-x64:x86_64-pc-windows-msvc
"

cd "$RUST_DIR"

for mapping in $PLATFORMS; do
	npm_platform="${mapping%%:*}"
	rust_target="${mapping##*:}"
	pkg_dir="$JS_DIR/binyay-${npm_platform}"

	if [[ ! -d "$pkg_dir" ]]; then
		echo "Warning: Package directory not found: $pkg_dir"
		continue
	fi

	echo "Building binyay-${npm_platform} (${rust_target})..."

	# Check if target is installed
	if ! rustup target list --installed | grep -q "^${rust_target}$"; then
		echo "  Skipping: target ${rust_target} not installed"
		echo "  Run: rustup target add ${rust_target}"
		continue
	fi

	# Build
	if cargo build --release --target "$rust_target" -p binyay 2>/dev/null; then
		# Copy binary
		if [[ "$npm_platform" == win32-* ]]; then
			bin_name="yay.exe"
		else
			bin_name="yay"
		fi

		src="$RUST_DIR/target/${rust_target}/release/${bin_name}"
		if [[ -f "$src" ]]; then
			cp "$src" "$pkg_dir/bin/"
			chmod +x "$pkg_dir/bin/${bin_name}"
			echo "  Built and copied to $pkg_dir/bin/${bin_name}"
		else
			echo "  Warning: Binary not found at $src"
		fi
	else
		echo "  Warning: Build failed for ${rust_target}"
	fi
done

echo ""
echo "Done!"
echo ""
echo "To publish all packages:"
echo "  for pkg in js/binyay*/; do (cd \"\$pkg\" && npm publish --access public); done"
