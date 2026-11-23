// Benchmark the parser against generated inputs. No dependencies.
//
// Usage: node bench.mjs [--impl path/to/yay.js] [shape ...]
//
// Each shape is generated at a few sizes, warmed up, and timed by the median
// of several runs. MB/s counts input bytes per second of parsing.

import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
let impl = new URL("./yay.js", import.meta.url).href;
const args = process.argv.slice(2);
if (args[0] === "--impl") {
  if (args.length < 2) {
    console.error("bench.mjs: --impl needs a path");
    process.exit(2);
  }
  impl = new URL(args[1], `file://${process.cwd()}/`).href;
  args.splice(0, 2);
}
const { parseYay } = await import(impl);

const indent = (text, pad) =>
  text
    .split("\n")
    .map((line) => (line.length ? pad + line : line))
    .join("\n");

let corpusUnits;
function corpusPieces() {
  if (corpusUnits) return corpusUnits;
  const dir = join(here, "../../test/yay");
  corpusUnits = [];
  for (const file of readdirSync(dir).sort()) {
    if (!file.endsWith(".yay")) continue;
    let text = readFileSync(join(dir, file), "utf8").replace(/\n+$/, "");
    text = text
      .split("\n")
      .filter((line) => !line.startsWith("#"))
      .join("\n");
    if (!text.trim()) continue;
    const candidates = [`K:\n${indent(text, "  ")}`];
    if (!text.includes("\n")) candidates.unshift(`K: ${text}`);
    for (const candidate of candidates) {
      try {
        parseYay(candidate.replace("K", "k0"));
        parseYay(`a: 1\n${candidate.replace("K", "k0")}\nz: 2`);
        corpusUnits.push(candidate);
        break;
      } catch {}
    }
  }
  return corpusUnits;
}

const shapes = {
  corpus: (n) => {
    const units = corpusPieces();
    const out = [];
    for (let i = 0; i < n; i++)
      out.push(units[i % units.length].replace("K", "k" + i));
    return out.join("\n") + "\n";
  },
  listInts: (n) => {
    const lines = [];
    for (let i = 0; i < n; i++) lines.push(`- ${i * 7919}`);
    return lines.join("\n") + "\n";
  },
  listFloats: (n) => {
    const lines = [];
    for (let i = 0; i < n; i++) lines.push(`- ${(i * 0.37).toFixed(3)}`);
    return lines.join("\n") + "\n";
  },
  listStrings: (n) => {
    const lines = [];
    for (let i = 0; i < n; i++)
      lines.push(`- "string item ${i} with some text"`);
    return lines.join("\n") + "\n";
  },
  wideObject: (n) => {
    const lines = [];
    for (let i = 0; i < n; i++) lines.push(`key${i}: "value number ${i}"`);
    return lines.join("\n") + "\n";
  },
  inlineIntsLine: (n) => {
    const values = [];
    for (let i = 0; i < n; i++) values.push(String(i * 7919));
    return "[" + values.join(", ") + "]\n";
  },
  manyInline: (n) => {
    const lines = [];
    for (let i = 0; i < n; i++)
      lines.push(
        `- {id: ${i}, name: "n${i}", tags: ["a", "b"], ok: true, ratio: 0.5}`,
      );
    return lines.join("\n") + "\n";
  },
  manyRefs: (n) => {
    const lines = [];
    for (let i = 0; i < n; i++)
      lines.push(`- <https://example.com/a/b%20c/${i}?q=x&r=y#frag>`);
    return lines.join("\n") + "\n";
  },
  manyTaggedInline: (n) => {
    const lines = [];
    for (let i = 0; i < n; i++) lines.push(`- ${i} (unit) (meters)`);
    return lines.join("\n") + "\n";
  },
  manyTaggedBlock: (n) => {
    const lines = [];
    for (let i = 0; i < n; i++) lines.push(`- (doc) \``, `    block ${i}`);
    return lines.join("\n") + "\n";
  },
  bytesInline: (n) =>
    "<" + "ff00a5".repeat(Math.ceil(n / 3)).slice(0, n * 2) + ">\n",
  blockString: (n) => {
    const lines = ["text: `"];
    for (let i = 0; i < n; i++)
      lines.push(
        `  Line ${i} of a long block string, with words and \`ticks\` # not a comment.`,
      );
    return lines.join("\n") + "\n";
  },
  longQuotedPlain: (n) => '"' + "x".repeat(n) + '"\n',
  longQuoted: (n) =>
    '"' + "abc def \\n \\u{1F600} ".repeat(Math.ceil(n / 22)) + '"\n',
  longQuotedParens: (n) => '"' + "(a) <b> ".repeat(Math.ceil(n / 8)) + '"\n',
  longSingle: (n) => "'" + "x".repeat(n) + "'\n",
  deepInline: (n) => "[".repeat(n) + "1" + "]".repeat(n) + "\n",
  deepInlineObj: (n) => "{a: ".repeat(n) + "1" + "}".repeat(n) + "\n",
  deepBlock: (n) => {
    const lines = [];
    for (let i = 0; i < n; i++) lines.push(" ".repeat(2 * i) + "a:");
    lines.push(" ".repeat(2 * n) + "b: 1");
    return lines.join("\n") + "\n";
  },
};

const scales = {
  corpus: [1000, 4000, 16000],
  listInts: [10e3, 40e3, 160e3],
  listFloats: [10e3, 40e3, 160e3],
  listStrings: [10e3, 40e3, 160e3],
  wideObject: [10e3, 40e3, 160e3],
  inlineIntsLine: [10e3, 40e3, 160e3],
  manyInline: [5e3, 20e3, 80e3],
  manyRefs: [5e3, 20e3, 80e3],
  manyTaggedInline: [5e3, 20e3, 80e3],
  manyTaggedBlock: [1e3, 2e3, 4e3],
  bytesInline: [64e3, 256e3, 1024e3],
  blockString: [5e3, 20e3, 80e3],
  longQuotedPlain: [256e3, 1024e3, 4096e3],
  longQuoted: [64e3, 256e3, 1024e3],
  longQuotedParens: [64e3, 256e3, 1024e3],
  longSingle: [256e3, 1024e3, 4096e3],
  deepInline: [32, 128, 256],
  deepInlineObj: [32, 128, 256],
  deepBlock: [250, 500, 1000],
};

function time(parse, text) {
  parse(text);
  parse(text);
  const runs = [];
  const start = performance.now();
  while (
    runs.length < 5 ||
    (runs.length < 15 && performance.now() - start < 200)
  ) {
    const one = performance.now();
    parse(text);
    runs.push(performance.now() - one);
  }
  runs.sort((a, b) => a - b);
  return runs[runs.length >> 1];
}

const names = args.length ? args : Object.keys(shapes);
console.log(`# node ${process.version} impl=${impl}`);
console.log("shape\tscale\tbytes\tms\tMB/s");
for (const name of names) {
  const generate = shapes[name];
  if (!generate) {
    console.error(`bench.mjs: no shape named ${name}`);
    process.exit(2);
  }
  for (const scale of scales[name]) {
    const text = generate(scale);
    const bytes = Buffer.byteLength(text);
    const ms = time(parseYay, text);
    console.log(
      [
        name,
        scale,
        bytes,
        ms.toFixed(2),
        (bytes / 1e6 / (ms / 1e3)).toFixed(1),
      ].join("\t"),
    );
  }
}
