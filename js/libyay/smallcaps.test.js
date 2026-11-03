// A round trip through Endo: binyay's YSON for a tagged value is the
// smallcaps encoding of Endo's makeTagged, and the value parseYay reads is
// the one decodeFromSmallcaps makes. Skipped when the dev dependencies
// (@endo/init, @endo/marshal) are not installed.

import { describe, it } from "node:test";
import assert from "node:assert";

import { parseYay, isTagged } from "./yay.js";

let endo;
try {
  await import("@endo/init");
  const { makeMarshal, makeTagged, passStyleOf } =
    await import("@endo/marshal");
  const { toCapData, fromCapData } = makeMarshal(undefined, undefined, {
    serializeBodyFormat: "smallcaps",
  });
  endo = { toCapData, fromCapData, makeTagged, passStyleOf };
} catch {
  endo = undefined;
}

// Each YAY document, and the YSON `yay -f yay -t yson` writes for it.
const cases = [
  ["10 ()\n", '{\n  "#tag": "",\n  "payload": "+10"\n}'],
  ["'x' (a\\)b)\n", '{\n  "#tag": "a)b",\n  "payload": "x"\n}'],
  [
    "[1.5, true] (!x)\n",
    '{\n  "#tag": "!!x",\n  "payload": [\n    1.5,\n    true\n  ]\n}',
  ],
];

describe("tagged values: a smallcaps round trip", { skip: !endo }, () => {
  for (const [yay, yson] of cases) {
    it(JSON.stringify(yay), () => {
      const { toCapData, fromCapData, makeTagged, passStyleOf } = endo;
      const value = parseYay(yay);
      assert.ok(isTagged(value));

      // decodeFromSmallcaps of binyay's YSON is makeTagged with an equal
      // tag and payload
      const decoded = fromCapData({ body: `#${yson}`, slots: [] });
      assert.strictEqual(passStyleOf(decoded), "tagged");
      assert.strictEqual(decoded[Symbol.toStringTag], value.tag);
      assert.deepStrictEqual(decoded.payload, value.payload);

      // encodeToSmallcaps of it reads back as the same YSON
      const { body } = toCapData(makeTagged(value.tag, value.payload));
      assert.ok(body.startsWith("#"));
      assert.deepStrictEqual(JSON.parse(body.slice(1)), JSON.parse(yson));
      const again = fromCapData({ body, slots: [] });
      assert.strictEqual(again[Symbol.toStringTag], value.tag);
      assert.deepStrictEqual(again.payload, value.payload);
    });
  }
});
