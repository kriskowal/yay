// References, against the cases of the design
// (doc/design/references/cases/): each .yay parses to its .yson, with a
// reference as "$uri", and each .nay and .meh is refused, MEH's leniency not
// being YAY's. Then the model, and the CapData bridge without Endo (see
// capdata.test.js for the round trip through @endo/marshal).

import { describe, it } from "node:test";
import assert from "node:assert";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "node:url";

import {
  parseYay,
  tagged,
  isTagged,
  reference,
  isReference,
  Reference,
  ysonToCapData,
  capDataToYson,
} from "./yay.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const casesDir = path.join(
  __dirname,
  "..",
  "..",
  "doc",
  "design",
  "references",
  "cases",
);

function cases(ext) {
  return fs
    .readdirSync(casesDir)
    .filter((f) => f.endsWith(`.${ext}`))
    .sort();
}

/** A string as YSON writes it: escaped when it begins with ! through /. */
function ysonString(s) {
  return s.length > 0 && s[0] >= "!" && s[0] <= "/" ? `!${s}` : s;
}

/** The value as the cases' YSON spells it. */
function toYson(value) {
  if (isReference(value)) return `$${value.reference}`;
  if (isTagged(value)) {
    return { "#tag": ysonString(value.tag), payload: toYson(value.payload) };
  }
  if (value instanceof Uint8Array) {
    return (
      "*" + Array.from(value, (b) => b.toString(16).padStart(2, "0")).join("")
    );
  }
  if (typeof value === "bigint") return value < 0n ? `${value}` : `+${value}`;
  if (typeof value === "string") return ysonString(value);
  if (Array.isArray(value)) return value.map(toYson);
  if (value !== null && typeof value === "object") {
    const out = {};
    for (const [k, v] of Object.entries(value)) {
      out[/^[#*!]/.test(k) ? `!${k}` : k] = toYson(v);
    }
    return out;
  }
  return value;
}

describe("references: the design's cases", () => {
  for (const name of cases("yay")) {
    it(name, () => {
      const yay = fs.readFileSync(path.join(casesDir, name), "utf8");
      const yson = fs.readFileSync(
        path.join(casesDir, name.replace(/\.yay$/, ".yson")),
        "utf8",
      );
      assert.deepStrictEqual(toYson(parseYay(yay, name)), JSON.parse(yson));
    });
  }
  for (const name of [...cases("nay"), ...cases("meh")]) {
    it(`${name} is refused`, () => {
      const yay = fs.readFileSync(path.join(casesDir, name), "utf8");
      assert.throws(() => parseYay(yay, name));
    });
  }
});

describe("references: errors", () => {
  const refused = [
    ["a: <HTTPS://x>\n", /Uppercase scheme "HTTPS" in reference/],
    ["a: <https://x y>\n", /Unexpected space in reference at 1:14/],
    ['a: <https://x"y>\n', /Unexpected character """ in reference at 1:14/],
    ["a: <https://x/%2g>\n", /Bad percent-encoding in reference .* at 1:15/],
    ["a: <https://x\n", /Unmatched angle bracket/],
    ["a: <https://x>(t)\n", /Expected one space before a tag/],
    ["a: <https://x>rgb\n", /Unexpected character "r"/],
    ["<https://x>: 1\n", /Unexpected character ":" at 1:12/],
    ["a: <./x>\n", /hex/],
  ];
  for (const [yay, message] of refused) {
    it(JSON.stringify(yay), () => {
      assert.throws(() => parseYay(yay, "test.yay"), message);
    });
  }
});

describe("references: the model", () => {
  it("is a frozen {reference}", () => {
    const r = reference("https://x");
    assert.ok(r instanceof Reference);
    assert.ok(Object.isFrozen(r));
    assert.strictEqual(r.reference, "https://x");
    assert.ok(isReference(r));
    assert.ok(!isReference("https://x"));
    assert.ok(!isReference({ reference: "https://x" }));
  });
  it("is equal by text, never to a string", () => {
    assert.deepStrictEqual(parseYay("<https://x>\n"), reference("https://x"));
    assert.notDeepStrictEqual(reference("https://x/"), reference("https://x"));
    assert.notDeepStrictEqual(reference("a:%2f"), reference("a:%2F"));
    assert.notDeepStrictEqual(parseYay("<https://x>\n"), "https://x");
    assert.notDeepStrictEqual(reference("https://x"), {
      reference: "https://x",
    });
  });
  it("tagged is a tagged value over the reference", () => {
    assert.deepStrictEqual(
      parseYay("<ocapn://a.tcp> (Counter)\n"),
      tagged("Counter", reference("ocapn://a.tcp")),
    );
  });
});

describe("references: the CapData bridge", () => {
  it("gives each distinct URI one slot, in order of first appearance", () => {
    const yson = JSON.stringify({
      a: "$ocapn://b.tcp",
      b: ["$ocapn://a.tcp", "$ocapn://b.tcp", "!$not-a-reference"],
      c: { "#tag": "Counter", payload: "$ocapn://a.tcp" },
      d: "+1",
    });
    const capData = ysonToCapData(yson);
    assert.deepStrictEqual(capData.slots, ["ocapn://b.tcp", "ocapn://a.tcp"]);
    assert.deepStrictEqual(JSON.parse(capData.body.slice(1)), {
      a: "$0",
      b: ["$1", "$0", "!$not-a-reference"],
      c: { "#tag": "Counter", payload: "$1" },
      d: "+1",
    });
    assert.deepStrictEqual(
      JSON.parse(capDataToYson(capData)),
      JSON.parse(yson),
    );
  });
  it("drops smallcaps' alleged interface on the way back", () => {
    const yson = capDataToYson({
      body: '#["$0.Alleged: Counter","$1"]',
      slots: ["ocapn://a.tcp", "https://x"],
    });
    assert.deepStrictEqual(JSON.parse(yson), ["$ocapn://a.tcp", "$https://x"]);
  });
  it("refuses a reference without a scheme, a promise, a missing slot", () => {
    assert.throws(() => ysonToCapData('"$0"'), /needs a scheme/);
    assert.throws(() => ysonToCapData('"$https://x y"'), /space/);
    assert.throws(
      () => capDataToYson({ body: '#"&0"', slots: ["https://x"] }),
      /promise/,
    );
    assert.throws(() => capDataToYson({ body: '#"$1"', slots: [] }), /slot/);
  });
});

describe("reference constructor and CapData edges", () => {
  const bad = [
    "",
    "not a uri",
    "HTTPS://x",
    "urn:a b",
    "https://a>b",
    "https://x/%zz",
    "./a.yay",
  ];
  it("refuses a string that is not a reference's text", () => {
    for (const uri of bad) {
      assert.throws(() => reference(uri), TypeError, JSON.stringify(uri));
      assert.throws(() => new Reference(uri), TypeError, JSON.stringify(uri));
    }
  });
  it("accepts what the parser reads", () => {
    for (const uri of ["urn:x", "https://example.com/a?b=c#d", "urn:%C3%A9"]) {
      assert.equal(reference(uri).reference, uri);
    }
  });
  it("refuses a CapData slot that is not a reference", () => {
    for (const uri of bad) {
      assert.throws(
        () => capDataToYson({ body: '#"$0"', slots: [uri] }),
        TypeError,
        JSON.stringify(uri),
      );
    }
  });
});
