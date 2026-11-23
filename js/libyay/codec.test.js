// A caller-configured reference codec and portable bytes, under Endo. The
// decode hook makes a remotable for each URI; the encode hook finds its URI
// again. With the immutable bytes mode, every value kind is pass data.

import { describe, it } from "node:test";
import assert from "node:assert";

import "@endo/init";
import { Far, passStyleOf } from "@endo/marshal";

import { parseYay, ysonToCapData, capDataToYson } from "./yay.js";

function makeUriCodec() {
  const byUri = new Map();
  const uriOf = new Map();
  return {
    decodeReference(uri) {
      if (!byUri.has(uri)) {
        const remotable = Far("Reference", {});
        byUri.set(uri, remotable);
        uriOf.set(remotable, uri);
      }
      return byUri.get(uri);
    },
    encodeReference(value) {
      const uri = uriOf.get(value);
      if (uri === undefined) {
        throw new TypeError("The encode hook saw a value it did not decode");
      }
      return uri;
    },
  };
}

const yay = `\
nothing: null
integer: 42
float: 1.5
truth: true
text: "hello"
list: [1, 2]
record: {a: 1}
bytes: <00ff>
tagged: 10 (seconds)
peer: <ocapn://a.tcp>
peerAgain: <ocapn://a.tcp>
`;

describe("a configured reference codec", () => {
  it("makes every value kind pass data", () => {
    const codec = makeUriCodec();
    const value = harden(parseYay(yay, { ...codec, bytes: "immutable" }));
    assert.strictEqual(passStyleOf(value), "copyRecord");
    assert.strictEqual(passStyleOf(value.integer), "bigint");
    assert.strictEqual(passStyleOf(value.float), "number");
    assert.strictEqual(passStyleOf(value.list), "copyArray");
    assert.strictEqual(passStyleOf(value.bytes), "byteArray");
    assert.strictEqual(passStyleOf(value.tagged), "tagged");
    assert.strictEqual(passStyleOf(value.peer), "remotable");
    assert.strictEqual(value.peer, value.peerAgain);
  });

  it("encodes a decoded reference back to the same text", () => {
    const codec = makeUriCodec();
    const value = parseYay(yay, codec);
    assert.strictEqual(codec.encodeReference(value.peer), "ocapn://a.tcp");
  });

  it("round trips YSON through decoded slots and the encode hook", () => {
    const codec = makeUriCodec();
    const yson = JSON.stringify({
      a: "$ocapn://a.tcp",
      b: ["$https://x", "$ocapn://a.tcp"],
      c: { "#tag": "Counter", payload: "$https://x" },
      d: "!$not-a-reference",
    });
    const capData = ysonToCapData(yson, codec);
    assert.strictEqual(capData.slots.length, 2);
    assert.ok(capData.slots.every((slot) => passStyleOf(slot) === "remotable"));
    assert.strictEqual(capDataToYson(capData, codec), yson);
  });
});

describe("portable bytes under Endo's shim", () => {
  it("makes the locked pass-style's immutable byteArray", () => {
    const bytes = parseYay("<00ff>\n", { bytes: "immutable" });
    assert.ok(bytes instanceof ArrayBuffer);
    assert.ok(bytes.immutable);
    assert.ok(Object.isFrozen(bytes));
    assert.strictEqual(passStyleOf(bytes), "byteArray");
    assert.deepStrictEqual(
      new Uint8Array(bytes.slice(0)),
      new Uint8Array([0, 255]),
    );
    const empty = parseYay("<>\n", { bytes: "immutable" });
    assert.strictEqual(passStyleOf(empty), "byteArray");
    assert.strictEqual(empty.byteLength, 0);
    const { block } = parseYay("block: >\n  01 02\n", {
      bytes: "immutable",
    });
    assert.strictEqual(passStyleOf(block), "byteArray");
    assert.deepStrictEqual(
      new Uint8Array(block.slice(0)),
      new Uint8Array([1, 2]),
    );
  });

  it("keeps the plain Uint8Array as the default", () => {
    const bytes = parseYay("<00ff>\n");
    assert.ok(bytes instanceof Uint8Array);
    assert.deepStrictEqual(bytes, new Uint8Array([0, 255]));
  });
});
