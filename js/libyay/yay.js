"use strict";

const KEY_CHARACTER = /^[A-Za-z0-9_-]$/;
const LOWER_HEX = /^[0-9a-f]*$/;
const UNICODE_ESCAPE = /^[0-9A-Fa-f]{1,6}$/;
const HEX_PAIR = /^[0-9A-Fa-f]{2}/;
const SCHEME_START = /^[A-Za-z]$/;
const SCHEME_CHARACTER = /^[A-Za-z0-9+.-]$/;
const UPPERCASE = /[A-Z]/;
const URI_ASCII = /^[A-Za-z0-9._~:/?#[\]@!$&'()*+,;=-]$/;
const UNICODE_SPACE =
  /[\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]/;

const SIMPLE_ESCAPES = Object.freeze({
  "\\": "\\",
  "/": "/",
  b: "\b",
  f: "\f",
  n: "\n",
  r: "\r",
  t: "\t",
});

/**
 * Parses a YAY document string and returns the corresponding JavaScript value.
 * @param {string} source - UTF-8 YAY document
 * @param {string} [filename] - Optional filename for error messages
 * @returns {unknown} - Parsed value (null, bigint, number, boolean, string, Array, object, Uint8Array)
 */
function parseYay(source, filename) {
  const ctx = { filename: filename || undefined };
  const lines = scan(source, ctx);
  const tokens = outlineLex(lines);
  return parseRoot(tokens, ctx);
}

function locSuffix(ctx, line, col) {
  if (!ctx.filename) return "";
  const oneBasedLine = line + 1;
  const oneBasedCol = col + 1;
  return (
    " at " + oneBasedLine + ":" + oneBasedCol + " of <" + ctx.filename + ">"
  );
}

/**
 * Throw a parser error at a source location.
 * @param {ParseContext} ctx
 * @param {string} message
 * @param {number} line
 * @param {number} col
 * @param {boolean=} appendLocation
 * @returns {never}
 */
function fail(ctx, message, line, col, appendLocation = true) {
  const error = new Error(
    message + (appendLocation ? locSuffix(ctx, line, col) : ""),
  );
  Object.defineProperties(error, {
    line: { value: line + 1 },
    column: { value: col + 1 },
    filename: { value: ctx.filename },
  });
  throw error;
}

/**
 * Check whether a code point is allowed in a YAY document.
 * @param {number} cp
 * @returns {boolean}
 */
function isAllowedCodePoint(cp) {
  return (
    cp === 0x000a ||
    (0x0020 <= cp && cp <= 0x007e) ||
    (0x00a0 <= cp && cp <= 0xd7ff) ||
    (0xe000 <= cp && cp <= 0xfffd && !(0xfdd0 <= cp && cp <= 0xfdef)) ||
    (0x10000 <= cp && cp <= 0x10ffff && (cp & 0xffff) < 0xfffe)
  );
}

/**
 * Strip an inline comment from a value. A comment after a value takes two
 * spaces before its "#"; one space is refused.
 * @param {string} s
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @param {ReturnType<typeof scanTopLevel>=} syntax
 * @returns {string}
 */
function stripInlineComment(
  s,
  ctx = {},
  lineNum = 0,
  col = 0,
  syntax = scanTopLevel(s),
) {
  if (syntax.comment === undefined) return s;
  checkCommentSpacing(s, syntax.comment, ctx, lineNum, col);
  return s.slice(0, syntax.comment).trimEnd();
}

/**
 * Refuse a comment, at s[hash], that has one space before it rather than
 * two.
 * @param {string} s
 * @param {number} hash
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 */
function checkCommentSpacing(s, hash, ctx, lineNum, col) {
  if (hash >= 1 && s[hash - 1] === " " && (hash < 2 || s[hash - 2] !== " ")) {
    throw new Error(
      "Expected two spaces before a comment" +
        locSuffix(ctx, lineNum, col + hash),
    );
  }
}

/**
 * @typedef {Object} ParseContext
 * @property {string=} filename
 */

/**
 * @typedef {Object} ScanLine
 * @property {string} line
 * @property {number} indent
 * @property {string} leader
 * @property {number} lineNum
 */
/**
 * @typedef {Object} Token
 * @property {'start'|'stop'|'text'|'break'} type
 * @property {string} text
 * @property {number=} indent
 * @property {number=} lineNum
 * @property {number=} col
 */

/**
 * Skip blank-line tokens and, where a caller is between blocks, stop tokens.
 * @param {Token[]} tokens
 * @param {number} i
 * @param {boolean=} includeStops
 * @returns {number}
 */
function skipBlank(tokens, i, includeStops = false) {
  while (
    i < tokens.length &&
    (tokens[i].type === "break" || (includeStops && tokens[i].type === "stop"))
  ) {
    i++;
  }
  return i;
}

// --- Scanner: source -> lines with { line, indent, leader, lineNum } ---

/**
 * @param {string} source
 * @param {ParseContext} ctx
 * @returns {ScanLine[]}
 */
function scan(source, ctx = {}) {
  if (source.length >= 1 && source.charCodeAt(0) === 0xfeff) {
    throw new Error("Illegal BOM" + locSuffix(ctx, 0, 0));
  }
  // Validate all code points.
  {
    let line = 0;
    let col = 0;
    for (let i = 0; i < source.length; i++) {
      const c = source.charCodeAt(i);
      let cp = c;
      // Decode surrogate pairs to get the actual code point.
      if (c >= 0xd800 && c <= 0xdbff) {
        if (
          i + 1 >= source.length ||
          source.charCodeAt(i + 1) < 0xdc00 ||
          source.charCodeAt(i + 1) > 0xdfff
        ) {
          throw new Error("Illegal surrogate" + locSuffix(ctx, line, col));
        }
        cp =
          (c - 0xd800) * 0x400 + (source.charCodeAt(i + 1) - 0xdc00) + 0x10000;
        if (!isAllowedCodePoint(cp)) {
          throw new Error(
            "Forbidden code point U+" +
              cp.toString(16).toUpperCase().padStart(4, "0") +
              locSuffix(ctx, line, col),
          );
        }
        col++;
        i++;
        continue;
      }
      if (c >= 0xdc00 && c <= 0xdfff) {
        throw new Error("Illegal surrogate" + locSuffix(ctx, line, col));
      }
      if (!isAllowedCodePoint(cp)) {
        if (cp === 0x09) {
          throw new Error(
            "Tab not allowed (use spaces)" + locSuffix(ctx, line, col),
          );
        }
        throw new Error(
          "Forbidden code point U+" +
            cp.toString(16).toUpperCase().padStart(4, "0") +
            locSuffix(ctx, line, col),
        );
      }
      if (cp === 0x0a) {
        line++;
        col = 0;
      } else {
        col++;
      }
    }
  }
  const lines = [];
  const lineStrings = source.split(/\n/);
  // While inside a block string, the column that ends it: the indent of a
  // bare opener, or the key's column for a property opener. Its content
  // lines are literal: no list leaders, no "-" or "*" checks, no comments.
  let blockOpener = -1;
  for (let i = 0; i < lineStrings.length; i++) {
    const lineStr = lineStrings[i];
    if (lineStr.length > 0 && lineStr.charCodeAt(lineStr.length - 1) === 0x20) {
      throw new Error(
        "Unexpected trailing space" + locSuffix(ctx, i, lineStr.length - 1),
      );
    }
    let indent = 0;
    while (indent < lineStr.length && lineStr[indent] === " ") {
      indent++;
    }
    const rest = lineStr.slice(indent);
    if (blockOpener >= 0) {
      if (lineStr.length === 0 || indent > blockOpener) {
        lines.push({ line: rest, indent, leader: "", lineNum: i });
        continue;
      }
      blockOpener = -1;
    }
    if (rest.startsWith("#") && indent === 0) {
      continue; // comment only at column 0
    }
    // leader identifies list/bytes syntax while preserving the line payload.
    let leader = "";
    let line = rest;
    if (rest.startsWith("- ")) {
      leader = "-";
      line = rest.slice(2);
    } else if (rest === "-") {
      // Bare "-" without space is invalid - must be "- " followed by value
      throw new Error(
        'Expected space after "-"' + locSuffix(ctx, i, indent + 1),
      );
    } else if (rest.match(/^-\.?\d/)) {
      leader = "";
      line = rest;
    } else if (rest === "-infinity") {
      leader = "";
      line = rest;
    } else if (
      rest.length >= 2 &&
      rest[0] === "-" &&
      rest[1] !== " " &&
      rest[1] !== "." &&
      !/^\d/.test(rest[1])
    ) {
      // Compact list syntax (-value without space) is not allowed
      throw new Error(
        'Expected space after "-"' + locSuffix(ctx, i, indent + 1),
      );
    } else if (
      rest === "*" ||
      (rest.length >= 2 && rest[0] === "*" && rest[1] === " ")
    ) {
      throw new Error('Unexpected character "*"' + locSuffix(ctx, i, indent));
    }
    if (isBareBlockOpener(line) || isBareBlockOpener(afterPrefix(line) ?? "")) {
      blockOpener = indent;
    } else if (line.endsWith(": `") || isTaggedPropertyOpener(line)) {
      blockOpener = indent + (rest.length - line.length);
    }
    lines.push({ line, indent, leader, lineNum: i });
  }
  return lines;
}

/**
 * Whether a line's content (after any list leader) is a bare "`" or
 * "` text" block string opener.
 * @param {string} line
 */
function isBareBlockOpener(line) {
  return line === "`" || line.startsWith("` ");
}

// --- Outline Lexer: lines -> tokens { type, text, indent?, lineNum?, col? } ---

/**
 * @param {ScanLine[]} lines
 * @returns {Token[]}
 */
function outlineLex(lines) {
  const tokens = [];
  let stack = [0];
  let top = 0;
  for (const { line, indent, leader, lineNum } of lines) {
    // Close blocks on dedent. A blank line has no indent of its own, so it
    // closes nothing.
    while (line.length > 0 && indent < top) {
      tokens.push({ type: "stop", text: "" });
      stack.pop();
      top = stack[stack.length - 1];
    }
    if (leader.length > 0 && indent > top) {
      tokens.push({
        type: "start",
        text: leader,
        indent,
        lineNum,
        col: indent,
      });
      stack.push(indent);
      top = indent;
    } else if (leader.length > 0 && indent === top) {
      tokens.push({ type: "stop", text: "" });
      tokens.push({
        type: "start",
        text: leader,
        indent,
        lineNum,
        col: indent,
      });
    }
    // Every blank line is a break, so that a block string keeps a run of
    // blank lines.
    if (line.length > 0) {
      // The text's column follows the list leader, "- ", if any.
      tokens.push({
        type: "text",
        text: line,
        indent,
        lineNum,
        col: leader ? indent + 2 : indent,
      });
    } else {
      tokens.push({ type: "break", text: "", lineNum, col: indent });
    }
  }
  while (stack.length > 1) {
    tokens.push({ type: "stop", text: "" });
    stack.pop();
  }
  return tokens;
}

// --- Value parser: tokens -> value ---

/**
 * Give an object a property of its own. Assignment would take a
 * "__proto__" key for the prototype; YAY has no such key.
 * @param {Record<string, unknown>} obj
 * @param {string} key
 * @param {unknown} value
 */
function setOwn(obj, key, value) {
  Object.defineProperty(obj, key, {
    value,
    writable: true,
    enumerable: true,
    configurable: true,
  });
}

/**
 * Record where a key first appears in an object, and refuse it the second
 * time. Each key keeps its first position, not only its name, so that the
 * error can say where the first one is.
 * @param {Map<string, [number, number]>} seen
 * @param {string} key
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 */
function claimKey(seen, key, ctx, lineNum, col) {
  const first = seen.get(key);
  if (first !== undefined) {
    throw new Error(
      `Duplicate key "${key}" (first at ${first[0] + 1}:${first[1] + 1})` +
        locSuffix(ctx, lineNum, col),
    );
  }
  seen.set(key, [lineNum, col]);
}

/**
 * @param {Token[]} tokens
 * @param {ParseContext} ctx
 * @returns {unknown}
 */
function parseRoot(tokens, ctx = {}) {
  let i = skipBlank(tokens, 0, true);
  if (i >= tokens.length) {
    throw new Error(
      "No value found in document" +
        (ctx.filename ? " <" + ctx.filename + ">" : ""),
    );
  }
  const t = tokens[i];
  if (t.type === "text" && (t.indent ?? 0) > 0) {
    const line = (t.lineNum ?? 0) + 1;
    throw new Error(
      "Unexpected indent" +
        (ctx.filename ? " at " + line + ":1 of <" + ctx.filename + ">" : ""),
    );
  }
  // A tagged root block: the tags alone on the first line, the block below
  // at column 0
  if (t.type === "text" && t.text.startsWith("(")) {
    const [tags, rest] = parsePrefix(t.text, ctx, t.lineNum ?? 0, t.col ?? 0);
    if (rest !== "") throw prefixWithoutBlock(ctx, t.lineNum ?? 0, t.col ?? 0);
    const j = skipBlank(tokens, i + 1, true);
    if (j >= tokens.length) {
      throw prefixWithoutBlock(ctx, t.lineNum ?? 0, t.col ?? 0);
    }
    return wrapPrefix(parseRoot(tokens.slice(j), ctx), tags);
  }
  if (
    t.type === "text" &&
    findKeyColonOutsideQuotes(t.text) >= 0 &&
    (t.indent ?? 0) === 0 &&
    !t.text.startsWith("{") &&
    !t.text.startsWith("[")
  ) {
    const [value, next] = parseRootObject(tokens, i, ctx);
    return ensureAtEnd(value, tokens, next, ctx);
  }
  const [value, next] = parseValue(tokens, i, ctx);
  return ensureAtEnd(value, tokens, next, ctx);
}

function ensureAtEnd(value, tokens, i, ctx = {}) {
  const j = skipBlank(tokens, i, true);
  if (j < tokens.length) {
    const t = tokens[j];
    const line = (t.lineNum ?? 0) + 1;
    const col = (t.col ?? 0) + 1;
    throw new Error(
      "Unexpected extra content" +
        (ctx.filename
          ? " at " + line + ":" + col + " of <" + ctx.filename + ">"
          : ""),
    );
  }
  return value;
}

/**
 * @param {Token[]} tokens
 * @param {number} i
 * @param {ParseContext} ctx
 * @returns {[unknown, number]}
 */
function parseValue(tokens, i, ctx = {}) {
  const t = tokens[i];
  if (t.type === "text") {
    if (t.text.startsWith(" ")) {
      const line = (t.lineNum ?? 0) + 1;
      const col = (t.col ?? 0) + 1;
      throw new Error(
        "Unexpected leading space" +
          (ctx.filename
            ? " at " + line + ":" + col + " of <" + ctx.filename + ">"
            : ""),
      );
    }
    if (t.text === "$") {
      const line = (t.lineNum ?? 0) + 1;
      const col = (t.col ?? 0) + 1;
      throw new Error(
        'Unexpected character "$"' +
          (ctx.filename
            ? " at " + line + ":" + col + " of <" + ctx.filename + ">"
            : ""),
      );
    }
  }
  if (t.type === "start") {
    if (t.text === "-") {
      return parseListArray(tokens, i, ctx);
    }
  }
  if (t.type === "text") {
    const s = t.text;
    const lineNum = t.lineNum ?? 0;
    const sCol = t.col ?? 0;
    const syntax = scanTopLevel(s);
    // The block forms, which read the lines below
    if (s === "`" || s.startsWith("` ")) {
      const firstLine = s.length > 2 ? s.slice(2) : "";
      // Use token's indent as base - block string content must be indented more
      return collectBlockString(
        tokens,
        i,
        firstLine,
        false,
        t.indent ?? 0,
        ctx,
      );
    }
    if (s.startsWith(">")) {
      const firstCol = s.startsWith("> ") ? sCol + 2 : sCol + 1;
      const firstLine = s.slice(firstCol - sCol);
      if (firstLine.length === 0) {
        fail(
          ctx,
          "Expected hex or comment in hex block",
          lineNum,
          firstCol,
          false,
        );
      }
      return parseBlockBytes(
        tokens,
        i,
        ctx,
        firstLine,
        t.indent ?? 0,
        firstCol,
      );
    }
    if (s.startsWith("[")) {
      const close = syntax.closeBracket;
      if (close >= 0 && s[close + 1] === ":") {
        throw new Error(
          'Unexpected character ":"' +
            locSuffix(ctx, lineNum, sCol + close + 1),
        );
      }
    }
    // A property on this line: "key: value" or "key:" over a block. An
    // inline value with tags after it, an inline container, a quoted string
    // and a reference or bytes never take a key here.
    const bare = stripInlineComment(s, ctx, lineNum, sCol, syntax);
    const first =
      syntax.firstTag !== undefined && syntax.firstTag < bare.length
        ? syntax.firstTag
        : undefined;
    const tagged =
      first !== undefined &&
      first > 0 &&
      findKeyColonOutsideQuotes(bare, syntax, first) < 0;
    if (
      !tagged &&
      !/^["'[{<]/.test(bare) &&
      findKeyColonOutsideQuotes(bare, syntax, bare.length) >= 0
    ) {
      const keyValue = splitKeyValue(
        bare,
        sCol,
        ctx,
        lineNum,
        undefined,
        syntax,
      );
      const { key, valuePart, valueCol } = keyValue;
      if (valuePart === "" && key.length > 0) {
        return parseObjectOrNamedArray(tokens, i, key, ctx);
      }
      if (key.length > 0) {
        const value = parseScalar(valuePart, ctx, lineNum, valueCol);
        return [{ [key]: value }, i + 1];
      }
    }
    // Every inline value reads as a property's does
    return [parseScalar(s, ctx, lineNum, sCol, syntax), i + 1];
  }
  return [undefined, i + 1];
}

/**
 * @param {string} s
 * @param {number} sCol
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number|undefined} inlineCol - column of opening brace for inline objects
 * @param {ReturnType<typeof scanTopLevel>=} syntax
 * @returns {{key: string, valuePart: string, valueCol: number}|null}
 */
function splitKeyValue(
  s,
  sCol,
  ctx,
  lineNum,
  inlineCol,
  syntax = scanTopLevel(s),
) {
  const colonIdx = findKeyColonOutsideQuotes(s, syntax);
  if (colonIdx < 0) return null;
  const keyRaw = s.slice(0, colonIdx);
  if (keyRaw.endsWith(" ")) {
    const col = sCol + Math.max(0, keyRaw.length - 1);
    throw new Error(
      'Unexpected space before ":"' + locSuffix(ctx, lineNum, col),
    );
  }
  let key = keyRaw;
  if (keyRaw.startsWith('"') || keyRaw.startsWith("'")) {
    const quote = keyRaw[0];
    if (keyRaw.length < 2 || keyRaw[keyRaw.length - 1] !== quote) {
      const col = sCol + Math.max(0, keyRaw.length - 1);
      throw new Error("Unterminated string" + locSuffix(ctx, lineNum, col));
    }
    key = parseQuotedString(keyRaw, ctx, lineNum, sCol);
  } else {
    if (keyRaw.length === 0) {
      throw new Error("Missing key" + locSuffix(ctx, lineNum, sCol + colonIdx));
    }
    if (
      keyRaw.startsWith("<") &&
      keyRaw.endsWith(">") &&
      inlineCol === undefined
    ) {
      // A reference or bytes is a value, never a key: the colon is stray.
      // In an inline object, "<" is a bad key's first character, below.
      throw new Error(
        'Unexpected character ":"' + locSuffix(ctx, lineNum, sCol + colonIdx),
      );
    }
    for (let i = 0; i < keyRaw.length; i++) {
      const c = keyRaw[i];
      if (!KEY_CHARACTER.test(c)) {
        // First character invalid = "Invalid key", subsequent = "Invalid key character"
        const errMsg = i === 0 ? "Invalid key" : "Invalid key character";
        // For inline objects, report column of opening brace; otherwise report character position
        const errCol =
          i === 0 && inlineCol !== undefined ? inlineCol : sCol + i;
        throw new Error(errMsg + locSuffix(ctx, lineNum, errCol));
      }
    }
  }
  const valueSlice = s.slice(colonIdx + 1);
  let valuePart = valueSlice;
  let valueCol = sCol + colonIdx + 1;
  if (valueSlice.length > 0 && !valueSlice.startsWith(" ")) {
    throw new Error(
      'Expected space after ":"' + locSuffix(ctx, lineNum, sCol + colonIdx),
    );
  }
  if (valueSlice.startsWith(" ")) {
    if (valueSlice.startsWith("  ")) {
      throw new Error(
        'Unexpected space after ":"' +
          locSuffix(ctx, lineNum, sCol + colonIdx + 2),
      );
    }
    valuePart = valueSlice.slice(1);
    valueCol = sCol + colonIdx + 2;
  }
  return { key, valuePart, valueCol };
}

/**
 * @param {string} s
 * @param {ReturnType<typeof scanTopLevel>=} syntax
 * @param {number=} end
 * @returns {number}
 */
function findKeyColonOutsideQuotes(
  s,
  syntax = scanTopLevel(s),
  end = s.length,
) {
  return syntax.colon !== undefined && syntax.colon < end ? syntax.colon : -1;
}

/**
 * @param {string} valuePart
 * @param {string} leader
 * @returns {boolean}
 */
function isPropertyBlockLeaderOnly(valuePart, leader) {
  if (valuePart === leader) return true;
  if (!valuePart.startsWith(leader)) return false;
  let i = 1;
  while (i < valuePart.length && valuePart[i] === " ") i++;
  if (i >= valuePart.length) return true;
  return valuePart[i] === "#";
}

/**
 * Split an inline container body at its top-level commas. Quotes, tags, and
 * references are already opaque in the shared syntax scan.
 * @param {string} body
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @param {{angles?: boolean, validateArrays?: boolean}=} options
 * @returns {{text: string, start: number}[]}
 */
function splitTopLevelCommas(
  body,
  ctx,
  lineNum,
  col,
  { angles = false, validateArrays = false } = {},
) {
  const { masked } = scanTopLevel(body);
  const parts = [];
  let start = 0;
  let braceDepth = 0;
  let bracketDepth = 0;
  let angleDepth = 0;
  for (let i = 0; i < masked.length; i++) {
    const c = masked[i];
    if (validateArrays && c === "[" && body[i + 1] === " ") {
      fail(ctx, 'Unexpected space after "["', lineNum, col + i + 1);
    }
    if (validateArrays && c === "]" && body[i - 1] === " ") {
      fail(ctx, 'Unexpected space before "]"', lineNum, col + i - 1);
    }
    if (validateArrays && c === ",") {
      if (i > 0 && body[i - 1] === " ") {
        fail(ctx, 'Unexpected space before ","', lineNum, col + i - 1);
      }
      if (i + 1 < body.length && body[i + 1] !== " " && body[i + 1] !== "]") {
        fail(ctx, 'Expected space after ","', lineNum, col + i);
      }
      if (body[i + 1] === " " && body[i + 2] === " ") {
        fail(ctx, 'Unexpected space after ","', lineNum, col + i + 2);
      }
    }
    if (c === "{") braceDepth++;
    else if (c === "}" && braceDepth > 0) braceDepth--;
    else if (c === "[") bracketDepth++;
    else if (c === "]" && bracketDepth > 0) bracketDepth--;
    else if (angles && c === "<") angleDepth++;
    else if (angles && c === ">" && angleDepth > 0) angleDepth--;
    else if (
      c === "," &&
      braceDepth === 0 &&
      bracketDepth === 0 &&
      angleDepth === 0
    ) {
      if (!validateArrays && i > 0 && body[i - 1] === " ") {
        fail(ctx, 'Unexpected space before ","', lineNum, col + i - 1);
      }
      if (i + 1 >= body.length || body[i + 1] !== " ") {
        fail(ctx, 'Expected space after ","', lineNum, col + i);
      }
      parts.push({ text: body.slice(start, i), start });
      start = i + 2;
    }
  }
  parts.push({ text: body.slice(start), start });
  return parts;
}

/**
 * @param {string} s
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {Record<string, unknown>|null}
 */
function parseInlineObject(s, ctx = {}, lineNum = 0, col = 0) {
  return nested(ctx, lineNum, col, () =>
    parseInlineObjectBody(s, ctx, lineNum, col),
  );
}

function parseInlineObjectBody(s, ctx, lineNum, col) {
  if (!s.startsWith("{")) return null;
  if (!s.includes("}")) {
    return null;
  }
  if (!s.endsWith("}")) {
    throw new Error(
      "Unexpected inline object content" + locSuffix(ctx, lineNum, col),
    );
  }
  if (s === "{}") return {};
  if (s[1] === " ") {
    throw new Error(
      'Unexpected space after "{"' + locSuffix(ctx, lineNum, col + 1),
    );
  }
  if (s[s.length - 2] === " ") {
    throw new Error(
      'Unexpected space before "}"' +
        locSuffix(ctx, lineNum, col + s.length - 2),
    );
  }
  const body = s.slice(1, -1);
  const parts = splitTopLevelCommas(body, ctx, lineNum, col + 1);
  const obj = {};
  /** @type {Map<string, [number, number]>} */
  const seen = new Map();
  for (const part of parts) {
    if (part.text.length === 0) {
      throw new Error("Missing key" + locSuffix(ctx, lineNum, col + 1));
    }
    const partCol = col + 1 + part.start;
    const keyValue = splitKeyValue(part.text, partCol, ctx, lineNum, col);
    if (!keyValue) {
      throw new Error(
        "Expected colon after key" + locSuffix(ctx, lineNum, col),
      );
    }
    const { key, valuePart, valueCol } = keyValue;
    claimKey(seen, key, ctx, lineNum, partCol);
    if (valuePart === "") {
      throw new Error("Missing value" + locSuffix(ctx, lineNum, valueCol));
    }
    setOwn(obj, key, parseScalar(valuePart, ctx, lineNum, valueCol));
  }
  return obj;
}

/**
 * @param {string} s
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {unknown}
 */
function parseScalar(
  s,
  ctx = {},
  lineNum = 0,
  col = 0,
  syntax = scanTopLevel(s),
) {
  // Strip inline comments first
  s = stripInlineComment(s, ctx, lineNum, col, syntax);
  const first =
    syntax.firstTag !== undefined && syntax.firstTag < s.length
      ? syntax.firstTag
      : undefined;
  if (first !== undefined) {
    if (first === 0) throw prefixWithoutBlock(ctx, lineNum, col);
    const atomEnd = s.slice(0, first).trimEnd().length;
    const [tags, n] = parseSuffix(
      s.slice(atomEnd),
      ctx,
      lineNum,
      col + atomEnd,
    );
    if (atomEnd + n !== s.length) {
      throw new Error(
        `Unexpected character "${s[atomEnd + n]}"` +
          locSuffix(ctx, lineNum, col + atomEnd + n),
      );
    }
    const atom = s.slice(0, atomEnd);
    return wrapTags(parseScalarAtom(atom, ctx, lineNum, col), tags);
  }
  return parseScalarAtom(s, ctx, lineNum, col);
}

function parseScalarAtom(s, ctx = {}, lineNum = 0, col = 0) {
  if (s === "null") return null;
  if (s === "true") return true;
  if (s === "false") return false;
  if (s === "nan") return NaN;
  if (s === "infinity") return Infinity;
  if (s === "-infinity") return -Infinity;
  const num = parseNumber(s, ctx, lineNum, col);
  if (num !== undefined) return num;
  if (s.startsWith('"')) {
    if (s.length < 2 || !s.endsWith('"')) {
      // Closed early, with more after it, or never closed
      let k = 1;
      while (k < s.length && s[k] !== '"') k += s[k] === "\\" ? 2 : 1;
      if (k < s.length) {
        throw new Error(
          `Unexpected character "${s[0]}"` + locSuffix(ctx, lineNum, col),
        );
      }
    }
    return parseQuotedString(s, ctx, lineNum, col);
  }
  if (s.startsWith("'")) return parseSingleQuotedString(s, ctx, lineNum, col);
  if (s.startsWith("[")) {
    // Inline arrays must close on the same line.
    if (!s.endsWith("]")) {
      throw new Error(
        "Unexpected newline in inline array" + locSuffix(ctx, lineNum, col),
      );
    }
    return parseInlineArray(s, ctx, lineNum, col);
  }
  if (s.startsWith("{")) {
    // Inline objects must close on the same line.
    if (!s.includes("}")) {
      throw new Error(
        "Unexpected newline in inline object" + locSuffix(ctx, lineNum, col),
      );
    }
    return parseInlineObject(s, ctx, lineNum, col);
  }
  if (isReferenceStart(s)) {
    const [value, n] = parseReference(s, ctx, lineNum, col);
    if (n !== s.length) {
      throw new Error(
        `Unexpected character "${s[n]}"` + locSuffix(ctx, lineNum, col + n),
      );
    }
    return value;
  }
  if (s.startsWith("<")) return parseAngleBytes(s, ctx, lineNum, col);
  // Bare words are not valid - strings must be quoted
  const firstChar = s.charAt(0) || "?";
  throw new Error(
    `Unexpected character "${firstChar}"` + locSuffix(ctx, lineNum, col),
  );
}

/**
 * @param {string} s
 * @returns {number|bigint|undefined}
 */
/**
 * @param {string} s
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {number|bigint|undefined}
 */
function parseNumber(s, ctx = {}, lineNum = 0, col = 0) {
  // Check for uppercase E in exponent (must be lowercase)
  // Only check if this looks like a number (starts with digit, minus, or dot)
  const firstNonSpace = s.replace(/^ */, "")[0];
  if (
    (firstNonSpace >= "0" && firstNonSpace <= "9") ||
    firstNonSpace === "-" ||
    firstNonSpace === "."
  ) {
    const eIdx = s.indexOf("E");
    if (eIdx >= 0) {
      throw new Error(
        "Uppercase exponent (use lowercase 'e')" +
          locSuffix(ctx, lineNum, col + eIdx),
      );
    }
  }

  let hasDigit = false;
  let hasExponent = false;
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (c === " ") continue;
    if (c >= "0" && c <= "9") {
      hasDigit = true;
      continue;
    }
    if (c === ".") continue;
    if (c === "-" && i === 0) continue;
    // Allow 'e' for exponent notation (E already rejected above)
    if (c === "e" && hasDigit && !hasExponent) {
      hasExponent = true;
      continue;
    }
    // Allow +/- after exponent
    if ((c === "+" || c === "-") && hasExponent) {
      const prev = i > 0 ? s[i - 1] : "";
      if (prev === "e") continue;
    }
    // Not a numeric candidate.
    return undefined;
  }
  if (!hasDigit) return undefined;
  for (let i = 0; i < s.length; i++) {
    if (s[i] !== " ") continue;
    const prev = i > 0 ? s[i - 1] : "";
    const next = i + 1 < s.length ? s[i + 1] : "";
    const isDigitPrev = prev >= "0" && prev <= "9";
    const isDigitNext = next >= "0" && next <= "9";
    if (!(isDigitPrev && isDigitNext)) {
      throw new Error(
        "Unexpected space in number" + locSuffix(ctx, lineNum, col + i),
      );
    }
  }
  const compact = s.replace(/ /g, "");
  if (/^-?\d+$/.test(compact)) return BigInt(compact);
  // Float patterns: with decimal point, or with exponent, or both
  if (
    /^-?\d*\.\d*([eE][+-]?\d+)?$/.test(compact) ||
    /^-?\d+[eE][+-]?\d+$/.test(compact)
  ) {
    const n = Number(compact);
    if (!isNaN(n)) return n;
  }
  return undefined;
}

/**
 * @param {string} s
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {string}
 */
function parseQuotedString(s, ctx = {}, lineNum = 0, col = 0) {
  if (s.startsWith('"')) return parseJsonQuotedString(s, ctx, lineNum, col);
  if (s.startsWith("'")) return parseSingleQuotedString(s, ctx, lineNum, col);
  return s;
}

/**
 * The deepest inline arrays and objects may nest, one in another. Deeper
 * nesting is refused rather than overflowing the stack.
 */
const MAX_INLINE_DEPTH = 256;

/**
 * Read one level of inline nesting with read(), refusing more than
 * MAX_INLINE_DEPTH levels.
 * @template T
 * @param {ParseContext & {depth?: number}} ctx
 * @param {number} lineNum
 * @param {number} col
 * @param {() => T} read
 * @returns {T}
 */
function nested(ctx, lineNum, col, read) {
  const depth = (ctx.depth ?? 0) + 1;
  if (depth > MAX_INLINE_DEPTH) {
    throw new Error(
      `Inline nesting deeper than ${MAX_INLINE_DEPTH} levels` +
        locSuffix(ctx, lineNum, col),
    );
  }
  ctx.depth = depth;
  try {
    return read();
  } finally {
    ctx.depth = depth - 1;
  }
}

/**
 * Read a single-quoted string, the whole of s. Its only escapes are \' and
 * \\; any other backslash is literal.
 * @param {string} s
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {string}
 */
function parseSingleQuotedString(s, ctx = {}, lineNum = 0, col = 0) {
  let out = "";
  let run = 1;
  for (let i = 1; i < s.length; i++) {
    const c = s[i];
    if (c === "\\" && (s[i + 1] === "'" || s[i + 1] === "\\")) {
      out += s.slice(run, i);
      run = i + 1;
      i++;
    } else if (c === "'") {
      if (i !== s.length - 1) {
        throw new Error(
          'Unexpected character "\'"' + locSuffix(ctx, lineNum, col),
        );
      }
      return out + s.slice(run, i);
    }
  }
  throw new Error(
    "Unterminated string" + locSuffix(ctx, lineNum, col + s.length - 1),
  );
}

/**
 * Read the escape beginning at s[index]. A quoted string escapes its quote;
 * a tag escapes its closing parenthesis. Both otherwise use the same JSON
 * escapes and YAY's braced Unicode escape.
 * @param {string} s
 * @param {number} index
 * @param {string} terminator
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {[string, number]}
 */
function readEscape(s, index, terminator, ctx, lineNum, col) {
  const escape = s[index + 1];
  if (escape === terminator) return [terminator, index + 2];
  if (escape !== undefined && Object.hasOwn(SIMPLE_ESCAPES, escape)) {
    return [SIMPLE_ESCAPES[escape], index + 2];
  }
  if (escape !== "u" || s[index + 2] !== "{") {
    fail(ctx, "Bad escaped character", lineNum, col + index + 1);
  }
  const close = s.indexOf("}", index + 3);
  const hex = close < 0 ? "" : s.slice(index + 3, close);
  if (!UNICODE_ESCAPE.test(hex)) {
    fail(ctx, "Bad Unicode escape", lineNum, col + index + 2);
  }
  const code = parseInt(hex, 16);
  if (code >= 0xd800 && code <= 0xdfff) {
    fail(ctx, "Illegal surrogate", lineNum, col + index + 2);
  }
  if (code > 0x10ffff) {
    fail(ctx, "Unicode code point out of range", lineNum, col + index + 2);
  }
  return [String.fromCodePoint(code), close + 1];
}

/**
 * Minimal JSON string parser for deterministic errors.
 * @param {string} s
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {string}
 */
function parseJsonQuotedString(s, ctx = {}, lineNum = 0, col = 0) {
  if (s.length < 2 || s[0] !== '"') return s;
  if (s[s.length - 1] !== '"') {
    const index = Math.max(0, s.length - 1);
    throw new Error(
      "Unterminated string" + locSuffix(ctx, lineNum, col + index),
    );
  }
  let out = "";
  for (let i = 1; i < s.length - 1; i++) {
    const ch = s[i];
    if (ch === "\\") {
      const [value, next] = readEscape(s, i, '"', ctx, lineNum, col);
      out += value;
      i = next - 1;
    } else {
      // Unescaped control characters are illegal in JSON strings.
      const code = ch.charCodeAt(0);
      if (code < 0x20) {
        throw new Error(
          "Bad character in string" + locSuffix(ctx, lineNum, col + i),
        );
      }
      out += ch;
    }
  }
  return out;
}

/**
 * @param {string} s
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {unknown[]}
 */
function parseInlineArray(s, ctx = {}, lineNum = 0, col = 0) {
  return nested(ctx, lineNum, col, () =>
    parseInlineArrayBody(s, ctx, lineNum, col),
  );
}

function parseInlineArrayBody(s, ctx, lineNum, col) {
  if (!s.startsWith("[")) return [];
  if (!s.endsWith("]")) {
    throw new Error("Unterminated inline array" + locSuffix(ctx, lineNum, col));
  }
  if (s === "[]") return [];
  if (s[1] === " ") {
    throw new Error(
      'Unexpected space after "["' + locSuffix(ctx, lineNum, col + 1),
    );
  }
  if (s[s.length - 2] === " ") {
    throw new Error(
      'Unexpected space before "]"' +
        locSuffix(ctx, lineNum, col + s.length - 2),
    );
  }
  const body = s.slice(1, -1);
  const parts = splitTopLevelCommas(body, ctx, lineNum, col + 1, {
    angles: true,
    validateArrays: true,
  });
  const arr = [];
  for (const part of parts) {
    if (part.text.length === 0) {
      throw new Error(
        "Missing array element" + locSuffix(ctx, lineNum, col + 1 + part.start),
      );
    }
    const partCol = col + 1 + part.start;
    arr.push(parseScalar(part.text, ctx, lineNum, partCol));
  }
  return arr;
}

/**
 * @param {string} s
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {Uint8Array}
 */
function parseAngleBytes(s, ctx = {}, lineNum = 0, col = 0) {
  if (!s.endsWith(">")) {
    throw new Error("Unmatched angle bracket" + locSuffix(ctx, lineNum, col));
  }
  if (s === "<>") return new Uint8Array(0);
  if (s.length > 2 && s[1] === " ") {
    throw new Error(
      'Unexpected space after "<"' + locSuffix(ctx, lineNum, col + 1),
    );
  }
  if (s.length > 2 && s[s.length - 2] === " ") {
    throw new Error(
      'Unexpected space before ">"' +
        locSuffix(ctx, lineNum, col + s.length - 2),
    );
  }
  const inner = s.slice(1, -1);
  // Check for uppercase hex digits
  for (let i = 0; i < inner.length; i++) {
    const c = inner[i];
    if (c >= "A" && c <= "F") {
      throw new Error(
        "Uppercase hex digit (use lowercase)" +
          locSuffix(ctx, lineNum, col + 1 + i),
      );
    }
  }
  return hexToBytes(inner.replace(/\s/g, ""), ctx, lineNum, col);
}

/**
 * Decode lowercase hex, inline or block, the spaces already removed.
 * Uppercase digits are refused where they are written, before this.
 * @param {string} hex
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {Uint8Array}
 */
function hexToBytes(hex, ctx, lineNum, col) {
  if (hex.length % 2 !== 0)
    throw new Error(
      "Odd number of hex digits in byte literal" + locSuffix(ctx, lineNum, col),
    );
  if (!LOWER_HEX.test(hex))
    throw new Error("Invalid hex digit" + locSuffix(ctx, lineNum, col));
  const bytes = new Uint8Array(hex.length / 2);
  for (let j = 0; j < bytes.length; j++) {
    bytes[j] = parseInt(hex.slice(j * 2, j * 2 + 2), 16);
  }
  return bytes;
}

/**
 * Read one line of block bytes: hex, then an optional comment after two
 * spaces, or a comment alone. Returns the hex without its spaces.
 * @param {string} text
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {string}
 */
function blockHexLine(text, ctx, lineNum, col) {
  if (text.startsWith("#")) return "";
  let body = text;
  const hash = text.indexOf("#");
  if (hash >= 0) {
    checkCommentSpacing(text, hash, ctx, lineNum, col);
    if (text[hash - 1] === " ") body = text.slice(0, hash).trimEnd();
  }
  for (let i = 0; i < body.length; i++) {
    if (body[i] >= "A" && body[i] <= "F") {
      throw new Error(
        "Uppercase hex digit (use lowercase)" +
          locSuffix(ctx, lineNum, col + i),
      );
    }
  }
  return body.replace(/ /g, "");
}

/**
 * @param {Token[]} tokens
 * @param {number} i
 * @param {ParseContext} ctx
 * @returns {[unknown[], number]}
 */
function parseListArray(tokens, i, ctx = {}, minIndent = -1) {
  const arr = [];
  while (
    i < tokens.length &&
    tokens[i].type === "start" &&
    tokens[i].text === "-"
  ) {
    const listIndent = tokens[i].indent ?? 0;
    // Stop if we encounter a list item at a lower indent than expected
    if (minIndent >= 0 && listIndent < minIndent) break;
    i = skipBlank(tokens, i + 1);
    if (i >= tokens.length) break;
    const [value, j, some] = parseListItem(tokens, i, listIndent, ctx);
    if (some !== false) arr.push(value);
    i = j;
    // Skip stops and breaks between items
    i = skipBlank(tokens, i, true);
  }
  return [arr, i];
}

/**
 * Parse one list item's content, at tokens[i], after its "-".
 * @param {Token[]} tokens
 * @param {number} i
 * @param {number} listIndent
 * @param {ParseContext} ctx
 * @returns {[unknown, number, boolean?]} the value, the next index, and
 * false when the item holds no value
 */
function parseListItem(tokens, i, listIndent, ctx) {
  const next = tokens[i];
  // A tagged block: the item's content follows the tags
  if (next.type === "text" && next.text.startsWith("(")) {
    const lineNum = next.lineNum ?? 0;
    const col = next.col ?? 0;
    const [tags, rest] = parsePrefix(next.text, ctx, lineNum, col);
    if (rest === "") {
      const j = skipBlank(tokens, i + 1);
      if (
        j >= tokens.length ||
        tokens[j].type === "stop" ||
        (tokens[j].indent ?? 0) <= listIndent
      ) {
        throw prefixWithoutBlock(ctx, lineNum, col);
      }
      const [value, k] = parseListItem(tokens, j, listIndent, ctx);
      return [wrapPrefix(value, tags), k];
    }
    if (!prefixTakes(rest)) throw prefixWithoutBlock(ctx, lineNum, col);
    const retokened = tokens.slice();
    retokened[i] = {
      ...next,
      text: rest,
      col: col + next.text.length - rest.length,
    };
    const [value, k] = parseListItem(retokened, i, listIndent, ctx);
    return [wrapPrefix(value, tags), k];
  }
  if (
    next.type === "text" &&
    next.text === "" &&
    i + 1 < tokens.length &&
    tokens[i + 1].type === "start" &&
    tokens[i + 1].text === "-"
  ) {
    return parseListArray(tokens, i + 1, ctx);
  } else if (next.type === "start" && next.text === "-") {
    return parseListArray(tokens, i, ctx);
  } else if (
    next.type === "text" &&
    (next.indent ?? 0) >= listIndent &&
    isInlineBullet(next.text)
  ) {
    // Inline bullet list inside a multiline list item.
    const group = [];
    let j = i;
    for (;;) {
      if (
        j < tokens.length &&
        tokens[j].type === "text" &&
        (tokens[j].indent ?? 0) >= listIndent &&
        isInlineBullet(tokens[j].text)
      ) {
        const valStr = parseInlineBulletValue(
          tokens[j].text,
          ctx,
          tokens[j].lineNum ?? 0,
          tokens[j].col ?? 0,
        );
        group.push(
          parseNestedInlineBullet(
            valStr,
            ctx,
            tokens[j].lineNum ?? 0,
            (tokens[j].col ?? 0) + 2,
          ),
        );
        j++;
      } else if (
        j < tokens.length &&
        tokens[j].type === "start" &&
        tokens[j].text === "-" &&
        (tokens[j].indent ?? 0) > listIndent &&
        j + 1 < tokens.length &&
        tokens[j + 1].type === "text" &&
        isInlineBullet(tokens[j + 1].text)
      ) {
        const valStr = parseInlineBulletValue(
          tokens[j + 1].text,
          ctx,
          tokens[j + 1].lineNum ?? 0,
          tokens[j + 1].col ?? 0,
        );
        group.push(
          parseNestedInlineBullet(
            valStr,
            ctx,
            tokens[j + 1].lineNum ?? 0,
            (tokens[j + 1].col ?? 0) + 2,
          ),
        );
        j += 2;
      } else {
        break;
      }
    }
    // If next token is start(-) at deeper indent, same list continues with nested bullets (e.g. "- - a" then "  - b").
    while (
      j < tokens.length &&
      tokens[j].type === "start" &&
      tokens[j].text === "-" &&
      (tokens[j].indent ?? 0) > listIndent
    ) {
      j++;
      j = skipBlank(tokens, j);
      if (j >= tokens.length) break;
      const [subVal, nextJ] = parseValue(tokens, j, ctx);
      group.push(subVal);
      j = nextJ;
      while (j < tokens.length && tokens[j].type === "stop") j++;
    }
    return [group, j];
  } else if (
    next.type === "text" &&
    !next.text.startsWith("[") &&
    !next.text.startsWith("{") &&
    findKeyColonOutsideQuotes(next.text) >= 0
  ) {
    const nextIndent = next.indent ?? 0;
    const inlineIndent = nextIndent === listIndent ? nextIndent : undefined;
    const baseIndent = inlineIndent !== undefined ? nextIndent + 2 : nextIndent;
    const [obj, nextIndex] = parseObjectBlock(
      tokens,
      i,
      baseIndent,
      ctx,
      inlineIndent,
    );
    return [obj, nextIndex];
  } else if (next.type === "text" || next.type === "start") {
    const [value, j] = parseValue(tokens, i, ctx);
    const k = skipBlank(tokens, j);
    const afterBreak = k < tokens.length ? tokens[k] : null;
    if (
      afterBreak &&
      afterBreak.type === "start" &&
      afterBreak.text === "-" &&
      (afterBreak.indent ?? 0) > listIndent
    ) {
      const group = [value];
      i = k;
      while (
        i < tokens.length &&
        tokens[i].type === "start" &&
        tokens[i].text === "-" &&
        (tokens[i].indent ?? 0) > listIndent
      ) {
        i = skipBlank(tokens, i + 1);
        if (i >= tokens.length) break;
        const [subVal, nextI] = parseValue(tokens, i, ctx);
        group.push(subVal);
        i = nextI;
        while (i < tokens.length && tokens[i].type === "stop") i++;
      }
      return [group, i];
    }
    return [value, j];
  }
  return [undefined, i + 1, false];
}

/**
 * Parse a block string with an optional base indent constraint.
 * @param {Token[]} tokens
 * @param {number} i
 * @param {string|undefined} firstLine
 * @param {boolean} inPropertyContext
 * @param {number} baseIndent - If >= 0, stop collecting when indent <= baseIndent
 * @param {ParseContext} ctx
 * @returns {[string, number]}
 */
function collectBlockString(
  tokens,
  i,
  firstLine,
  inPropertyContext,
  baseIndent,
  ctx,
) {
  const startToken = tokens[i];
  const lines = [];
  if (firstLine !== undefined) {
    lines.push(firstLine);
    i++;
  } else {
    i++;
  }
  // Collect continuation lines with their indent so we can strip minimum indent (for property block strings).
  const continuationLines = [];
  while (
    i < tokens.length &&
    (tokens[i].type === "text" || tokens[i].type === "break")
  ) {
    if (tokens[i].type === "break") {
      continuationLines.push({ indent: undefined, text: "" });
      i++;
    } else {
      // If we have a base indent constraint, stop when we see a line at or below that indent
      if (baseIndent >= 0 && (tokens[i].indent ?? 0) <= baseIndent) {
        break;
      }
      continuationLines.push({
        indent: tokens[i].indent ?? 0,
        text: tokens[i].text,
      });
      i++;
    }
  }
  const minIndent = continuationLines
    .filter((x) => x.indent !== undefined)
    .reduce((min, x) => (x.indent < min ? x.indent : min), Infinity);
  const effectiveMin = minIndent === Infinity ? 0 : minIndent;
  for (const { indent, text } of continuationLines) {
    if (indent === undefined) {
      lines.push("");
    } else {
      // Token text is already after indent; add back (indent - minIndent) spaces for relative indent.
      const extraSpaces = indent - effectiveMin;
      lines.push((extraSpaces > 0 ? " ".repeat(extraSpaces) : "") + text);
    }
  }
  // Trim leading and trailing empty lines; then one leading newline and one trailing newline.
  let start = 0;
  while (start < lines.length && lines[start] === "") start++;
  let end = lines.length;
  while (end > start && lines[end - 1] === "") end--;
  const trimmed = lines.slice(start, end);
  // When block starts with quote on its own line (firstLine === ''), output has a leading newline.
  // But NOT in property context.
  const leadingNewline =
    firstLine === "" && trimmed.length > 0 && !inPropertyContext;
  const body =
    (leadingNewline ? "\n" : "") +
    trimmed.join("\n") +
    (trimmed.length > 0 ? "\n" : "");
  // Empty block strings are not allowed - use "" for empty string
  if (body === "") {
    fail(
      ctx,
      'Empty block string not allowed (use "" or "\\n" explicitly)',
      startToken.lineNum ?? 0,
      startToken.col ?? 0,
      false,
    );
  }
  return [body, i];
}

/**
 * Parse concatenated quoted strings (multiple quoted strings on consecutive lines).
 * Returns null if there's only one string (single string on new line is invalid).
 * @param {Token[]} tokens
 * @param {number} i
 * @param {number} baseIndent
 * @param {ParseContext} ctx
 * @returns {[string, number] | null}
 */
function parseConcatenatedStrings(tokens, i, baseIndent, ctx = {}) {
  const parts = [];
  const startI = i;

  while (i < tokens.length) {
    const t = tokens[i];

    if (t.type === "break" || t.type === "stop") {
      i++;
      continue;
    }

    if (t.type !== "text" || (t.indent ?? 0) < baseIndent) {
      break;
    }

    const trimmed = t.text.trim();

    // Check if this line is a quoted string
    const isDoubleQuoted =
      trimmed.startsWith('"') && trimmed.endsWith('"') && trimmed.length >= 2;
    const isSingleQuoted =
      trimmed.startsWith("'") && trimmed.endsWith("'") && trimmed.length >= 2;

    if (!isDoubleQuoted && !isSingleQuoted) {
      break;
    }

    // Parse the quoted string
    const parsed = parseQuotedString(trimmed, ctx, t.lineNum ?? 0, t.col ?? 0);
    parts.push(parsed);
    i++;
  }

  // Require at least 2 strings for concatenation
  // A single string on a new line is invalid (use inline syntax instead)
  if (parts.length < 2) {
    return null;
  }

  return [parts.join(""), i];
}

/**
 * @param {string} text
 * @returns {boolean}
 */
function isInlineBullet(text) {
  let i = 0;
  while (i < text.length && text[i] === " ") i++;
  return (
    i < text.length &&
    text[i] === "-" &&
    i + 1 < text.length &&
    text[i + 1] === " "
  );
}

/**
 * @param {string} text
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {string}
 */
function parseInlineBulletValue(text, ctx = {}, lineNum = 0, col = 0) {
  let i = 0;
  while (i < text.length && text[i] === " ") i++;
  if (i >= text.length || text[i] !== "-") return "";
  const dashIndex = i;
  const afterDash = dashIndex + 1;
  if (afterDash >= text.length || text[afterDash] !== " ") return "";
  if (afterDash + 1 < text.length && text[afterDash + 1] === " ") {
    throw new Error(
      'Unexpected space after "-"' +
        locSuffix(ctx, lineNum, col + afterDash + 1),
    );
  }
  return text.slice(afterDash + 1);
}

/**
 * Recursively parse an inline bullet value, handling nested "- " prefixes.
 * Returns the parsed value (could be a nested array or a scalar).
 */
function parseNestedInlineBullet(text, ctx = {}, lineNum = 0, col = 0) {
  // Check if the text itself is another inline bullet
  if (isInlineBullet(text)) {
    const innerText = parseInlineBulletValue(text, ctx, lineNum, col);
    const innerValue = parseNestedInlineBullet(
      innerText,
      ctx,
      lineNum,
      col + 2,
    );
    return [innerValue];
  }
  // Otherwise, parse as a scalar
  return parseScalar(text, ctx, lineNum, col);
}

// Note: parseMultilineBytes for '*' syntax was removed as dead code.
// The scanner rejects '*' syntax, so this function was unreachable.

/**
 * @param {Token[]} tokens
 * @param {number} i
 * @param {ParseContext} ctx
 * @param {string} firstLineRaw
 * @param {number} baseIndent
 * @returns {[Uint8Array, number]}
 */
function parseBlockBytes(
  tokens,
  i,
  ctx = {},
  firstLineRaw,
  baseIndent,
  firstCol = 0,
) {
  const startToken = tokens[i];
  const lineNum = startToken.lineNum ?? 0;
  const col = startToken.col ?? 0;
  let hex = "";
  if (firstLineRaw) {
    hex += blockHexLine(firstLineRaw, ctx, lineNum, firstCol);
  }
  i++;
  while (i < tokens.length) {
    const t = tokens[i];
    if (t.type === "break") {
      i++;
      continue;
    }
    if (t.type === "text" && (t.indent ?? 0) > baseIndent) {
      hex += blockHexLine(t.text, ctx, t.lineNum ?? 0, t.col ?? 0);
      i++;
      continue;
    }
    break;
  }
  return [hexToBytes(hex, ctx, lineNum, col), i];
}

// Note: parseMultilineAngleBytes was removed as dead code.
// The "< hex" syntax (without closing ">") is invalid - inline byte arrays must be closed on the same line.

/**
 * Read the value of one object property. The three object readers differ only
 * in how an empty value chooses the following block, captured by emptyMode.
 * @param {Token[]} tokens
 * @param {number} i
 * @param {{key: string, valuePart: string, valueCol: number}} keyValue
 * @param {ParseContext} ctx
 * @param {{baseIndent: number, blockIndent?: number, emptyMode: "root"|"named"|"block"}} options
 * @returns {[unknown, number]}
 */
function parsePropertyValue(
  tokens,
  i,
  keyValue,
  ctx,
  { baseIndent, blockIndent = baseIndent, emptyMode },
) {
  const token = tokens[i];
  const lineNum = token.lineNum ?? 0;
  let valuePart = keyValue.valuePart;
  let tags = null;
  if (valuePart.startsWith("(")) {
    [tags, valuePart] = parsePrefix(valuePart, ctx, lineNum, keyValue.valueCol);
    if (!prefixTakes(valuePart)) {
      throw prefixWithoutBlock(ctx, lineNum, keyValue.valueCol);
    }
  }

  let value;
  let next;
  if (valuePart.startsWith(">")) {
    if (!isPropertyBlockLeaderOnly(valuePart, ">")) {
      fail(
        ctx,
        "Expected newline after block leader in property",
        lineNum,
        keyValue.valueCol,
        false,
      );
    }
    [value, next] = parseBlockBytes(tokens, i, ctx, "", baseIndent);
  } else if (valuePart === "{}") {
    value = {};
    next = i + 1;
  } else if (valuePart.startsWith("`")) {
    if (valuePart !== "`") {
      fail(
        ctx,
        "Expected newline after block leader in property",
        lineNum,
        keyValue.valueCol,
        false,
      );
    }
    [value, next] = collectBlockString(tokens, i, "", true, blockIndent, ctx);
  } else if (valuePart === "") {
    if (emptyMode === "root") {
      const [valueObject, after] = parseObjectOrNamedArray(
        tokens,
        i,
        keyValue.key,
        ctx,
      );
      value = valueObject[keyValue.key];
      next = after;
    } else {
      next = skipBlank(tokens, i + 1, emptyMode === "block");
      const following = tokens[next];
      if (emptyMode === "block") {
        if (
          !following ||
          (following.indent ?? 0) < baseIndent ||
          ((following.indent ?? 0) === baseIndent && following.type !== "start")
        ) {
          if (tags) {
            throw prefixWithoutBlock(ctx, lineNum, keyValue.valueCol);
          }
          fail(
            ctx,
            "Expected value after property",
            lineNum,
            (token.col ?? 0) + keyValue.key.length + 1,
          );
        }
      }
      if (following?.type === "start" && following.text === "-") {
        [value, next] = parseListArray(
          tokens,
          next,
          ctx,
          emptyMode === "block" ? baseIndent : -1,
        );
      } else {
        const parentIndent =
          emptyMode === "block" ? baseIndent : (token.indent ?? 0);
        if (
          following?.type === "text" &&
          (following.indent ?? 0) > parentIndent
        ) {
          [value, next] = parseObjectBlock(
            tokens,
            next,
            following.indent ?? 0,
            ctx,
          );
        } else if (emptyMode === "named") {
          fail(
            ctx,
            "Expected value after property",
            lineNum,
            (token.col ?? 0) + keyValue.key.length + 1,
          );
        }
      }
    }
  } else {
    value = parseScalar(valuePart, ctx, lineNum, keyValue.valueCol);
    next = i + 1;
  }

  if (tags) {
    if (isMissingBlock(value)) {
      throw prefixWithoutBlock(ctx, lineNum, keyValue.valueCol);
    }
    value = wrapPrefix(value, tags);
  }
  return [value, next];
}

/**
 * @param {Token[]} tokens
 * @param {number} i
 * @param {string} key
 * @param {ParseContext} ctx
 * @returns {[Record<string, unknown>, number]}
 */
function parseObjectOrNamedArray(tokens, i, key, ctx = {}) {
  const keyToken = tokens[i];
  const keyCol = keyToken.col ?? keyToken.indent ?? 0;
  i = skipBlank(tokens, i + 1, true);
  const baseIndent = i < tokens.length ? (tokens[i].indent ?? 0) : 0;
  const first = i < tokens.length ? tokens[i] : null;
  // A block array may stand at the key's column. Every other block value
  // must be indented deeper, so a sibling item or property is no value.
  if (
    !first ||
    (first.indent ?? 0) < keyCol ||
    ((first.indent ?? 0) === keyCol && first.type !== "start")
  ) {
    const col = (keyToken.col ?? 0) + key.length + 1;
    throw new Error(
      "Expected value after property" +
        locSuffix(ctx, keyToken.lineNum ?? 0, col),
    );
  }
  if (first && first.type === "start" && first.text === "-") {
    const [arr, next] = parseListArray(tokens, i, ctx);
    return [{ [key]: arr }, next];
  }
  // Note: '*' syntax for multiline bytes is rejected by the scanner
  // Note: "key: <" without closing ">" is invalid - inline byte arrays must be closed on the same line
  // Reject a block string leader or a lone quote on a separate line
  if (
    first &&
    first.type === "text" &&
    (first.text.trim() === "`" || first.text === '"')
  ) {
    throw new Error(
      "Unexpected indent" + locSuffix(ctx, first.lineNum ?? 0, 0),
    );
  }
  // Concatenated quoted strings (multiple quoted strings on consecutive lines).
  // A quoted line with a colon outside its quotes is a property with a quoted
  // key, which opens a nested object.
  if (first && first.type === "text") {
    const trimmed = first.text.trim();
    if (
      ((trimmed.startsWith('"') &&
        trimmed.endsWith('"') &&
        trimmed.length >= 2) ||
        (trimmed.startsWith("'") &&
          trimmed.endsWith("'") &&
          trimmed.length >= 2)) &&
      !splitKeyValue(first.text, first.col ?? 0, ctx, first.lineNum ?? 0)
    ) {
      const result = parseConcatenatedStrings(tokens, i, baseIndent, ctx);
      if (result !== null) {
        const [concatStr, next] = result;
        return [{ [key]: concatStr }, next];
      }
      // Single string on new line is invalid - fall through to error
      throw new Error(
        "Unexpected indent" + locSuffix(ctx, first.lineNum ?? 0, 0),
      );
    }
  }
  const obj = {};
  /** @type {Map<string, [number, number]>} */
  const seen = new Map();
  while (i < tokens.length) {
    const t = tokens[i];
    if (t.type === "stop") {
      i++;
      continue;
    }
    if (t.type === "text") {
      const s = t.text;
      // Reject inline values on separate line (they look like keys starting with special chars)
      if (s.startsWith("{") || s.startsWith("[") || s.startsWith("<")) {
        throw new Error(
          "Unexpected indent" + locSuffix(ctx, t.lineNum ?? 0, 0),
        );
      }
      const keyValue = splitKeyValue(s, t.col ?? 0, ctx, t.lineNum ?? 0);
      if (keyValue) {
        const k = keyValue.key;
        if ((t.indent ?? 0) < baseIndent) break;
        if ((t.indent ?? 0) <= baseIndent) {
          claimKey(seen, k, ctx, t.lineNum ?? 0, t.col ?? 0);
        }
        if (k && (t.indent ?? 0) <= baseIndent) {
          const [value, next] = parsePropertyValue(tokens, i, keyValue, ctx, {
            baseIndent: t.indent ?? 0,
            emptyMode: "named",
          });
          setOwn(obj, k, value);
          i = next;
        } else {
          i++;
        }
      } else {
        // Text without colon in nested object context is invalid
        // (e.g., inline array/object/bytes/string on separate line)
        throw new Error(
          "Unexpected indent" + locSuffix(ctx, t.lineNum ?? 0, 0),
        );
      }
    } else {
      i++;
    }
  }
  return [{ [key]: Object.keys(obj).length ? obj : undefined }, i];
}

/**
 * @param {Token[]} tokens
 * @param {number} i
 * @param {number} baseIndent
 * @param {ParseContext} ctx
 * @param {number=} inlineIndent
 * @returns {[Record<string, unknown>, number]}
 */
function parseObjectBlock(tokens, i, baseIndent, ctx = {}, inlineIndent) {
  const obj = {};
  /** @type {Map<string, [number, number]>} */
  const seen = new Map();
  let firstText = true;
  while (i < tokens.length) {
    const t = tokens[i];
    if (inlineIndent !== undefined) {
      if (t.type === "stop") break;
      if (t.type === "start" && (t.indent ?? 0) <= inlineIndent) break;
    }
    if (t.type === "stop") {
      i++;
      continue;
    }
    // A list leader outdented past this object begins the next item of an
    // enclosing list.
    if (t.type === "start" && (t.indent ?? 0) < baseIndent) break;
    if (t.type !== "text") {
      i++;
      continue;
    }
    let indent = t.indent ?? 0;
    if (firstText && inlineIndent !== undefined && indent === inlineIndent) {
      indent = baseIndent;
    }
    firstText = false;
    if (indent < baseIndent) break;
    if (indent > baseIndent) {
      i++;
      continue;
    }
    const keyValue = splitKeyValue(t.text, t.col ?? 0, ctx, t.lineNum ?? 0);
    if (!keyValue) {
      i++;
      continue;
    }
    const k = keyValue.key;
    claimKey(seen, k, ctx, t.lineNum ?? 0, t.col ?? 0);
    const [value, next] = parsePropertyValue(tokens, i, keyValue, ctx, {
      baseIndent,
      blockIndent: indent,
      emptyMode: "block",
    });
    setOwn(obj, k, value);
    i = next;
  }
  return [obj, i];
}

// Root as object (multiple key: value lines at indent 0)
/**
 * @param {Token[]} tokens
 * @param {number} i
 * @param {ParseContext} ctx
 * @returns {[Record<string, unknown>, number]}
 */
function parseRootObject(tokens, i, ctx = {}) {
  const obj = {};
  /** @type {Map<string, [number, number]>} */
  const seen = new Map();
  const baseIndent = 0;
  while (i < tokens.length) {
    const t = tokens[i];
    if (t.type === "stop") {
      i++;
      continue;
    }
    if (t.type === "text") {
      const s = t.text;
      const keyValue = splitKeyValue(s, t.col ?? 0, ctx, t.lineNum ?? 0);
      if (keyValue && (t.indent ?? 0) === baseIndent) {
        const k = keyValue.key;
        claimKey(seen, k, ctx, t.lineNum ?? 0, t.col ?? 0);
        const [value, next] = parsePropertyValue(tokens, i, keyValue, ctx, {
          baseIndent,
          emptyMode: "root",
        });
        setOwn(obj, k, value);
        i = next;
      } else if ((t.indent ?? 0) > baseIndent) {
        // A line indented under a property that took no block, placed at
        // the line's start as every other unexpected indent is
        throw new Error(
          "Unexpected indent" + locSuffix(ctx, t.lineNum ?? 0, 0),
        );
      } else {
        i++;
      }
    } else {
      i++;
    }
  }
  return [obj, i];
}

// --- Tagged values ---

const PASS_STYLE = Symbol.for("passStyle");

/**
 * A tagged value: a tag, any string, and one payload, any YAY value.
 * The outer record is frozen, ready for a hardened environment to harden its
 * payload and accept it as Endo pass data.
 * @param {string} tag
 * @param {unknown} payload
 * @returns {object}
 */
function tagged(tag, payload) {
  if (typeof tag !== "string") {
    throw new TypeError("A tag must be a string");
  }
  return Object.freeze(
    Object.create(Object.prototype, {
      [PASS_STYLE]: { value: "tagged" },
      [Symbol.toStringTag]: { value: tag },
      payload: { value: payload, enumerable: true },
    }),
  );
}

/**
 * @param {unknown} value
 * @returns {boolean}
 */
function isTagged(value) {
  if (
    value === null ||
    typeof value !== "object" ||
    Object.getPrototypeOf(value) !== Object.prototype ||
    !Object.isFrozen(value)
  ) {
    return false;
  }
  const descriptors = Object.getOwnPropertyDescriptors(value);
  const passStyle = descriptors[PASS_STYLE];
  const tag = descriptors[Symbol.toStringTag];
  const payload = descriptors.payload;
  return (
    Reflect.ownKeys(descriptors).length === 3 &&
    passStyle?.value === "tagged" &&
    passStyle.enumerable === false &&
    passStyle.writable === false &&
    passStyle.configurable === false &&
    typeof tag?.value === "string" &&
    tag.enumerable === false &&
    tag.writable === false &&
    tag.configurable === false &&
    payload !== undefined &&
    "value" in payload &&
    payload.enumerable === true &&
    payload.writable === false &&
    payload.configurable === false
  );
}

/**
 * Scan syntax that may contain quotes, tags, and references once. The masked
 * text replaces their payloads with "x", unit for unit, so structural scans
 * can use the original offsets without reimplementing quote awareness.
 * @param {string} s
 * @returns {{masked: string, firstTag: number|undefined, comment: number|undefined, colon: number|undefined, closeBracket: number}}
 */
function scanTopLevel(s) {
  /** @type {[number, number][]} */
  const masks = [];
  let firstTag;
  let comment;
  let colon;
  let closeBracket = -1;
  let quote = "";
  let quoteStart = 0;
  let depth = 0;
  let bracketDepth = 0;
  let commentEscape = false;
  let i = 0;
  while (i < s.length) {
    const c = s[i];
    if (quote !== "") {
      if (c === "\\") {
        i += Math.min(2, s.length - i);
        continue;
      }
      if (c === quote) {
        masks.push([quoteStart, i]);
        quote = "";
      }
      i++;
      continue;
    }
    // A reference's text is no syntax either
    if (c === "<" && isReferenceStart(s.slice(i))) {
      const end = s.indexOf(">", i);
      if (end >= 0) {
        masks.push([i + 1, end]);
        i = end + 1;
        continue;
      }
    }
    if (c === '"' || c === "'") {
      quote = c;
      quoteStart = i + 1;
    } else if (c === "[" || c === "{") {
      depth++;
      if (c === "[") bracketDepth++;
    } else if (c === "]" || c === "}") {
      depth--;
      if (c === "]") {
        bracketDepth--;
        if (bracketDepth === 0 && closeBracket < 0) closeBracket = i;
      }
    } else if (c === "(") {
      if (depth === 0 && firstTag === undefined) firstTag = i;
      const tagStart = i + 1;
      i++;
      while (i < s.length && s[i] !== ")") {
        const n = s[i] === "\\" ? Math.min(2, s.length - i) : 1;
        i += n;
      }
      masks.push([tagStart, i]);
      continue;
    } else if (c === ":" && colon === undefined) {
      colon = i;
    }
    if (comment === undefined) {
      if (commentEscape) {
        commentEscape = false;
      } else if (c === "\\") {
        commentEscape = true;
      } else if (c === "#") {
        comment = i;
      }
    }
    i++;
  }
  if (quote !== "") masks.push([quoteStart, s.length]);
  let masked = s;
  if (masks.length > 0) {
    const chunks = [];
    let start = 0;
    for (const [from, to] of masks) {
      chunks.push(s.slice(start, from), "x".repeat(to - from));
      start = to;
    }
    chunks.push(s.slice(start));
    masked = chunks.join("");
  }
  return { masked, firstTag, comment, colon, closeBracket };
}

/**
 * Parse one tag at the start of s, which begins with "(". Returns the tag
 * and the number of units consumed, the closing ")" included. The tag is
 * escaped as a double-quoted string is, with \) in the place of \".
 * @param {string} s
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {[string, number]}
 */
function parseTag(s, ctx, lineNum, col) {
  let out = "";
  let i = 1;
  while (i < s.length) {
    const c = s[i];
    if (c === ")") return [out, i + 1];
    if (c === "\t") {
      throw new Error(
        "Tab not allowed (use spaces)" + locSuffix(ctx, lineNum, col + i),
      );
    }
    if (c !== "\\") {
      out += c;
      i++;
      continue;
    }
    const [value, next] = readEscape(s, i, ")", ctx, lineNum, col);
    out += value;
    i = next;
  }
  throw new Error("Unterminated string" + locSuffix(ctx, lineNum, col));
}

/**
 * Parse a run of tags after a value: each one space, then a tag. Returns the
 * tags, innermost first, and the number of units consumed. A tag fused to
 * its value or after two spaces is refused.
 * @param {string} s
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {[string[], number]}
 */
function parseSuffix(s, ctx, lineNum, col) {
  const tags = [];
  let pos = 0;
  for (;;) {
    let spaces = 0;
    while (s[pos + spaces] === " ") spaces++;
    if (s[pos + spaces] !== "(") break;
    if (spaces !== 1) {
      throw new Error(
        (spaces === 0
          ? "Expected one space before a tag"
          : 'Unexpected space before "("') + locSuffix(ctx, lineNum, col + pos),
      );
    }
    const [tag, n] = parseTag(s.slice(pos + 1), ctx, lineNum, col + pos + 1);
    tags.push(tag);
    pos += 1 + n;
  }
  return [tags, pos];
}

/**
 * Parse a run of tags before a block: tags separated by one space. Returns
 * the tags, outermost first, and what follows the last one (after its
 * space, or a comment, which is the empty string).
 * @param {string} s
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {[string[], string]}
 */
function parsePrefix(s, ctx, lineNum, col) {
  const [first, n] = parseTag(s, ctx, lineNum, col);
  const [tags, m] = parseSuffix(s.slice(n), ctx, lineNum, col + n);
  tags.unshift(first);
  const rest = s.slice(n + m);
  if (rest === "" || rest.startsWith("  #")) return [tags, ""];
  if (rest.startsWith(" ") && !rest.startsWith("  ")) {
    return [tags, rest.slice(1)];
  }
  throw new Error(
    'Unexpected space after ")"' + locSuffix(ctx, lineNum, col + n + m),
  );
}

/**
 * Wrap a payload in tags, innermost first.
 * @param {unknown} value
 * @param {string[]} tags
 */
function wrapTags(value, tags) {
  return tags.reduce((v, tag) => tagged(tag, v), value);
}

/**
 * Wrap a payload in a prefix's tags, outermost first.
 * @param {unknown} value
 * @param {string[]} tags
 */
function wrapPrefix(value, tags) {
  return wrapTags(value, [...tags].reverse());
}

/**
 * Is rest, what follows a prefix, a block it may introduce on its own line?
 * @param {string} rest
 */
function prefixTakes(rest) {
  return rest === "" || rest.startsWith("`") || rest.startsWith(">");
}

/**
 * The tags at the start of a line, and the rest, or null if the line does
 * not begin with well-formed tags. For the scanner, which finds block
 * openers before the parser reports errors.
 * @param {string} s
 * @returns {string|null}
 */
function afterPrefix(s) {
  if (!s.startsWith("(")) return null;
  try {
    return parsePrefix(s, {}, 0, 0)[1];
  } catch {
    return null;
  }
}

/**
 * Whether a line is a property whose tags introduce a block string, as in
 * "key: (tag) `".
 * @param {string} line
 */
function isTaggedPropertyOpener(line) {
  const colon = findKeyColonOutsideQuotes(line);
  if (colon < 0 || !line.startsWith(" (", colon + 1)) return false;
  return afterPrefix(line.slice(colon + 2)) === "`";
}

/**
 * Whether what a property's prefix tagged is no block: nothing, or the empty
 * list a property reads when the next line is a sibling item.
 * @param {unknown} value
 */
function isMissingBlock(value) {
  return (
    value === undefined ||
    value === null ||
    (Array.isArray(value) && value.length === 0)
  );
}

/**
 * The error for a prefix that introduces no block.
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 */
function prefixWithoutBlock(ctx, lineNum, col) {
  return new Error(
    "A tag before a value needs a block below it; write the tag after an inline value" +
      locSuffix(ctx, lineNum, col),
  );
}

// --- References ---

/**
 * A reference: a URI or IRI, held as written; it designates, and is never
 * dereferenced. Instances are frozen. Two are equal when their texts are,
 * code unit for code unit, and a reference never equals a string or a plain
 * object.
 */
class Reference {
  /**
   * @param {string} uri
   */
  constructor(uri) {
    if (typeof uri !== "string") {
      throw new TypeError("A reference must be a string");
    }
    checkReferenceText(uri);
    this.reference = uri;
    Object.freeze(this);
  }
}

/**
 * Refuse a string that is not a reference's text, by the parser's own rule:
 * a lowercase scheme, a colon, URI or IRI characters, and well-formed
 * percent-encodings.
 * @param {string} uri
 */
function checkReferenceText(uri) {
  const text = "<" + uri + ">";
  let n = -1;
  let why = "it needs a scheme and a colon";
  if (isReferenceStart(text)) {
    try {
      [, n] = parseReference(text, {}, 0, 0);
    } catch (error) {
      why = /** @type {Error} */ (error).message;
    }
  }
  if (n !== text.length) {
    if (n >= 0) why = 'Unexpected character ">" in reference';
    throw new TypeError(`Not a reference: ${JSON.stringify(uri)} (${why})`);
  }
}

/**
 * A reference whose text the parser has already checked.
 * @param {string} uri
 * @returns {Reference}
 */
function checkedReference(uri) {
  const ref = Object.create(Reference.prototype);
  ref.reference = uri;
  return Object.freeze(ref);
}

/**
 * Make a reference, as the YAY parser does and as `yay -t js` writes it.
 * @param {string} uri
 * @returns {Reference}
 */
function reference(uri) {
  return new Reference(uri);
}

/**
 * @param {unknown} value
 * @returns {value is Reference}
 */
function isReference(value) {
  return value instanceof Reference;
}

/**
 * The length of the scheme, colon included, if s (the text after "<")
 * begins [A-Za-z][A-Za-z0-9+.-]*":". Uppercase letters are matched so that
 * the reader can name an uppercase scheme rather than call it bad hex.
 * @param {string} s
 * @returns {number|undefined}
 */
function schemeLength(s) {
  if (!SCHEME_START.test(s[0] ?? "")) return undefined;
  for (let i = 1; i < s.length; i++) {
    const c = s[i];
    if (c === ":") return i + 1;
    if (!SCHEME_CHARACTER.test(c)) return undefined;
  }
  return undefined;
}

/**
 * Whether s begins with "<" and a scheme, so is a reference.
 * @param {string} s
 */
function isReferenceStart(s) {
  return s.startsWith("<") && schemeLength(s.slice(1)) !== undefined;
}

/**
 * Whether the character c may appear in a reference (RFC 3986 and RFC
 * 3987), "%" aside, which must begin a percent-encoding.
 * @param {string} c
 */
function isUriChar(c) {
  const cp = c.codePointAt(0) ?? 0;
  if (cp < 0x80) return URI_ASCII.test(c);
  return cp >= 0xa0 && !UNICODE_SPACE.test(c) && cp !== 0xfffe && cp !== 0xffff;
}

/**
 * Parse a reference at the start of s, which begins with "<" and a scheme.
 * Returns the reference and the number of units consumed, the ">" included.
 * The text between the brackets is the URI, verbatim.
 * @param {string} s
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {[Reference, number]}
 */
function parseReference(s, ctx, lineNum, col) {
  const schemeEnd = 1 + /** @type {number} */ (schemeLength(s.slice(1))) - 1;
  const scheme = s.slice(1, schemeEnd);
  if (UPPERCASE.test(scheme)) {
    throw new Error(
      `Uppercase scheme "${scheme}" in reference (use lowercase)` +
        locSuffix(ctx, lineNum, col),
    );
  }
  let i = schemeEnd;
  while (i < s.length) {
    const c = String.fromCodePoint(/** @type {number} */ (s.codePointAt(i)));
    if (c === ">") return [checkedReference(s.slice(1, i)), i + 1];
    if (c === "%") {
      if (!HEX_PAIR.test(s.slice(i + 1))) {
        throw new Error(
          "Bad percent-encoding in reference (use % and two hex digits)" +
            locSuffix(ctx, lineNum, col + i),
        );
      }
    } else if (!isUriChar(c)) {
      const message =
        c === " "
          ? "Unexpected space in reference"
          : c === "\t"
            ? "Tab not allowed in reference"
            : `Unexpected character "${c}" in reference`;
      throw new Error(message + locSuffix(ctx, lineNum, col + i));
    }
    i += c.length;
  }
  throw new Error("Unmatched angle bracket" + locSuffix(ctx, lineNum, col));
}

// --- The CapData bridge ---

/**
 * Walk a YSON value, mapping each string that begins with "$" (a
 * reference, or a slot) through f. A tagged value's tag is a string, never
 * a reference, so only its payload is walked.
 * @param {unknown} value
 * @param {(s: string) => string} f
 * @returns {unknown}
 */
function mapDollars(value, f) {
  if (typeof value === "string") {
    return value.startsWith("$") ? f(value) : value;
  }
  if (Array.isArray(value)) return value.map((v) => mapDollars(v, f));
  if (value !== null && typeof value === "object") {
    const out = {};
    for (const [k, v] of Object.entries(value)) {
      setOwn(out, k, k === "#tag" && "payload" in value ? v : mapDollars(v, f));
    }
    return out;
  }
  return value;
}

/**
 * From YSON to CapData: each distinct reference goes to `slots` in order of
 * first appearance, and the body names it `$0`, `$1`, ... as smallcaps
 * names a remotable. `makeMarshal(convertValToSlot, convertSlotToVal)` then
 * reads it, and its convertSlotToVal decides what each URI becomes.
 * @param {string} yson
 * @returns {{body: string, slots: string[]}}
 */
function ysonToCapData(yson) {
  /** @type {string[]} */
  const slots = [];
  const index = new Map();
  const body = mapDollars(JSON.parse(yson), (s) => {
    const uri = s.slice(1);
    if (!isReferenceStart(`<${uri}`)) {
      throw new Error(`A reference needs a scheme: ${JSON.stringify(s)}`);
    }
    const text = `<${uri}>`;
    const [, consumed] = parseReference(text, {}, 0, 0);
    if (consumed !== text.length) {
      throw new Error(
        `Unexpected character ">" in reference: ${JSON.stringify(s)}`,
      );
    }
    let slot = index.get(uri);
    if (slot === undefined) {
      slot = slots.length;
      index.set(uri, slot);
      slots.push(uri);
    }
    return `$${slot}`;
  });
  return { body: `#${JSON.stringify(body)}`, slots };
}

/**
 * From CapData to YSON: the reverse of ysonToCapData. Each `$n` (with or
 * without smallcaps' interface, `$n.Alleged: x`, which YAY does not keep)
 * becomes `"$" + slots[n]`. A body with promises (`&n`) is refused.
 * @param {{body: string, slots: string[]}} capData
 * @returns {string}
 */
function capDataToYson({ body, slots }) {
  if (!body.startsWith("#")) {
    throw new Error("Expected a smallcaps body, beginning with #");
  }
  const value = JSON.parse(body.slice(1));
  const check = (v) => {
    if (typeof v === "string" && v.startsWith("&")) {
      throw new Error(`A promise has no YAY form: ${JSON.stringify(v)}`);
    }
    if (Array.isArray(v)) v.forEach(check);
    else if (v !== null && typeof v === "object") {
      Object.values(v).forEach(check);
    }
  };
  check(value);
  const yson = mapDollars(value, (s) => {
    const match = /^\$(0|[1-9][0-9]*)(\..*)?$/.exec(s);
    const uri = match ? slots[Number(match[1])] : undefined;
    if (typeof uri !== "string") {
      throw new Error(`No slot for ${JSON.stringify(s)}`);
    }
    // A slot is a reference's text, as YSON's "$" strings are
    checkReferenceText(uri);
    return `$${uri}`;
  });
  return JSON.stringify(yson);
}

export {
  parseYay,
  tagged,
  isTagged,
  reference,
  isReference,
  Reference,
  ysonToCapData,
  capDataToYson,
};
