// Tests for scripts/postinstall.js against a fake platform package.
// Run with `node --test js/binyay/test/postinstall.test.js` (Node 18 or
// later), as scripts/test-npm.sh does.

const { test } = require("node:test");
const assert = require("node:assert");
const { execFileSync, spawnSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const here = path.join(__dirname, "..");
const { version } = require(path.join(here, "package.json"));
const platformPackage = `binyay-${process.platform}-${process.arch}`;
const wrapper = fs.readFileSync(path.join(here, "bin", "yay"), "utf8");

// Lay out node_modules/binyay (this package) and, unless `reports` is null,
// node_modules/<platform package> with a bin/yay that prints `reports` for
// --version and "native" otherwise.
function install(t, reports) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "binyay-postinstall-"));
  t.after(() => {
    fs.chmodSync(path.join(dir, "node_modules", "binyay", "bin"), 0o755);
    fs.rmSync(dir, { recursive: true, force: true });
  });
  const pkg = path.join(dir, "node_modules", "binyay");
  for (const sub of ["bin", "scripts"]) {
    fs.mkdirSync(path.join(pkg, sub), { recursive: true });
  }
  fs.copyFileSync(
    path.join(here, "package.json"),
    path.join(pkg, "package.json"),
  );
  fs.copyFileSync(path.join(here, "bin", "yay"), path.join(pkg, "bin", "yay"));
  fs.chmodSync(path.join(pkg, "bin", "yay"), 0o755);
  fs.copyFileSync(
    path.join(here, "scripts", "postinstall.js"),
    path.join(pkg, "scripts", "postinstall.js"),
  );
  if (reports !== null) {
    const native = path.join(dir, "node_modules", platformPackage);
    fs.mkdirSync(path.join(native, "bin"), { recursive: true });
    fs.writeFileSync(
      path.join(native, "package.json"),
      JSON.stringify({ name: platformPackage, version }),
    );
    fs.writeFileSync(
      path.join(native, "bin", "yay"),
      `#!/bin/sh\nif [ "$1" = --version ]; then echo '${reports}'; else echo native; fi\n`,
      { mode: 0o755 },
    );
  }
  return pkg;
}

function postinstall(pkg) {
  const env = { ...process.env };
  delete env.npm_config_user_agent;
  return spawnSync(
    process.execPath,
    [path.join(pkg, "scripts", "postinstall.js")],
    {
      cwd: pkg,
      env,
      encoding: "utf8",
    },
  );
}

function warnings(result) {
  return result.stderr
    .split("\n")
    .filter((line) => line.startsWith("Warning:"));
}

const unix = process.platform !== "win32";

test(
  "replaces bin/yay when the binary reports this version",
  { skip: !unix },
  (t) => {
    const pkg = install(t, `yay ${version}`);
    const result = postinstall(pkg);
    assert.strictEqual(result.status, 0, result.stderr);
    assert.strictEqual(result.stderr, "");
    const bin = path.join(pkg, "bin", "yay");
    assert.notStrictEqual(fs.readFileSync(bin, "utf8"), wrapper);
    assert.strictEqual(execFileSync(bin).toString(), "native\n");
    assert.ok(fs.statSync(bin).mode & 0o100, "executable");
    assert.deepStrictEqual(fs.readdirSync(path.join(pkg, "bin")), ["yay"]);
    // Running again leaves the same binary and nothing else.
    const again = postinstall(pkg);
    assert.strictEqual(again.status, 0, again.stderr);
    assert.strictEqual(execFileSync(bin).toString(), "native\n");
    assert.deepStrictEqual(fs.readdirSync(path.join(pkg, "bin")), ["yay"]);
  },
);

function keepsWrapper(t, pkg, pattern) {
  const result = postinstall(pkg);
  assert.strictEqual(result.status, 0, result.stderr);
  const lines = warnings(result);
  assert.strictEqual(lines.length, 1, result.stderr);
  assert.match(lines[0], pattern);
  assert.match(lines[0], /bin\/yay stays the Node wrapper/);
  assert.strictEqual(
    fs.readFileSync(path.join(pkg, "bin", "yay"), "utf8"),
    wrapper,
  );
  assert.deepStrictEqual(fs.readdirSync(path.join(pkg, "bin")), ["yay"]);
}

test("keeps the wrapper when the platform package is missing", (t) => {
  keepsWrapper(
    t,
    install(t, null),
    new RegExp(`${platformPackage} is not installed`),
  );
});

test(
  "keeps the wrapper when the binary reports another version",
  { skip: !unix },
  (t) => {
    keepsWrapper(
      t,
      install(t, "yay 0.0.1"),
      /reports "yay 0\.0\.1", not "yay /,
    );
  },
);

test("keeps the wrapper when bin/ cannot be written", { skip: !unix }, (t) => {
  const pkg = install(t, `yay ${version}`);
  fs.chmodSync(path.join(pkg, "bin"), 0o555);
  keepsWrapper(t, pkg, /EACCES/);
});

test("the wrapper fails clearly when the platform package is missing", (t) => {
  const pkg = install(t, null);
  const result = spawnSync(
    process.execPath,
    [path.join(pkg, "bin", "yay"), "--version"],
    {
      encoding: "utf8",
    },
  );
  assert.strictEqual(result.status, 1);
  assert.match(
    result.stderr,
    new RegExp(`Failed to find binary package: ${platformPackage}`),
  );
});

test(
  "the wrapper fails clearly when the binary is missing",
  { skip: !unix },
  (t) => {
    const pkg = install(t, `yay ${version}`);
    fs.rmSync(path.join(pkg, "..", platformPackage, "bin", "yay"));
    const result = spawnSync(
      process.execPath,
      [path.join(pkg, "bin", "yay"), "--version"],
      {
        encoding: "utf8",
      },
    );
    assert.strictEqual(result.status, 1);
    assert.match(result.stderr, /Missing binary: .*yay/);
  },
);
