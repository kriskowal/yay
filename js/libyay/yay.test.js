import { describe, it } from "node:test";
import assert from "node:assert";
import fs from "fs";
import path from "path";

import { parseYay, reference, tagged } from "./yay.js";
import { casesDir, cases } from "./test-helpers.js";

const testRoot = casesDir("test");
const yayDir = path.join(testRoot, "yay");
const nayDir = path.join(testRoot, "nay");
const mehDir = path.join(testRoot, "meh");
const jsDir = path.join(testRoot, "js");

/**
 * Decode UTF-8 from raw bytes without replacing surrogate code points.
 * Used for negative .nay fixtures so BOM (U+FEFF) and lone surrogates (e.g. U+D800)
 * are passed through to parseYay instead of being replaced by Node's default decoder.
 */
function decodeUtf8(buffer) {
  const bytes = new Uint8Array(buffer);
  let s = "";
  let i = 0;
  while (i < bytes.length) {
    const b = bytes[i];
    if (b < 0x80) {
      s += String.fromCharCode(b);
      i++;
      continue;
    }
    if (b >= 0xc0 && b < 0xe0) {
      if (i + 1 >= bytes.length) {
        s += String.fromCharCode(b);
        i++;
        continue;
      }
      const b1 = bytes[i + 1];
      if ((b1 & 0xc0) !== 0x80) {
        s += String.fromCharCode(b);
        i++;
        continue;
      }
      const c = ((b & 0x1f) << 6) | (b1 & 0x3f);
      s +=
        c >= 0xd800 && c <= 0xdfff
          ? String.fromCharCode(c)
          : String.fromCodePoint(c);
      i += 2;
      continue;
    }
    if (b >= 0xe0 && b < 0xf0) {
      if (i + 2 >= bytes.length) {
        s += String.fromCharCode(b);
        i++;
        continue;
      }
      const b1 = bytes[i + 1];
      const b2 = bytes[i + 2];
      if ((b1 & 0xc0) !== 0x80 || (b2 & 0xc0) !== 0x80) {
        s += String.fromCharCode(b);
        i++;
        continue;
      }
      const c = ((b & 0x0f) << 12) | ((b1 & 0x3f) << 6) | (b2 & 0x3f);
      s +=
        c >= 0xd800 && c <= 0xdfff
          ? String.fromCharCode(c)
          : String.fromCodePoint(c);
      i += 3;
      continue;
    }
    if (b >= 0xf0 && b < 0xf8) {
      if (i + 3 >= bytes.length) {
        s += String.fromCharCode(b);
        i++;
        continue;
      }
      const b1 = bytes[i + 1];
      const b2 = bytes[i + 2];
      const b3 = bytes[i + 3];
      if (
        (b1 & 0xc0) !== 0x80 ||
        (b2 & 0xc0) !== 0x80 ||
        (b3 & 0xc0) !== 0x80
      ) {
        s += String.fromCharCode(b);
        i++;
        continue;
      }
      const c =
        ((b & 0x07) << 18) |
        ((b1 & 0x3f) << 12) |
        ((b2 & 0x3f) << 6) |
        (b3 & 0x3f);
      s += String.fromCodePoint(c);
      i += 4;
      continue;
    }
    s += String.fromCharCode(b);
    i++;
  }
  return s;
}

function loadFixture(basename) {
  const yayPath = path.join(yayDir, `${basename}.yay`);
  const jsPath = path.join(jsDir, `${basename}.js`);
  const yaySource = fs.readFileSync(yayPath, "utf8");
  const jsSource = fs.readFileSync(jsPath, "utf8");
  const expected = eval(jsSource);
  return { yaySource, expected };
}

describe("parseYay", () => {
  const yayFiles = cases(yayDir, "yay");
  const basenames = yayFiles.map((f) => path.basename(f, ".yay"));

  for (const basename of basenames) {
    it(`fixture ${basename} matches expected value`, () => {
      const { yaySource, expected } = loadFixture(basename);
      const parsed = parseYay(yaySource, { decodeReference: reference });
      assert.deepStrictEqual(parsed, expected, `fixture ${basename}`);
    });
  }
});

it("decodes hex without Uint8Array.fromHex", () => {
  const fromHex = Uint8Array.fromHex;
  Uint8Array.fromHex = () => {
    throw new Error("the parser used Uint8Array.fromHex");
  };
  try {
    assert.deepStrictEqual(parseYay("<00ff>\n"), new Uint8Array([0, 255]));
    assert.deepStrictEqual(parseYay("> 00 ff\n"), new Uint8Array([0, 255]));
  } finally {
    if (fromHex === undefined) delete Uint8Array.fromHex;
    else Uint8Array.fromHex = fromHex;
  }
});

describe("bytes: plain by default, with a hook or immutable mode", () => {
  it("is a plain Uint8Array by default", () => {
    const value = parseYay("a: <00ff>\nb: <>\n");
    assert.ok(value.a instanceof Uint8Array);
    assert.deepStrictEqual(value.a, new Uint8Array([0, 255]));
    assert.deepStrictEqual(value.b, new Uint8Array(0));
  });
  it("takes a caller's hook for inline, block and empty bytes", () => {
    const hex = (bytes) =>
      Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
    const value = parseYay("a: <00ff>\nb: <>\nc: >\n  01 02\n", {
      bytes: hex,
    });
    assert.deepStrictEqual(value, { a: "00ff", b: "", c: "0102" });
  });
  it("fails loudly when immutable bytes are unavailable", () => {
    assert.throws(
      () => parseYay("<00ff>\n", { bytes: "immutable" }),
      /Immutable bytes are unavailable/,
    );
  });
  it("refuses an unknown bytes option", () => {
    assert.throws(
      () => parseYay("<00ff>\n", { bytes: "freeze" }),
      /bytes must be "plain", "immutable", or a function/,
    );
  });
});

describe("parseYay negative (invalid .nay)", () => {
  const nayFiles = cases(nayDir, "nay");
  const basenames = nayFiles.map((f) => path.basename(f, ".nay"));

  for (const basename of basenames) {
    it(`invalid ${basename} throws expected error`, () => {
      const nayPath = path.join(nayDir, `${basename}.nay`);
      const errorPath = path.join(nayDir, `${basename}.error`);
      const rawBytes = fs.readFileSync(nayPath);
      let naySource = decodeUtf8(rawBytes);
      if (basename === "unicode-invalid-bom") {
        naySource = "\uFEFF" + naySource;
      }
      const expectedMessage = fs.readFileSync(errorPath, "utf8").trim();
      const filename = `${basename}.nay`;
      assert.throws(
        () => parseYay(naySource, { filename, decodeReference: reference }),
        (err) => {
          // The whole message, its line and column included
          assert.equal(
            err.message.trim(),
            expectedMessage,
            `error message for ${basename}`,
          );
          return true;
        },
        `parseYay should throw for ${basename}.nay`,
      );
    });
  }
});

// test/meh holds MEH's cases: <name>.<variant>.meh reads as MEH to
// <name>.yay, or is refused with the message in <name>.error. JavaScript has
// no MEH reader, but YAY is MEH without the leniency, which changes spacing
// and never meaning. So each <name>.yay is YAY; YAY reads each .meh to the
// value of its .yay or refuses it; and YAY refuses what MEH refuses.
describe("parseYay over the MEH corpus (test/meh)", () => {
  const files = fs.readdirSync(mehDir).sort();
  const read = (name) => fs.readFileSync(path.join(mehDir, name), "utf8");

  for (const name of files.filter((f) => f.endsWith(".yay"))) {
    it(`${name} is YAY`, () => {
      parseYay(read(name), { filename: name, decodeReference: reference });
    });
  }

  const mehs = files.filter((f) => f.endsWith(".meh"));
  it("finds the cases", () => {
    assert.ok(mehs.length > 0, `MEH cases in ${mehDir}`);
  });
  for (const name of mehs) {
    const stem = name.slice(0, -".meh".length);
    const test = stem.includes(".")
      ? stem.slice(0, stem.lastIndexOf("."))
      : stem;
    it(`${name} is refused or reads as ${test}.yay`, () => {
      if (files.includes(`${test}.yay`)) {
        let value;
        try {
          value = parseYay(read(name), {
            filename: name,
            decodeReference: reference,
          });
        } catch {
          return;
        }
        assert.deepStrictEqual(
          value,
          parseYay(read(`${test}.yay`), { decodeReference: reference }),
        );
      } else {
        assert.ok(files.includes(`${test}.error`), `${test}.yay or .error`);
        assert.throws(() =>
          parseYay(read(name), {
            filename: name,
            decodeReference: reference,
          }),
        );
      }
    });
  }
});

describe("inline nesting limit", () => {
  it("reads 256 levels of inline arrays and objects", () => {
    const arrays = "[".repeat(256) + "1" + "]".repeat(256);
    assert.doesNotThrow(() => parseYay(`a: ${arrays}\n`));
    const objects = "{a: ".repeat(256) + "1" + "}".repeat(256);
    assert.doesNotThrow(() => parseYay(`a: ${objects}\n`));
  });
  it("refuses deeper nesting with a YAY error, not a RangeError", () => {
    for (const [open, close] of [
      ["[", "]"],
      ["{a: ", "}"],
    ]) {
      const deep = open.repeat(100000) + "1" + close.repeat(100000);
      assert.throws(
        () => parseYay(`a: ${deep}\n`, { filename: "deep.yay" }),
        (err) =>
          !(err instanceof RangeError) &&
          /^Inline nesting deeper than 256 levels at 1:\d+ of <deep\.yay>$/.test(
            err.message,
          ),
      );
    }
  });
});
