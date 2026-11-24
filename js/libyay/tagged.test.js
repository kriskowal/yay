// Tagged values: MEH's leniency is not YAY's, so the corpus's MEH tagged cases
// are refused as YAY. Then the model. The corpus tagged cases in test/yay and
// test/nay are read with the rest of the corpus by yay.test.js.

import { describe, it } from "node:test";
import assert from "node:assert";
import fs from "fs";
import path from "path";

import "@endo/init";
import { makeTagged, passStyleOf } from "@endo/pass-style";

import { parseYay, tagged, isTagged } from "./yay.js";
import { casesDir, cases } from "./test-helpers.js";

const mehDir = casesDir("test", "meh");

describe("tagged values: MEH's tagged cases are not YAY", () => {
  const names = cases(mehDir, "meh", "tagged-");
  it("finds the cases", () => {
    assert.ok(names.length >= 3, `MEH tagged cases in ${mehDir}`);
  });
  for (const name of names) {
    it(`${name} is refused`, () => {
      const meh = fs.readFileSync(path.join(mehDir, name), "utf8");
      assert.throws(() => parseYay(meh, { filename: name }));
    });
  }
});

describe("tagged values: the model", () => {
  it("has the pre-hardened shape of Endo's makeTagged", () => {
    const actual = parseYay("(answer)\nvalue: 42\n");
    const expected = makeTagged("answer", { value: 42n });

    assert.strictEqual(
      Object.getPrototypeOf(actual),
      Object.getPrototypeOf(expected),
    );
    assert.deepStrictEqual(
      Object.getOwnPropertyDescriptors(actual),
      Object.getOwnPropertyDescriptors(expected),
    );
    assert.deepStrictEqual(Reflect.ownKeys(actual), Reflect.ownKeys(expected));
    assert.ok(Object.isFrozen(actual));
    assert.ok(!Object.isFrozen(actual.payload));
    assert.deepStrictEqual(Object.keys(actual), ["payload"]);
    assert.ok(isTagged(expected));

    harden(actual);
    assert.strictEqual(passStyleOf(actual), "tagged");
  });
  it("becomes pass data for scalars, records, arrays, and tags", () => {
    const value = parseYay(`
array:
  - 42
  - 1.5
  - "text"
  - true
  - null
  - "payload" (tag)
`);
    harden(value);
    assert.strictEqual(passStyleOf(value), "copyRecord");
    assert.strictEqual(passStyleOf(value.array), "copyArray");
    assert.deepStrictEqual(
      value.array.map((item) => passStyleOf(item)),
      ["bigint", "number", "string", "boolean", "null", "tagged"],
    );
  });
  it("requires a string tag", () => {
    assert.throws(() => tagged(1, null), TypeError);
  });
  it("equals a tagged value with an equal tag and payload", () => {
    assert.deepStrictEqual(tagged("a", [1n]), tagged("a", [1n]));
    assert.notDeepStrictEqual(tagged("a", 1n), tagged("b", 1n));
    assert.notDeepStrictEqual(tagged("a", 1n), tagged("a", 2n));
  });
  it("never equals its payload or a plain object", () => {
    assert.notDeepStrictEqual(tagged("a", 1n), 1n);
    assert.notDeepStrictEqual(tagged("a", 1n), { payload: 1n });
    assert.ok(!isTagged({ tag: "a", payload: 1n }));
  });
  it("nests innermost first", () => {
    assert.deepStrictEqual(
      parseYay('"t" (datetime) (utc)\n'),
      tagged("utc", tagged("datetime", "t")),
    );
    assert.deepStrictEqual(
      parseYay("(outer) (inner)\n- 1\n"),
      tagged("outer", tagged("inner", [1n])),
    );
  });
  it("reads the tag's escapes", () => {
    assert.deepStrictEqual(parseYay("3 (a\\)b)\n"), tagged("a)b", 3n));
    assert.deepStrictEqual(
      parseYay('"x" (back\\\\slash\\t\\/\\u{41})\n'),
      tagged("back\\slash\t/A", "x"),
    );
    for (const bad of ['1 (a\\"b)\n', "1 (a\\(b)\n", "1 (\\u{d800})\n"]) {
      assert.throws(() => parseYay(bad), bad);
    }
  });
});

describe("tagged values: blocks", () => {
  it("keeps a tagged block string's lines literal", () => {
    assert.deepStrictEqual(
      parseYay("a: (md) `\n  - not an item\n  # not a comment\nb: 1\n"),
      { a: tagged("md", "- not an item\n# not a comment\n"), b: 1n },
    );
    assert.deepStrictEqual(parseYay("- (md) `\n  - not an item\n- 2\n"), [
      tagged("md", "\n- not an item\n"),
      2n,
    ]);
  });
  it("refuses a prefix with no block below", () => {
    for (const bad of [
      "(t)\n",
      "(t) `\n  x\n",
      "a: (t)\nb: 1\n",
      "a: (t) 1\n",
      "- a: (t)\n- 1\n",
      "- (t)\n- 1\n",
      "x:\n  a: (t)\n  b: 1\n",
    ]) {
      assert.throws(() => parseYay(bad), JSON.stringify(bad));
    }
  });
  it("reads a block string whose text holds parentheses", () => {
    assert.deepStrictEqual(parseYay("` a (b) c\n"), "a (b) c\n");
    assert.deepStrictEqual(parseYay("a: `\n  x (y)\n"), { a: "x (y)\n" });
  });
});
