// YSON is smallcaps: each YAY document here goes through binyay, live, as
// `yay -f yay -t yson`, and @endo/marshal's smallcaps decoder reads the
// YSON as the passable the document means, which its encoder writes back
// as the same YSON. Bytes are the exception for now: YSON writes them
// "*<hex>", as endojs/endo#2871 gives byteArrays, and @endo/marshal 1.10.0
// still refuses "*".
//
// The test needs the @endo/init and @endo/marshal dev dependencies and a
// built binyay: YAY_BIN, or rust/target/{release,debug}/yay. A local run
// without binyay skips and says why.

import { describe, it } from "node:test";
import assert from "node:assert";
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";

import "@endo/init";
import { passStyleOf, Far } from "@endo/marshal";

import { parseYay, isTagged, ysonToCapData } from "./yay.js";
import { makeSmallcapsMarshal } from "./endo-test-helper.js";

const yayBin = [
  process.env.YAY_BIN,
  fileURLToPath(new URL("../../rust/target/release/yay", import.meta.url)),
  fileURLToPath(new URL("../../rust/target/debug/yay", import.meta.url)),
].find((path) => path && existsSync(path));
const missing = yayBin
  ? undefined
  : "no binyay: set YAY_BIN or run cargo build -p binyay in rust/";

const skip = missing;
if (skip) {
  console.log(`smallcaps.test.js: skipped: ${missing}`);
}

/** The YSON binyay writes for a YAY document. */
const toYson = (yay) =>
  execFileSync(yayBin, ["-f", "yay", "-t", "yson"], {
    input: yay,
    encoding: "utf8",
  });

/** A passable as plain data: its pass style at every level, and its value. */
function show(value) {
  const style = passStyleOf(value);
  switch (style) {
    case "tagged":
      return [style, value[Symbol.toStringTag], show(value.payload)];
    case "copyArray":
      return [style, ...value.map(show)];
    case "copyRecord":
      return [
        style,
        Object.fromEntries(
          Object.entries(value).map(([key, item]) => [key, show(item)]),
        ),
      ];
    case "bigint":
      return [style, `${value}`];
    case "number":
      return [style, Object.is(value, -0) ? "-0" : `${value}`];
    case "remotable":
      return [style, value[Symbol.toStringTag]];
    default:
      return [style, value];
  }
}

const big = (n) => ["bigint", n];
const num = (n) => ["number", n];
const str = (s) => ["string", s];
const tag = (t, payload) => ["tagged", t, payload];
const arr = (...items) => ["copyArray", ...items];
const rec = (fields) => ["copyRecord", fields];

// Each YAY document, and what smallcaps decodes binyay's YSON for it to.
// binyay sorts keys as smallcaps does, so the re-encoding is exact.
const cases = [
  ["an inline tag", "10 (s)\n", tag("s", big("10"))],
  ["the empty tag", "10 ()\n", tag("", big("10"))],
  [
    "a block tag",
    "color: (rgb)\n  - 255\n  - 136\n  - 0\n",
    rec({ color: tag("rgb", arr(big("255"), big("136"), big("0"))) }),
  ],
  [
    "stacked tags, the nearest the payload innermost",
    "when: '2026-11-04T01:00:00Z' (datetime) (utc)\n",
    rec({
      when: tag("utc", tag("datetime", str("2026-11-04T01:00:00Z"))),
    }),
  ],
  ["an escaped )", "'x' (a\\)b)\n", tag("a)b", str("x"))],
  [
    "tags beginning with ! and #",
    "a: [1.5, true] (!x)\nb: null (#h)\n",
    rec({
      a: tag("!x", arr(num("1.5"), ["boolean", true])),
      b: tag("#h", ["null", null]),
    }),
  ],
  [
    "the !-escaped keys and strings",
    "\"!bang\": '#hash'\n\"#hash\": '*star'\n\"*star\": '+plus'\n",
    rec({
      "!bang": str("#hash"),
      "#hash": str("*star"),
      "*star": str("+plus"),
    }),
  ],
  [
    "bigints and negative integers",
    "big: 12345678901234567890\nneg: -7\nzero: 0\n",
    rec({
      big: big("12345678901234567890"),
      neg: big("-7"),
      zero: big("0"),
    }),
  ],
  [
    "floats, nan and the infinities",
    "a: 1.5\nb: -2.25\nc: nan\nd: infinity\ne: -infinity\n",
    rec({
      a: num("1.5"),
      b: num("-2.25"),
      c: num("NaN"),
      d: num("Infinity"),
      e: num("-Infinity"),
    }),
  ],
  [
    "nested objects and arrays",
    "outer:\n  inner:\n    - [1, 'two']\n    - {three: 3.0} (t)\n  empty: []\n",
    rec({
      outer: rec({
        empty: arr(),
        inner: arr(
          arr(big("1"), str("two")),
          tag("t", rec({ three: num("3") })),
        ),
      }),
    }),
  ],
];

describe("YSON is smallcaps: live binyay output", { skip }, () => {
  for (const [name, yay, expected] of cases) {
    it(name, () => {
      const { toCapData, fromCapData } = makeSmallcapsMarshal();
      const yson = toYson(yay);
      const decoded = fromCapData({ body: `#${yson}`, slots: [] });
      assert.deepStrictEqual(show(decoded), expected);

      // encodeToSmallcaps writes back binyay's YSON, but for whitespace
      const { body, slots } = toCapData(decoded);
      assert.deepStrictEqual(slots, []);
      assert.strictEqual(body, `#${JSON.stringify(JSON.parse(yson))}`);
    });
  }

  it("a tagged value reads in libyay as smallcaps decodes it", () => {
    const { fromCapData } = makeSmallcapsMarshal();
    const yay = "[1.5, true] (!x)\n";
    const value = parseYay(yay);
    assert.ok(isTagged(value));
    const decoded = fromCapData({ body: `#${toYson(yay)}`, slots: [] });
    assert.strictEqual(decoded[Symbol.toStringTag], value.tag);
    assert.deepStrictEqual(decoded.payload, value.payload);
  });

  it("-0 decodes as -0, which smallcaps writes back as 0", () => {
    const { toCapData, fromCapData } = makeSmallcapsMarshal();
    const yson = toYson("-0.0\n");
    assert.strictEqual(yson.trim(), "-0");
    const decoded = fromCapData({ body: `#${yson}`, slots: [] });
    assert.deepStrictEqual(show(decoded), num("-0"));
    // smallcaps' body is JSON.stringify's, which drops the sign of zero
    assert.strictEqual(toCapData(decoded).body, "#0");
  });

  it("a reference needs the CapData bridge", () => {
    const yay = "icon: <https://example.com/cat.png> (image/png)\n";
    const yson = toYson(yay);
    assert.match(yson, /"\$https:\/\/example\.com\/cat\.png"/);

    // smallcaps reads "$" as "$<slot index>.<iface>", and the URI is no index
    const bare = makeSmallcapsMarshal();
    assert.throws(
      () => bare.fromCapData({ body: `#${yson}`, slots: [] }),
      /NaN is not a safe integer/,
    );

    // ysonToCapData moves the URI into a slot, and smallcaps makes it the
    // remotable the caller's convertSlotToVal gives
    const remotable = Far("Reference", {});
    const slotted = makeSmallcapsMarshal(
      (val) => (val === remotable ? "https://example.com/cat.png" : undefined),
      (uri) => {
        assert.strictEqual(uri, "https://example.com/cat.png");
        return remotable;
      },
    );
    const capData = ysonToCapData(yson);
    assert.deepStrictEqual(capData.slots, ["https://example.com/cat.png"]);
    const decoded = slotted.fromCapData(capData);
    assert.deepStrictEqual(
      show(decoded),
      rec({ icon: tag("image/png", ["remotable", "Alleged: Reference"]) }),
    );
    assert.strictEqual(decoded.icon.payload, remotable);
    assert.deepStrictEqual(slotted.toCapData(decoded), {
      body: '#{"icon":{"#tag":"image/png","payload":"$0.Alleged: Reference"}}',
      slots: ["https://example.com/cat.png"],
    });
  });

  // Bytes: YSON writes "*<hex>", the form endojs/endo#2871 ("fix(marshal):
  // marshal ByteArrays to hex", https://github.com/endojs/endo/pull/2871)
  // gives byteArrays. @endo/marshal 1.10.0 reserves "*" and refuses it.
  // When Endo ships #2871 these assertions fail: make them decode a
  // byteArray instead.
  for (const [name, yay, yson] of [
    ["bytes", "<ff8800ff>\n", '"*ff8800ff"'],
    ["tagged bytes", "<ff8800ff> (rgba)\n", '"*ff8800ff"'],
    ["empty bytes", "<>\n", '"*"'],
  ]) {
    it(`${name}: "*<hex>", which @endo/marshal 1.10.0 refuses`, () => {
      const { fromCapData } = makeSmallcapsMarshal();
      const written = toYson(yay);
      assert.ok(written.includes(yson), written);
      assert.throws(
        () => fromCapData({ body: `#${written}`, slots: [] }),
        /Special char "\*" reserved for future use/,
      );
    });
  }
});
