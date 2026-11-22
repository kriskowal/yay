// Tagged values, against the cases of the design
// (doc/design/tagged-values-g/cases/): each .yay parses to its .json, with a
// tagged value as {"#tag": tag, "payload": payload} and bytes as "*hex", and
// each .nay and .meh is refused, MEH's leniency not being YAY's.

import { describe, it } from "node:test";
import assert from "node:assert";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "node:url";

import "@endo/init";
import { makeTagged, passStyleOf } from "@endo/pass-style";

import { parseYay, tagged, isTagged } from "./yay.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const casesDir = path.join(
  __dirname,
  "..",
  "..",
  "doc",
  "design",
  "tagged-values-g",
  "cases",
);

function cases(ext) {
  return fs
    .readdirSync(casesDir)
    .filter((f) => f.endsWith(`.${ext}`))
    .sort();
}

/** The value as the cases' JSON spells it. */
function desugar(value) {
  if (isTagged(value)) {
    return {
      "#tag": value[Symbol.toStringTag],
      payload: desugar(value.payload),
    };
  }
  if (value instanceof Uint8Array) {
    return (
      "*" + Array.from(value, (b) => b.toString(16).padStart(2, "0")).join("")
    );
  }
  if (Array.isArray(value)) return value.map(desugar);
  // JSON's numbers are floats
  if (typeof value === "bigint") return Number(value);
  // The prototype's JSON spells these as strings or bare words
  if (typeof value === "number" && Number.isNaN(value)) return "NaN";
  if (value === Infinity) return "Infinity";
  if (value === -Infinity) return "-Infinity";
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([k, v]) => [k, desugar(v)]),
    );
  }
  return value;
}

describe("tagged values: the design's accepted cases", () => {
  const names = cases("yay");
  it("finds the cases", () => assert.ok(names.length > 0));
  for (const name of names) {
    it(name, () => {
      const src = fs.readFileSync(path.join(casesDir, name), "utf8");
      const json = fs
        .readFileSync(path.join(casesDir, name.replace(/\.yay$/, ".json")))
        .toString()
        .replace("-Infinity", '"-Infinity"');
      assert.deepStrictEqual(desugar(parseYay(src, name)), JSON.parse(json));
    });
  }
});

describe("tagged values: the design's refused cases", () => {
  const names = [...cases("nay"), ...cases("meh")];
  it("finds the cases", () => assert.ok(names.length > 0));
  for (const name of names) {
    it(name, () => {
      const src = fs.readFileSync(path.join(casesDir, name), "utf8");
      assert.throws(() => parseYay(src, name));
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
