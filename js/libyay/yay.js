"use strict";

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
 * Strip inline comments from a string.
 * Returns the value part (trimmed) without the comment.
 * @param {string} s
 * @returns {string}
 */
function stripInlineComment(s) {
  let inDouble = false;
  let inSingle = false;
  let escape = false;

  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (escape) {
      escape = false;
      continue;
    }
    if (c === "\\") {
      escape = true;
      continue;
    }
    if (c === '"' && !inSingle) {
      inDouble = !inDouble;
    } else if (c === "'" && !inDouble) {
      inSingle = !inSingle;
    } else if (c === "#" && !inDouble && !inSingle) {
      return s.slice(0, i).trimEnd();
    }
  }
  return s;
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
    lines.push({ line, indent, leader, lineNum: i });
  }
  return lines;
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
  let broken = false;
  for (const { line, indent, leader, lineNum } of lines) {
    // Close blocks on dedent.
    while (indent < top) {
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
      broken = false;
    } else if (leader.length > 0 && indent === top) {
      tokens.push({ type: "stop", text: "" });
      tokens.push({
        type: "start",
        text: leader,
        indent,
        lineNum,
        col: indent,
      });
      broken = false;
    }
    if (line.length > 0) {
      tokens.push({ type: "text", text: line, indent, lineNum, col: indent });
      broken = false;
    } else if (!broken) {
      tokens.push({ type: "break", text: "", lineNum, col: indent });
      broken = true;
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
 * @param {Token[]} tokens
 * @param {ParseContext} ctx
 * @returns {unknown}
 */
function parseRoot(tokens, ctx = {}) {
  let i = 0;
  while (
    i < tokens.length &&
    (tokens[i].type === "stop" || tokens[i].type === "break")
  )
    i++;
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
  if (
    t.type === "text" &&
    findKeyColonOutsideQuotes(t.text) >= 0 &&
    (t.indent ?? 0) === 0 &&
    !t.text.startsWith("{")
  ) {
    const [value, next] = parseRootObject(tokens, i, ctx);
    return ensureAtEnd(value, tokens, next, ctx);
  }
  const [value, next] = parseValue(tokens, i, ctx);
  return ensureAtEnd(value, tokens, next, ctx);
}

function ensureAtEnd(value, tokens, i, ctx = {}) {
  let j = i;
  while (
    j < tokens.length &&
    (tokens[j].type === "stop" || tokens[j].type === "break")
  )
    j++;
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
    const raw = t.text;
    const s = raw;
    const sCol = t.col ?? 0;
    if (s === "null") return [null, i + 1];
    if (s === "true") return [true, i + 1];
    if (s === "false") return [false, i + 1];
    if (s === "nan") return [NaN, i + 1];
    if (s === "infinity") return [Infinity, i + 1];
    if (s === "-infinity") return [-Infinity, i + 1];
    const num = parseNumber(s, ctx, t.lineNum ?? 0, sCol);
    if (num !== undefined) return [num, i + 1];
    if (s === "`" || (s.startsWith("`") && s.length >= 2 && s[1] === " ")) {
      const firstLine = s.length > 2 ? s.slice(2) : "";
      // Use token's indent as base - block string content must be indented more
      return parseBlockStringWithIndent(
        tokens,
        i,
        firstLine,
        false,
        t.indent ?? 0,
      );
    }
    if (
      (s.startsWith('"') && s.length > 1) ||
      (s.startsWith("'") && s.length > 1)
    ) {
      return [parseQuotedString(s, ctx, t.lineNum ?? 0, sCol), i + 1];
    }
    if (s.startsWith("[")) {
      // Inline arrays must close on the same line.
      if (!s.endsWith("]")) {
        throw new Error(
          "Unexpected newline in inline array" +
            locSuffix(ctx, t.lineNum ?? 0, sCol),
        );
      }
      validateInlineArrayWhitespace(s, ctx, t.lineNum ?? 0, sCol);
      return [parseInlineArray(s, ctx, t.lineNum ?? 0, sCol), i + 1];
    }
    if (s.startsWith(">")) {
      let firstLine = s.slice(1);
      if (firstLine.startsWith(" ")) firstLine = firstLine.slice(1);
      if (firstLine.length === 0) {
        throw new Error("Expected hex or comment in hex block");
      }
      return parseBlockBytes(tokens, i, ctx, firstLine, t.indent ?? 0);
    }
    if (s.startsWith("{")) {
      // Inline objects must close on the same line.
      if (!s.includes("}")) {
        throw new Error(
          "Unexpected newline in inline object" +
            locSuffix(ctx, t.lineNum ?? 0, sCol),
        );
      }
      const inlineObj = parseInlineObject(s, ctx, t.lineNum ?? 0, sCol);
      if (inlineObj !== null) return [inlineObj, i + 1];
    }
    if (s.startsWith("<") && s.includes(">"))
      return [parseAngleBytes(s, ctx, t.lineNum, t.col), i + 1];
    if (s.startsWith("<")) {
      throw new Error(
        "Unmatched angle bracket" + locSuffix(ctx, t.lineNum ?? 0, sCol),
      );
    }
    const keyValue = splitKeyValue(s, sCol, ctx, t.lineNum ?? 0);
    if (keyValue) {
      const { key, valuePart, valueCol } = keyValue;
      if (valuePart === "" && key.length > 0) {
        return parseObjectOrNamedArray(tokens, i, key, ctx);
      }
      // Note: "key: <" without closing ">" is invalid - inline byte arrays must be closed on the same line
      if (key.length > 0) {
        const value =
          valuePart === ""
            ? undefined
            : parseScalar(valuePart, ctx, t.lineNum ?? 0, valueCol);
        return [{ [key]: value }, i + 1];
      }
    }
    return [parseScalar(s, ctx, t.lineNum ?? 0, sCol), i + 1];
  }
  return [undefined, i + 1];
}

/**
 * @param {string} s
 * @param {number} sCol
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number|undefined} inlineCol - column of opening brace for inline objects
 * @returns {{key: string, valuePart: string, valueCol: number}|null}
 */
function splitKeyValue(s, sCol, ctx, lineNum, inlineCol) {
  const colonIdx = findKeyColonOutsideQuotes(s);
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
    key =
      quote === '"'
        ? parseQuotedString(keyRaw, ctx, lineNum, sCol)
        : keyRaw.slice(1, -1);
  } else {
    if (keyRaw.length === 0) {
      throw new Error("Missing key" + locSuffix(ctx, lineNum, sCol + colonIdx));
    }
    for (let i = 0; i < keyRaw.length; i++) {
      const c = keyRaw[i];
      const isAlpha = (c >= "a" && c <= "z") || (c >= "A" && c <= "Z");
      const isDigit = c >= "0" && c <= "9";
      const isUnderscore = c === "_";
      const isHyphen = c === "-";
      if (!isAlpha && !isDigit && !isUnderscore && !isHyphen) {
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
 * @returns {number}
 */
function findKeyColonOutsideQuotes(s) {
  let inSingle = false;
  let inDouble = false;
  let escape = false;
  for (let i = 0; i < s.length; i++) {
    const ch = s[i];
    if (escape) {
      escape = false;
      continue;
    }
    if (inSingle) {
      if (ch === "\\") {
        escape = true;
      } else if (ch === "'") {
        inSingle = false;
      }
      continue;
    }
    if (inDouble) {
      if (ch === "\\") {
        escape = true;
      } else if (ch === '"') {
        inDouble = false;
      }
      continue;
    }
    if (ch === "'") {
      inSingle = true;
      continue;
    }
    if (ch === '"') {
      inDouble = true;
      continue;
    }
    if (ch === ":") return i;
  }
  return -1;
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
 * @param {string} s
 * @param {ParseContext} ctx
 * @param {number} lineNum
 * @param {number} col
 * @returns {Record<string, unknown>|null}
 */
function parseInlineObject(s, ctx = {}, lineNum = 0, col = 0) {
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
  const parts = [];
  let start = 0;
  let inSingle = false;
  let inDouble = false;
  let escape = false;
  let braceDepth = 0;
  let bracketDepth = 0;
  for (let i = 0; i < body.length; i++) {
    const ch = body[i];
    if (escape) {
      escape = false;
      continue;
    }
    if (inSingle) {
      if (ch === "\\") {
        escape = true;
      } else if (ch === "'") {
        inSingle = false;
      }
      continue;
    }
    if (inDouble) {
      if (ch === "\\") {
        escape = true;
      } else if (ch === '"') {
        inDouble = false;
      }
      continue;
    }
    if (ch === "'") {
      inSingle = true;
      continue;
    }
    if (ch === '"') {
      inDouble = true;
      continue;
    }
    if (ch === "{") {
      braceDepth++;
      continue;
    }
    if (ch === "}") {
      if (braceDepth > 0) braceDepth--;
      continue;
    }
    if (ch === "[") {
      bracketDepth++;
      continue;
    }
    if (ch === "]") {
      if (bracketDepth > 0) bracketDepth--;
      continue;
    }
    if (ch === "," && braceDepth === 0 && bracketDepth === 0) {
      if (i > 0 && body[i - 1] === " ") {
        throw new Error(
          'Unexpected space before ","' +
            locSuffix(ctx, lineNum, col + 1 + i - 1),
        );
      }
      if (i + 1 >= body.length || body[i + 1] !== " ") {
        throw new Error(
          'Expected space after ","' + locSuffix(ctx, lineNum, col + 1 + i),
        );
      }
      parts.push({ text: body.slice(start, i), start });
      start = i + 2;
    }
  }
  parts.push({ text: body.slice(start), start });
  const obj = {};
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
    if (valuePart === "") {
      throw new Error("Missing value" + locSuffix(ctx, lineNum, valueCol));
    }
    obj[key] = parseScalar(valuePart, ctx, lineNum, valueCol);
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
function parseScalar(s, ctx = {}, lineNum = 0, col = 0) {
  // Strip inline comments first
  s = stripInlineComment(s);

  if (s === "null") return null;
  if (s === "true") return true;
  if (s === "false") return false;
  if (s === "nan") return NaN;
  if (s === "infinity") return Infinity;
  if (s === "-infinity") return -Infinity;
  const num = parseNumber(s, ctx, lineNum, col);
  if (num !== undefined) return num;
  if (s.startsWith('"')) {
    if (!s.endsWith('"') || s.length < 2) {
      throw new Error(
        "Unterminated string" + locSuffix(ctx, lineNum, col + s.length),
      );
    }
    return parseQuotedString(s, ctx, lineNum, col);
  }
  if (s.startsWith("'")) {
    if (!s.endsWith("'") || s.length < 2) {
      throw new Error(
        "Unterminated string" + locSuffix(ctx, lineNum, col + s.length),
      );
    }
    return s.slice(1, -1);
  }
  if (s.startsWith("[")) return parseInlineArray(s, ctx, lineNum, col);
  if (s.startsWith("{")) return parseInlineObject(s, ctx, lineNum, col);
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
  if (s.startsWith("'")) return s.slice(1, -1);
  return s;
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
      // Escapes follow JSON rules; report the offending escape character.
      if (i + 1 >= s.length - 1) {
        throw new Error(
          "Bad escaped character" + locSuffix(ctx, lineNum, col + i + 1),
        );
      }
      const esc = s[i + 1];
      switch (esc) {
        case '"':
          out += '"';
          i++;
          break;
        case "\\":
          out += "\\";
          i++;
          break;
        case "/":
          out += "/";
          i++;
          break;
        case "b":
          out += "\b";
          i++;
          break;
        case "f":
          out += "\f";
          i++;
          break;
        case "n":
          out += "\n";
          i++;
          break;
        case "r":
          out += "\r";
          i++;
          break;
        case "t":
          out += "\t";
          i++;
          break;
        case "u": {
          // YAY uses \u{XXXXXX} syntax (variable-length with braces)
          const braceStart = i + 2;
          const uCol = col + i + 1; // Column of 'u' for "Bad escaped character"
          const braceCol = col + braceStart; // Column of '{' for other errors
          if (braceStart >= s.length - 1 || s[braceStart] !== "{") {
            // Old-style \uXXXX syntax is not supported
            throw new Error(
              "Bad escaped character" + locSuffix(ctx, lineNum, uCol),
            );
          }
          // Find closing brace
          let braceEnd = braceStart + 1;
          while (braceEnd < s.length - 1 && s[braceEnd] !== "}") {
            braceEnd++;
          }
          if (braceEnd >= s.length - 1 || s[braceEnd] !== "}") {
            throw new Error(
              "Bad Unicode escape" + locSuffix(ctx, lineNum, braceCol),
            );
          }
          const hexStart = braceStart + 1;
          if (hexStart === braceEnd) {
            throw new Error(
              "Bad Unicode escape" + locSuffix(ctx, lineNum, braceCol),
            );
          }
          // Check for too many hex digits (max 6)
          if (braceEnd - hexStart > 6) {
            throw new Error(
              "Bad Unicode escape" + locSuffix(ctx, lineNum, braceCol),
            );
          }
          let hex = "";
          for (let j = hexStart; j < braceEnd; j++) {
            const c = s[j];
            if (!/[0-9a-fA-F]/.test(c)) {
              throw new Error(
                "Bad Unicode escape" + locSuffix(ctx, lineNum, braceCol),
              );
            }
            hex += c;
          }
          const code = parseInt(hex, 16);
          if (code >= 0xd800 && code <= 0xdfff) {
            throw new Error(
              "Illegal surrogate" + locSuffix(ctx, lineNum, braceCol),
            );
          }
          if (code > 0x10ffff) {
            throw new Error(
              "Unicode code point out of range" +
                locSuffix(ctx, lineNum, braceCol),
            );
          }
          out += String.fromCodePoint(code);
          i = braceEnd; // Loop will increment to braceEnd + 1
          break;
        }
        default:
          throw new Error(
            "Bad escaped character" + locSuffix(ctx, lineNum, col + i + 1),
          );
      }
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
  const parts = [];
  let start = 0;
  let inSingle = false;
  let inDouble = false;
  let escape = false;
  let braceDepth = 0;
  let bracketDepth = 0;
  let angleDepth = 0;
  for (let i = 0; i < body.length; i++) {
    const ch = body[i];
    if (escape) {
      escape = false;
      continue;
    }
    if (inSingle) {
      if (ch === "\\") {
        escape = true;
      } else if (ch === "'") {
        inSingle = false;
      }
      continue;
    }
    if (inDouble) {
      if (ch === "\\") {
        escape = true;
      } else if (ch === '"') {
        inDouble = false;
      }
      continue;
    }
    if (ch === "'") {
      inSingle = true;
      continue;
    }
    if (ch === '"') {
      inDouble = true;
      continue;
    }
    if (ch === "{") {
      braceDepth++;
      continue;
    }
    if (ch === "}") {
      if (braceDepth > 0) braceDepth--;
      continue;
    }
    if (ch === "[") {
      bracketDepth++;
      continue;
    }
    if (ch === "]") {
      if (bracketDepth > 0) bracketDepth--;
      continue;
    }
    if (ch === "<") {
      angleDepth++;
      continue;
    }
    if (ch === ">") {
      if (angleDepth > 0) angleDepth--;
      continue;
    }
    if (
      ch === "," &&
      braceDepth === 0 &&
      bracketDepth === 0 &&
      angleDepth === 0
    ) {
      if (i > 0 && body[i - 1] === " ") {
        throw new Error(
          'Unexpected space before ","' +
            locSuffix(ctx, lineNum, col + 1 + i - 1),
        );
      }
      if (i + 1 >= body.length || body[i + 1] !== " ") {
        throw new Error(
          'Expected space after ","' + locSuffix(ctx, lineNum, col + 1 + i),
        );
      }
      parts.push({ text: body.slice(start, i), start });
      start = i + 2;
    }
  }
  parts.push({ text: body.slice(start), start });
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
 */
function validateInlineArrayWhitespace(s, ctx = {}, lineNum = 0, col = 0) {
  let inSingle = false;
  let inDouble = false;
  let escape = false;
  let depth = 0;
  for (let i = 0; i < s.length; i++) {
    const ch = s[i];
    if (escape) {
      escape = false;
      continue;
    }
    if (inSingle) {
      if (ch === "\\") {
        escape = true;
      } else if (ch === "'") {
        inSingle = false;
      }
      continue;
    }
    if (inDouble) {
      if (ch === "\\") {
        escape = true;
      } else if (ch === '"') {
        inDouble = false;
      }
      continue;
    }
    if (ch === "'") {
      inSingle = true;
      continue;
    }
    if (ch === '"') {
      inDouble = true;
      continue;
    }
    if (ch === "[") {
      depth++;
      if (i + 1 < s.length && s[i + 1] === " ") {
        throw new Error(
          'Unexpected space after "["' + locSuffix(ctx, lineNum, col + i + 1),
        );
      }
      continue;
    }
    if (ch === "]") {
      if (i > 0 && s[i - 1] === " ") {
        throw new Error(
          'Unexpected space before "]"' + locSuffix(ctx, lineNum, col + i - 1),
        );
      }
      if (depth > 0) depth--;
      continue;
    }
    if (ch === ",") {
      if (i > 0 && s[i - 1] === " ") {
        throw new Error(
          'Unexpected space before ","' + locSuffix(ctx, lineNum, col + i - 1),
        );
      }
      if (i + 1 < s.length && s[i + 1] !== " " && s[i + 1] !== "]") {
        let lookaheadDepth = depth;
        let inS = false;
        let inD = false;
        let esc = false;
        let nextIsClosingWithSpace = false;
        for (let j = i + 1; j < s.length; j++) {
          const cj = s[j];
          if (esc) {
            esc = false;
            continue;
          }
          if (inS) {
            if (cj === "\\") esc = true;
            else if (cj === "'") inS = false;
            continue;
          }
          if (inD) {
            if (cj === "\\") esc = true;
            else if (cj === '"') inD = false;
            continue;
          }
          if (cj === "'") {
            inS = true;
            continue;
          }
          if (cj === '"') {
            inD = true;
            continue;
          }
          if (cj === "[") {
            lookaheadDepth++;
            continue;
          }
          if (cj === "]") {
            if (lookaheadDepth === depth) {
              nextIsClosingWithSpace = j > 0 && s[j - 1] === " ";
              break;
            }
            if (lookaheadDepth > 0) lookaheadDepth--;
            continue;
          }
          if (cj === "," && lookaheadDepth === depth) {
            break;
          }
        }
        if (!nextIsClosingWithSpace) {
          throw new Error(
            'Expected space after ","' + locSuffix(ctx, lineNum, col + i),
          );
        }
      }
      if (i + 2 < s.length && s[i + 1] === " " && s[i + 2] === " ") {
        throw new Error(
          'Unexpected space after ","' + locSuffix(ctx, lineNum, col + i + 2),
        );
      }
    }
  }
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
  const hex = inner.replace(/\s/g, "");
  if (hex.length % 2 !== 0)
    throw new Error(
      "Odd number of hex digits in byte literal" + locSuffix(ctx, lineNum, col),
    );
  if (!/^[0-9a-f]*$/.test(hex))
    throw new Error("Invalid hex digit" + locSuffix(ctx, lineNum, col));
  return Uint8Array.fromHex
    ? Uint8Array.fromHex(hex)
    : hexToUint8Array(hex, ctx, lineNum, col);
}

function hexToUint8Array(hex, ctx, lineNum, col) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let j = 0; j < bytes.length; j++) {
    const pair = hex.slice(j * 2, j * 2 + 2);
    const val = parseInt(pair, 16);
    if (isNaN(val)) {
      throw new Error("Invalid hex digit" + locSuffix(ctx, lineNum, col));
    }
    bytes[j] = val;
  }
  return bytes;
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
    i++;
    while (i < tokens.length && tokens[i].type === "break") i++;
    if (i >= tokens.length) break;
    const next = tokens[i];
    if (
      next.type === "text" &&
      next.text === "" &&
      i + 1 < tokens.length &&
      tokens[i + 1].type === "start" &&
      tokens[i + 1].text === "-"
    ) {
      const [value, j] = parseListArray(tokens, i + 1, ctx);
      arr.push(value);
      i = j;
    } else if (next.type === "start" && next.text === "-") {
      const [value, j] = parseListArray(tokens, i, ctx);
      arr.push(value);
      i = j;
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
        while (j < tokens.length && tokens[j].type === "break") j++;
        if (j >= tokens.length) break;
        const [subVal, nextJ] = parseValue(tokens, j, ctx);
        group.push(subVal);
        j = nextJ;
        while (j < tokens.length && tokens[j].type === "stop") j++;
      }
      arr.push(group);
      i = j;
    } else if (
      next.type === "text" &&
      findKeyColonOutsideQuotes(next.text) >= 0
    ) {
      const nextIndent = next.indent ?? 0;
      const inlineIndent = nextIndent === listIndent ? nextIndent : undefined;
      const baseIndent =
        inlineIndent !== undefined ? nextIndent + 2 : nextIndent;
      const [obj, nextIndex] = parseObjectBlock(
        tokens,
        i,
        baseIndent,
        ctx,
        inlineIndent,
      );
      arr.push(obj);
      i = nextIndex;
    } else if (next.type === "text" || next.type === "start") {
      const [value, j] = parseValue(tokens, i, ctx);
      let k = j;
      while (k < tokens.length && tokens[k].type === "break") k++;
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
          i++;
          while (i < tokens.length && tokens[i].type === "break") i++;
          if (i >= tokens.length) break;
          const [subVal, nextI] = parseValue(tokens, i, ctx);
          group.push(subVal);
          i = nextI;
          while (i < tokens.length && tokens[i].type === "stop") i++;
        }
        arr.push(group);
      } else {
        arr.push(value);
        i = j;
      }
    } else {
      i++;
    }
    // Skip stops and breaks between items
    while (
      i < tokens.length &&
      (tokens[i].type === "stop" || tokens[i].type === "break")
    )
      i++;
  }
  return [arr, i];
}

/**
 * Parse a block string with an optional base indent constraint.
 * @param {Token[]} tokens
 * @param {number} i
 * @param {string|undefined} firstLine
 * @param {boolean} inPropertyContext
 * @param {number} baseIndent - If >= 0, stop collecting when indent <= baseIndent
 * @returns {[string, number]}
 */
function parseBlockStringWithIndent(
  tokens,
  i,
  firstLine,
  inPropertyContext,
  baseIndent,
) {
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
    throw new Error(
      'Empty block string not allowed (use "" or "\\n" explicitly)',
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
function parseBlockBytes(tokens, i, ctx = {}, firstLineRaw, baseIndent) {
  const startToken = tokens[i];
  const lineNum = startToken.lineNum ?? 0;
  const col = startToken.col ?? 0;
  let hex = "";
  if (firstLineRaw !== undefined) {
    hex += firstLineRaw.replace(/#.*$/, "").replace(/\s/g, "").toLowerCase();
    i++;
  } else {
    i++;
  }
  while (i < tokens.length) {
    const t = tokens[i];
    if (t.type === "break") {
      i++;
      continue;
    }
    if (t.type === "text" && (t.indent ?? 0) > baseIndent) {
      hex += t.text.replace(/#.*$/, "").replace(/\s/g, "").toLowerCase();
      i++;
      continue;
    }
    break;
  }
  if (hex.length % 2 !== 0)
    throw new Error(
      "Odd number of hex digits in byte literal" + locSuffix(ctx, lineNum, col),
    );
  if (!/^[0-9a-f]*$/.test(hex))
    throw new Error("Invalid hex digit" + locSuffix(ctx, lineNum, col));
  const result = Uint8Array.fromHex
    ? Uint8Array.fromHex(hex)
    : hexToUint8Array(hex, ctx, lineNum, col);
  return [result, i];
}

// Note: parseMultilineAngleBytes was removed as dead code.
// The "< hex" syntax (without closing ">") is invalid - inline byte arrays must be closed on the same line.

/**
 * @param {Token[]} tokens
 * @param {number} i
 * @param {string} key
 * @param {ParseContext} ctx
 * @returns {[Record<string, unknown>, number]}
 */
function parseObjectOrNamedArray(tokens, i, key, ctx = {}) {
  const keyToken = tokens[i];
  const keyValue = splitKeyValue(
    keyToken.text,
    keyToken.col ?? 0,
    ctx,
    keyToken.lineNum ?? 0,
  );
  i++;
  while (
    i < tokens.length &&
    (tokens[i].type === "break" || tokens[i].type === "stop")
  )
    i++;
  const baseIndent = i < tokens.length ? (tokens[i].indent ?? 0) : 0;
  const first = i < tokens.length ? tokens[i] : null;
  // Check for empty property with no nested content
  if (
    !first ||
    (first.type === "text" && (first.indent ?? 0) <= (keyToken.indent ?? 0))
  ) {
    // Check if the next token is a sibling property (same indent) or parent (lower indent)
    // If so, this property has no value which is invalid
    if (
      !first ||
      (first.type === "text" &&
        splitKeyValue(first.text, first.col ?? 0, ctx, first.lineNum ?? 0))
    ) {
      const col = (keyToken.col ?? 0) + key.length + 1;
      throw new Error(
        "Expected value after property" +
          locSuffix(ctx, keyToken.lineNum ?? 0, col),
      );
    }
  }
  if (first && first.type === "start" && first.text === "-") {
    const [arr] = parseListArray(tokens, i);
    return [{ [key]: arr }, skipToNextKey(tokens, i, baseIndent)];
  }
  // Note: '*' syntax for multiline bytes is rejected by the scanner
  // Note: "key: <" without closing ">" is invalid - inline byte arrays must be closed on the same line
  if (first && first.type === "text" && first.text === '"') {
    const [body, next] = parseBlockStringWithIndent(
      tokens,
      i,
      undefined,
      false,
      -1,
    );
    return [{ [key]: body }, next];
  }
  // Reject block string leader on separate line - must be on same line as key
  if (first && first.type === "text" && first.text.trim() === "`") {
    throw new Error(
      "Unexpected indent" + locSuffix(ctx, first.lineNum ?? 0, 0),
    );
  }
  // Concatenated quoted strings (multiple quoted strings on consecutive lines)
  if (first && first.type === "text") {
    const trimmed = first.text.trim();
    if (
      (trimmed.startsWith('"') &&
        trimmed.endsWith('"') &&
        trimmed.length >= 2) ||
      (trimmed.startsWith("'") && trimmed.endsWith("'") && trimmed.length >= 2)
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
        const vPart = keyValue.valuePart;
        if ((t.indent ?? 0) < baseIndent) break;
        if (k && (t.indent ?? 0) <= baseIndent && obj.hasOwnProperty(k)) break;
        if (k && (t.indent ?? 0) <= baseIndent && !obj.hasOwnProperty(k)) {
          if (vPart === "{}") {
            obj[k] = {};
            i++;
          } else if (vPart.startsWith(">")) {
            // Block bytes in property context
            if (!isPropertyBlockLeaderOnly(vPart, ">")) {
              throw new Error(
                "Expected newline after block leader in property",
              );
            }
            const [bytes, next] = parseBlockBytes(
              tokens,
              i,
              ctx,
              "",
              t.indent ?? 0,
            );
            obj[k] = bytes;
            i = next;
          } else if (vPart.trim() === "`") {
            // Block string in property context: backtick alone on line
            const [body, next] = parseBlockStringWithIndent(
              tokens,
              i,
              "",
              true,
              t.indent ?? 0,
            );
            obj[k] = body;
            i = next;
          } else if (vPart === "") {
            i++;
            while (i < tokens.length && tokens[i].type === "break") i++;
            const nextT = tokens[i];
            // Note: '*' syntax for multiline bytes is rejected by the scanner
            if (nextT && nextT.type === "text" && nextT.text === '"') {
              const [body, next] = parseBlockStringWithIndent(
                tokens,
                i,
                undefined,
                false,
                -1,
              );
              obj[k] = body;
              i = next;
            } else if (nextT && nextT.type === "start" && nextT.text === "-") {
              const [arr, next] = parseListArray(tokens, i);
              obj[k] = arr;
              i = next;
            } else if (
              nextT &&
              nextT.type === "text" &&
              (nextT.indent ?? 0) > (t.indent ?? 0)
            ) {
              const [child, next] = parseObjectBlock(
                tokens,
                i,
                nextT.indent ?? 0,
                ctx,
              );
              obj[k] = child;
              i = next;
            } else {
              // Empty property with no nested content is invalid
              throw new Error(
                "Expected value after property" +
                  locSuffix(ctx, t.lineNum ?? 0, (t.col ?? 0) + k.length + 1),
              );
            }
          } else {
            // Inline value (scalar, array, object, bytes)
            obj[k] = parseScalar(vPart, ctx, t.lineNum ?? 0, keyValue.valueCol);
            i++;
          }
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

function skipToNextKey(tokens, i, baseIndent) {
  while (
    i < tokens.length &&
    tokens[i].type !== "stop" &&
    (tokens[i].indent ?? 0) > baseIndent
  )
    i++;
  while (i < tokens.length && tokens[i].type === "stop") i++;
  return i;
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
    const vPart = keyValue.valuePart;
    // Note: "key: <" without closing ">" is invalid - inline byte arrays must be closed on the same line
    if (vPart.startsWith(">")) {
      if (!isPropertyBlockLeaderOnly(vPart, ">")) {
        throw new Error("Expected newline after block leader in property");
      }
      const [bytes, j] = parseBlockBytes(tokens, i, ctx, "", baseIndent);
      obj[k] = bytes;
      i = j;
      continue;
    }
    if (vPart === "{}") {
      obj[k] = {};
      i++;
      continue;
    }
    // Block string in property context: backtick alone on line
    if (vPart.trim() === "`") {
      const [body, next] = parseBlockStringWithIndent(
        tokens,
        i,
        "",
        true,
        t.indent ?? 0,
      );
      obj[k] = body;
      i = next;
      continue;
    }
    if (vPart === "") {
      i++;
      while (
        i < tokens.length &&
        (tokens[i].type === "break" || tokens[i].type === "stop")
      )
        i++;
      const nextT = tokens[i];
      if (nextT && nextT.type === "start" && nextT.text === "-") {
        // Pass baseIndent as minIndent so nested arrays stop at the object's level
        const [arr, next] = parseListArray(tokens, i, ctx, baseIndent);
        obj[k] = arr;
        i = next;
        continue;
      }
      // Note: "key: <" without closing ">" is invalid - inline byte arrays must be closed on the same line
      // Note: A line with just '"' is not valid - block strings use backtick (`)
      if (nextT && nextT.type === "text" && (nextT.indent ?? 0) > baseIndent) {
        const [child, next] = parseObjectBlock(
          tokens,
          i,
          nextT.indent ?? 0,
          ctx,
        );
        obj[k] = child;
        i = next;
        continue;
      }
      // Empty property with no nested content is handled by parseObjectOrNamedArray
      // which throws an error before we reach here
      continue;
    }
    // Inline value (scalar, array, object, bytes)
    obj[k] = parseScalar(vPart, ctx, t.lineNum ?? 0, keyValue.valueCol);
    i++;
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
        const vPart = keyValue.valuePart;
        // Note: "key: <" without closing ">" is invalid - inline byte arrays must be closed on the same line
        if (vPart.startsWith(">")) {
          if (!isPropertyBlockLeaderOnly(vPart, ">")) {
            throw new Error("Expected newline after block leader in property");
          }
          const [bytes, j] = parseBlockBytes(tokens, i, ctx, "", baseIndent);
          obj[k] = bytes;
          i = j;
        } else if (vPart === "{}") {
          obj[k] = {};
          i++;
          // Note: "key: \"" is an unterminated string error, not a block string
          // Block strings use backtick (`) not double-quote (")
        } else if (vPart.startsWith("`")) {
          if (!isPropertyBlockLeaderOnly(vPart, "`")) {
            throw new Error("Expected newline after block leader in property");
          }
          i++;
          while (
            i < tokens.length &&
            (tokens[i].type === "break" || tokens[i].type === "stop")
          )
            i++;
          const nextT = tokens[i];
          if (nextT && nextT.type === "text" && nextT.text === "`") {
            // Empty block string (just opening and closing backticks) is not allowed
            throw new Error(
              'Empty block string not allowed (use "" or "\\n" explicitly)',
            );
          } else {
            const withIndent = [];
            while (
              i < tokens.length &&
              ((tokens[i].type === "text" &&
                (tokens[i].indent ?? 0) > baseIndent) ||
                tokens[i].type === "break")
            ) {
              if (tokens[i].type === "break") {
                withIndent.push({ indent: undefined, text: "" });
                i++;
              } else {
                withIndent.push({
                  indent: tokens[i].indent ?? 0,
                  text: tokens[i].text,
                });
                i++;
              }
            }
            const minIndent = withIndent
              .filter((x) => x.indent !== undefined)
              .reduce((min, x) => (x.indent < min ? x.indent : min), Infinity);
            const effectiveMin = minIndent === Infinity ? 0 : minIndent;
            const bodyLines = withIndent.map(({ indent, text }) =>
              indent === undefined
                ? ""
                : (indent - effectiveMin > 0
                    ? " ".repeat(indent - effectiveMin)
                    : "") + text,
            );
            let endLine = bodyLines.length;
            while (endLine > 0 && bodyLines[endLine - 1] === "") endLine--;
            const trimmedLines = bodyLines.slice(0, endLine);
            obj[k] =
              trimmedLines.join("\n") + (trimmedLines.length > 0 ? "\n" : "");
          }
        } else if (vPart === "") {
          const [valueObj, next] = parseObjectOrNamedArray(tokens, i, k, ctx);
          obj[k] = valueObj[k];
          i = next;
        } else {
          // Inline value (scalar, array, object, bytes)
          obj[k] = parseScalar(vPart, ctx, t.lineNum ?? 0, keyValue.valueCol);
          i++;
        }
      } else {
        i++;
      }
    } else {
      i++;
    }
  }
  return [obj, i];
}

export { parseYay };
