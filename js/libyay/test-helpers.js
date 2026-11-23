import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("../..", import.meta.url));

/** The absolute path to a case directory in this repository. */
function casesDir(...parts) {
  return path.join(repoRoot, ...parts);
}

/** Sorted case names with an extension and, optionally, a prefix. */
function cases(directory, extension, prefix = "") {
  return fs
    .readdirSync(directory)
    .filter((name) => name.startsWith(prefix) && name.endsWith(`.${extension}`))
    .sort();
}

/**
 * Recursively project a parsed value into the tagged-value fixtures' JSON
 * vocabulary.
 */
function toFixture(value, isTagged) {
  if (isTagged(value)) {
    return {
      "#tag": value[Symbol.toStringTag],
      payload: toFixture(value.payload, isTagged),
    };
  }
  if (value instanceof Uint8Array) {
    return (
      "*" +
      Array.from(value, (byte) => byte.toString(16).padStart(2, "0")).join("")
    );
  }
  if (Array.isArray(value)) {
    return value.map((item) => toFixture(item, isTagged));
  }
  if (typeof value === "bigint") return Number(value);
  if (typeof value === "number" && Number.isNaN(value)) return "NaN";
  if (value === Infinity) return "Infinity";
  if (value === -Infinity) return "-Infinity";
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [
        key,
        toFixture(item, isTagged),
      ]),
    );
  }
  return value;
}

export { casesDir, cases, toFixture };
