// References: MEH's leniency is not YAY's, so the corpus's MEH reference
// cases are refused as YAY. Then the messages, the model, and the CapData
// bridge without Endo (see capdata.test.js for the round trip through
// @endo/marshal). The corpus reference cases in test/yay and test/nay are
// read with the rest of the corpus by yay.test.js.

import { describe, it } from "node:test";
import assert from "node:assert";
import fs from "fs";
import path from "path";

import {
  parseYay,
  tagged,
  reference,
  isReference,
  Reference,
  ysonToCapData,
  capDataToYson,
} from "./yay.js";
import { casesDir, cases } from "./test-helpers.js";

const mehDir = casesDir("test", "meh");

describe("references: MEH's reference cases are not YAY", () => {
  const names = cases(mehDir, "meh", "reference-");
  it("finds the cases", () => {
    assert.ok(names.length >= 2, `MEH reference cases in ${mehDir}`);
  });
  for (const name of names) {
    it(`${name} is refused`, () => {
      const meh = fs.readFileSync(path.join(mehDir, name), "utf8");
      assert.throws(() => parseYay(meh, name));
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
