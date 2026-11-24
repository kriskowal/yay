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

export { casesDir, cases };
