#!/bin/bash
# Run tests for Java implementation

set -ueo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."
DIR="$ROOT/java"

if [[ ! -d "$DIR" ]]; then
	echo "Java directory not found"
	exit 2
fi

# Find build command
BUILD_CMD=""
if command -v gradle >/dev/null 2>&1; then
	BUILD_CMD="gradle"
elif [[ -f "$DIR/gradlew" ]]; then
	BUILD_CMD="./gradlew"
elif command -v mvn >/dev/null 2>&1; then
	BUILD_CMD="mvn"
else
	echo "Skipping: gradle/maven not installed"
	exit 2
fi

cd "$DIR"

echo "Running tests..."
$BUILD_CMD test
