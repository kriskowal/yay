#!/usr/bin/env node

// Replace the Node wrapper bin/yay with the platform's native yay, so that
// npm's bin link runs yay directly, with no Node process in between. This is
// what esbuild's install script does (lib/npm/node-install.ts, v0.28.2).
// Beyond speed, it matters on macOS, where a child that Node starts keeps no
// file descriptor beyond stdio, so the wrapper cannot pass on a shell's
// process substitution, such as `yay -s <(echo hi)`.
//
// Anything that fails leaves the wrapper in place and prints one warning:
// installation never fails here. With --ignore-scripts, or where this script
// does not run, the wrapper stays and still works.

const { execFileSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const PLATFORMS = {
  "darwin-arm64": "binyay-darwin-arm64",
  "darwin-x64": "binyay-darwin-x64",
  "linux-arm64": "binyay-linux-arm64",
  "linux-x64": "binyay-linux-x64",
  "win32-x64": "binyay-win32-x64",
};

const root = path.join(__dirname, "..");
const target = path.join(root, "bin", "yay");
const { version } = require(path.join(root, "package.json"));

// Find the platform package's binary, as bin/yay does.
function nativeBinary() {
  const platform = `${process.platform}-${process.arch}`;
  const pkg = PLATFORMS[platform];
  if (!pkg) {
    throw new Error(
      `no prebuilt binary for ${platform}; you may build from source: https://github.com/kriskowal/yay`,
    );
  }
  let pkgPath;
  try {
    pkgPath = require.resolve(`${pkg}/package.json`, { paths: [root] });
  } catch (e) {
    throw new Error(`${pkg} is not installed, so the yay command may not work`);
  }
  const binName = process.platform === "win32" ? "yay.exe" : "yay";
  return path.join(path.dirname(pkgPath), "bin", binName);
}

// Check that the binary runs and is this package's version.
function checkVersion(binary) {
  let stdout;
  try {
    // A pipe for stderr too: esbuild found that an unwritable stderr (Node
    // from the Snap Store) crashes the install otherwise.
    stdout = execFileSync(binary, ["--version"], { stdio: "pipe" })
      .toString()
      .trim();
  } catch (e) {
    throw new Error(
      `${binary} --version failed: ${String(e.message).split("\n")[0]}`,
    );
  }
  if (stdout !== `yay ${version}`) {
    throw new Error(
      `${binary} --version reports ${JSON.stringify(stdout)}, not "yay ${version}"`,
    );
  }
}

// Put the binary at bin/yay: a hard link where possible, otherwise a copy,
// made under a temporary name in bin/ and renamed over the wrapper.
function replace(binary) {
  const temp = path.join(root, "bin", `.yay-${process.pid}`);
  try {
    try {
      fs.linkSync(binary, temp);
    } catch (e) {
      fs.copyFileSync(binary, temp);
      fs.chmodSync(temp, 0o755);
    }
    fs.renameSync(temp, target);
  } finally {
    // Rename does nothing, and leaves the temporary name behind, when both
    // names are already the same file, as when this script runs again.
    try {
      fs.unlinkSync(temp);
    } catch (e) {
      // Already renamed, or never made.
    }
  }
}

// Yarn 2 and later do not support binary files in bin, and Yarn may run
// install scripts in the same place from several platforms (esbuild).
function isYarn() {
  return /\byarn\//.test(process.env.npm_config_user_agent || "");
}

try {
  const binary = nativeBinary();
  // On Windows, npm's .cmd shims run bin/yay with node, so the wrapper stays.
  if (process.platform !== "win32" && !isYarn()) {
    checkVersion(binary);
    replace(binary);
  }
} catch (e) {
  console.warn(`Warning: binyay: ${e.message}; bin/yay stays the Node wrapper`);
}
