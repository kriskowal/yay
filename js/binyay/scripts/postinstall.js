#!/usr/bin/env node

// Verify that the platform-specific binary package was installed
const PLATFORMS = {
  "darwin-arm64": "binyay-darwin-arm64",
  "darwin-x64": "binyay-darwin-x64",
  "linux-arm64": "binyay-linux-arm64",
  "linux-x64": "binyay-linux-x64",
  "win32-x64": "binyay-win32-x64",
};

const platform = `${process.platform}-${process.arch}`;
const pkg = PLATFORMS[platform];

if (!pkg) {
  console.warn(
    `Warning: binyay does not have a prebuilt binary for ${platform}`,
  );
  console.warn(
    "You may need to build from source: https://github.com/kriskowal/yay",
  );
  process.exit(0);
}

try {
  require.resolve(`${pkg}/package.json`);
} catch (e) {
  console.warn(`Warning: Failed to install ${pkg}`);
  console.warn("The yay command may not work on this system.");
}
