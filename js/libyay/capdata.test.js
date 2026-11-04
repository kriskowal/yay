// References through Endo: the bridge turns binyay's YSON into CapData,
// each distinct URI a slot, and @endo/marshal's makeMarshal decodes it with
// a convertSlotToVal that makes a remotable for each URI. Skipped when the
// dev dependencies (@endo/init, @endo/marshal) are not installed.

import { describe, it } from "node:test";
import assert from "node:assert";

import {
  parseYay,
  isTagged,
  isReference,
  ysonToCapData,
  capDataToYson,
} from "./yay.js";

let endo;
try {
  await import("@endo/init");
  const { makeMarshal, makeTagged, passStyleOf, Far } =
    await import("@endo/marshal");
  endo = { makeMarshal, makeTagged, passStyleOf, Far };
} catch {
  endo = undefined;
}

// A YAY document, and the YSON `yay -f yay -t yson` writes for it.
const yay = `\
counter: <ocapn://a2ef69ddd5f8.tcp-testing-only?host=127.0.0.1&port=22046> (Counter)
peers: [<ocapn://b.tcp>, <ocapn://a2ef69ddd5f8.tcp-testing-only?host=127.0.0.1&port=22046>]
name: '$not-a-reference'
`;
const yson = `{
  "counter": {
    "#tag": "Counter",
    "payload": "$ocapn://a2ef69ddd5f8.tcp-testing-only?host=127.0.0.1&port=22046"
  },
  "peers": [
    "$ocapn://b.tcp",
    "$ocapn://a2ef69ddd5f8.tcp-testing-only?host=127.0.0.1&port=22046"
  ],
  "name": "!$not-a-reference"
}`;

/** A marshal whose slots are URIs, recording each URI it is asked for. */
function makeUriMarshal() {
  const { makeMarshal, Far } = endo;
  const requested = [];
  const byUri = new Map();
  const uriOf = new Map();
  const convertSlotToVal = (uri, iface) => {
    requested.push(uri);
    if (!byUri.has(uri)) {
      const remotable = Far(iface ?? "Reference", {});
      byUri.set(uri, remotable);
      uriOf.set(remotable, uri);
    }
    return byUri.get(uri);
  };
  const convertValToSlot = (val) => uriOf.get(val);
  const marshal = makeMarshal(convertValToSlot, convertSlotToVal, {
    serializeBodyFormat: "smallcaps",
  });
  return { ...marshal, requested };
}

describe("references: a CapData round trip", { skip: !endo }, () => {
  it("decodes each URI to a remotable, and back", () => {
    const { makeTagged, passStyleOf } = endo;
    const { fromCapData, toCapData, requested } = makeUriMarshal();
    const capData = ysonToCapData(yson);

    // A repeated URI shares one slot
    assert.deepStrictEqual(capData.slots, [
      "ocapn://a2ef69ddd5f8.tcp-testing-only?host=127.0.0.1&port=22046",
      "ocapn://b.tcp",
    ]);

    const value = fromCapData(capData);
    assert.deepStrictEqual(requested, capData.slots);
    assert.strictEqual(value.name, "$not-a-reference");
    const [b, a] = value.peers;
    assert.strictEqual(passStyleOf(a), "remotable");
    assert.strictEqual(passStyleOf(b), "remotable");
    assert.notStrictEqual(a, b);

    // A tagged reference is makeTagged over the remotable
    assert.strictEqual(passStyleOf(value.counter), "tagged");
    assert.strictEqual(value.counter[Symbol.toStringTag], "Counter");
    assert.strictEqual(value.counter.payload, a);
    assert.deepStrictEqual(value.counter, makeTagged("Counter", a));

    // The same document as YAY reads to the same structure
    const parsed = parseYay(yay);
    assert.ok(isTagged(parsed.counter));
    assert.ok(isReference(parsed.counter.payload));
    assert.strictEqual(parsed.counter.payload.reference, capData.slots[0]);
    assert.deepStrictEqual(
      parsed.peers.map((r) => r.reference),
      [capData.slots[1], capData.slots[0]],
    );

    // Encoding it again gives back the YSON
    const again = toCapData(value);
    assert.deepStrictEqual(again.slots, capData.slots);
    assert.deepStrictEqual(JSON.parse(capDataToYson(again)), JSON.parse(yson));
  });
});
