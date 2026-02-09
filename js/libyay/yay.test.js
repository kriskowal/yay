import { describe, it } from "node:test";
import assert from "node:assert";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "node:url";

import { parseYay } from "./yay.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const testRoot = path.join(__dirname, "..", "..", "test");
const yayDir = path.join(testRoot, "yay");
const nayDir = path.join(testRoot, "nay");
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

// Uint8Array.fromHex may not exist in Node < 22
if (typeof Uint8Array.fromHex !== "function") {
  Uint8Array.fromHex = function fromHex(hex) {
    const str = hex.replace(/\s/g, "");
    if (str.length % 2 !== 0) throw new RangeError("Odd number of hex digits");
    if (!/^[0-9a-fA-F]*$/.test(str)) throw new SyntaxError("Invalid hex digit");
    const bytes = new Uint8Array(str.length / 2);
    for (let i = 0; i < bytes.length; i++) {
      bytes[i] = parseInt(str.slice(i * 2, i * 2 + 2), 16);
    }
    return bytes;
  };
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
  const yayFiles = fs.readdirSync(yayDir).filter((f) => f.endsWith(".yay"));
  const basenames = yayFiles.map((f) => path.basename(f, ".yay"));

  for (const basename of basenames) {
    it(`fixture ${basename} matches expected value`, () => {
      const { yaySource, expected } = loadFixture(basename);
      const parsed = parseYay(yaySource);
      assert.deepStrictEqual(parsed, expected, `fixture ${basename}`);
    });
  }
});

describe("parseYay JSON parity (.yayson)", () => {
  const yaysonFiles = fs
    .readdirSync(yayDir)
    .filter((f) => f.endsWith(".yayson"));
  const basenames = yaysonFiles.map((f) => path.basename(f, ".yayson"));

  for (const basename of basenames) {
    it(`fixture ${basename} matches JSON.parse`, () => {
      const yaysonPath = path.join(yayDir, `${basename}.yayson`);
      const yaysonSource = fs.readFileSync(yaysonPath, "utf8");
      const trimmed = yaysonSource.replace(/\r?\n$/, "");
      assert.ok(
        !/[\r\n]/.test(trimmed),
        `fixture ${basename} must be single-line JSON`,
      );
      const parsed = parseYay(yaysonSource);
      const expected = JSON.parse(yaysonSource);
      assert.deepStrictEqual(parsed, expected, `fixture ${basename}`);
    });
  }
});

describe("parseYay negative (invalid .nay)", () => {
  const nayFiles = fs.readdirSync(nayDir).filter((f) => f.endsWith(".nay"));
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
        () => parseYay(naySource, filename),
        (err) => {
          // Extract the error type (before "at X:Y of")
          const match = expectedMessage.match(/^(.+?)\s+at\s+\d+:\d+/);
          const expectedPattern = match ? match[1].trim() : expectedMessage;
          // Check if actual error contains expected pattern (case-insensitive)
          const actualMsg = err.message.trim();
          const containsPattern = actualMsg
            .toLowerCase()
            .includes(expectedPattern.toLowerCase());
          if (!containsPattern) {
            assert.fail(
              `error message for ${basename}: expected to contain "${expectedPattern}", got "${actualMsg}"`,
            );
          }
          return true;
        },
        `parseYay should throw for ${basename}.nay`,
      );
    });
  }
});
