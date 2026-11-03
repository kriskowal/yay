#!/usr/bin/env python3
"""
Throwaway prototype of option G (doc/design/tagged-values.md), not a port.

It rewrites option G's tagged values into YAY 2.0.0 text that spells each one
as the reserved object {"#tag": tag, payload: value}, then hands the result to
the unmodified Python port. A document with no G syntax comes back byte for
byte, which is how `check.py` shows that G changes no existing document.
`desugar(src, meh=True)` reads MEH's leniency: any run of spaces, or none,
between a value and its tag and between tags.
"""

import json
import re
import sys

KEY = re.compile(r'([A-Za-z0-9_-]+|"(?:[^"\\]|\\.)*"|\'(?:[^\'\\]|\\.)*\'):(?= |$)')
# What may follow a value and its tags inline: the end, a space, a separator.
AFTER = " ,]}"


class GSyntaxError(Exception):
    pass


def quoted(s, i):
    q = s[i]
    j = i + 1
    while j < len(s):
        if s[j] == "\\":
            j += 2
            continue
        if s[j] == q:
            return j + 1
        j += 1
    raise GSyntaxError("unterminated string")


def unquote(text):
    if text[0] == "'":
        return re.sub(r"\\(['\\])", r"\1", text[1:-1])
    try:
        return json.loads(re.sub(r"\\u\{([0-9a-fA-F]+)\}", lambda m: "\\u%04x" % int(m[1], 16), text))
    except ValueError as e:
        raise GSyntaxError(f"bad string {text}: {e}")


ESCAPES = {")": ")", "\\": "\\", "/": "/", "b": "\b", "f": "\f", "n": "\n", "r": "\r", "t": "\t"}


def paren_end(s, i):
    """The end of a parenthesized tag at s[i] == "(", past its ")"."""
    j = i + 1
    while j < len(s):
        if s[j] == "\\":
            j += 2
            continue
        if s[j] == ")":
            return j + 1
        j += 1
    raise GSyntaxError("unclosed (")


def paren_tag(s, i):
    """A tag at s[i] == "(", read as a double-quoted string whose delimiters
    are ( and ): the same escapes, with \\) in place of \\". Returns
    (tag, end) or raises."""
    j = i + 1
    out = ""
    while j < len(s):
        c = s[j]
        if c == ")":
            return out, j + 1
        if c == "\t":
            raise GSyntaxError("tab not allowed in a tag, as in a string; write \\t")
        if c == "\\":
            e = s[j + 1:j + 2]
            if e in ESCAPES:
                out += ESCAPES[e]
                j += 2
                continue
            m = re.match(r"u\{([0-9a-fA-F]{1,6})\}", s[j + 1:])
            if m:
                cp = int(m[1], 16)
                if 0xD800 <= cp <= 0xDFFF or cp > 0x10FFFF:
                    raise GSyntaxError("not a Unicode scalar value")
                out += chr(cp)
                j += 1 + m.end()
                continue
            raise GSyntaxError(f"unknown escape \\{e} in a tag")
        out += c
        j += 1
    raise GSyntaxError("unclosed (")


def wrap(tag, text):
    return '{"#tag": %s, payload: %s}' % (json.dumps(tag, ensure_ascii=False), text)


NUM_BLOCK = re.compile(r"-?(?:\d(?: ?\d)*)?(?:\.(?:\d(?: ?\d)*)?)?(?:e[+-]?\d(?: ?\d)*)?")
NUM_INLINE = re.compile(r"-?\d*(?:\.\d*)?(?:e[+-]?\d+)?")
KEYWORDS = ("-infinity", "null", "true", "false", "nan", "infinity")


def atom(s, i, block, meh=False):
    """One inline value at s[i]; (text, end, kind) or None."""
    c = s[i] if i < len(s) else ""
    if c in "\"'":
        return s[i:quoted(s, i)], quoted(s, i), "string"
    if c == "<":
        j = s.find(">", i) + 1
        return (s[i:j], j, "bytes") if j else None
    if c in "[{":
        close = "]" if c == "[" else "}"
        out, j = c, i + 1
        while j < len(s) and s[j] != close:
            if c == "{":
                m = KEY.match(s, j)
                if not m:
                    return None
                out += s[j:m.end() + 1]
                j = m.end() + 1
            r = tagged(s, j, False, meh)
            if r is None:
                return None
            out += r[0]
            j = r[1]
            if s.startswith(", ", j):
                out += ", "
                j += 2
            elif j < len(s) and s[j] != close:
                return None
        if j >= len(s):
            return None
        return out + close, j + 1, "container"
    m = (NUM_BLOCK if block else NUM_INLINE).match(s, i)
    if m and re.search(r"\d", m[0]):
        return m[0], m.end(), "number"
    for k in KEYWORDS:
        if s.startswith(k, i) and not (i + len(k) < len(s) and (s[i + len(k)].isalnum() or s[i + len(k)] in "_")):
            return k, i + len(k), "keyword"
    return None


def tagged(s, i, block, meh=False):
    """An inline value, then each parenthesized tag, innermost first."""
    r = atom(s, i, block, meh)
    if r is None:
        return None
    out, j, kind = r
    if j < len(s) and s[j] not in AFTER + "(":
        return None  # a letter after a value: refused, as in 2.0.0
    while True:
        k = j
        if meh:
            while k < len(s) and s[k] == " ":
                k += 1
        elif s.startswith(" ", k):
            k += 1
        if not s.startswith("(", k):
            break
        if not meh and k != j + 1:
            raise GSyntaxError("one space before a tag in YAY; MEH takes any")
        tag, j = paren_tag(s, k)
        out = wrap(tag, out)
        if j < len(s) and s[j] not in AFTER + "(":
            raise GSyntaxError(f"unexpected {s[j]!r} after a tag")
    return out, j


def prefix_tags(rest, meh=False):
    """rest is one or more (tag) one space apart, perhaps then " `" or " >"
    and what follows them: (first tag, the rest after it) or None."""
    if not rest.startswith("("):
        return None
    tag, j = paren_tag(rest, 0)
    after = rest[j:]
    if after == "":
        return tag, ""
    if not after.startswith(" "):
        if not (meh and after.startswith("(")):
            raise GSyntaxError("one space after a prefix tag")
    stripped = after.lstrip(" ")
    if not meh and len(after) - len(stripped) != 1:
        raise GSyntaxError("one space after a prefix tag in YAY; MEH takes any")
    after = stripped
    if after.startswith("(") or is_block_intro(after):
        return tag, after
    raise GSyntaxError("only a block follows a prefix tag; an inline value takes its tag after it")


def split_comment(rest):
    i = 0
    while i < len(rest):
        if rest[i] in "\"'":
            i = quoted(rest, i)
            continue
        if rest[i] == "(":
            i = paren_end(rest, i)
            continue
        if rest.startswith("  #", i):
            return rest[:i], rest[i:]
        i += 1
    return rest, ""


def is_block_intro(rest):
    return rest == "`" or rest.startswith("` ") or rest == ">" or rest.startswith("> ")


def indent(line):
    return len(line) - len(line.lstrip(" "))


def desugar(src, meh=False):
    lines = src.split("\n")
    out = []
    i = 0
    first = True
    while i < len(lines):
        line = lines[i]
        n = indent(line)
        body = line[n:]
        if body == "" or body.startswith("#"):
            out.append(line)
            i += 1
            continue
        root = first and n == 0
        first = False
        lead = ""
        while body.startswith("- "):
            lead += "- "
            body = body[2:]
        if body == "-":
            out.append(line)
            i += 1
            continue
        m = KEY.match(body)
        key = None
        if m and not body.startswith("{"):
            key = m[0]
            body = body[m.end():]
            if body.startswith(" "):
                body = body[1:]
            elif body:
                out.append(line)
                i += 1
                continue
        col = n + len(lead)
        head = " " * n + lead + (key + (" " if body else "") if key else "")
        if is_block_intro(body):
            out.append(line)
            i += 1
            while i < len(lines) and (lines[i] == "" or indent(lines[i]) > n):
                out.append(lines[i])
                i += 1
            continue
        rest, comment = split_comment(body)
        p = prefix_tags(rest, meh)
        if p is not None:
            tag, after = p
            # The block below: deeper than the key or the dash, and at the
            # root, the rest of the document at column 0.
            whole = root and not key and not lead
            thr = -1 if whole else (col if key else max(col - 2, n))
            below = next((x for x in lines[i + 1:] if x.strip()), None)
            same_line = after.startswith("` ") or after.startswith("> ")
            if not same_line and not after.startswith("(") and after not in ("`", ">") and (
                    below is None or indent(below) <= thr):
                raise GSyntaxError(f"({tag}) has no block below it")
            if whole and below is not None and indent(below) != 0 and not same_line:
                raise GSyntaxError("a tagged root block sits at column 0")
            if key and after.startswith("` "):
                raise GSyntaxError("a property's block string opens alone on its line")
            ind = " " * (col + 2 if key else col)
            if key:
                out.append(" " * n + lead + key)
            tagline = ("" if key else " " * n + lead) + ind * bool(key) + '"#tag": ' + json.dumps(tag, ensure_ascii=False)
            out.append(tagline + comment)
            shift = 2
            pay = []
            if not key and same_line:
                # The port has no `key:` over a same-line block string, so the
                # first line moves down to the body's indent, which reads the same.
                out.append(ind + "payload: `")
                if after.startswith("` "):
                    k = i + 1
                    deeper = []
                    while k < len(lines) and (lines[k] == "" or indent(lines[k]) > n):
                        if lines[k]:
                            deeper.append(indent(lines[k]))
                        k += 1
                    pay = [" " * ((min(deeper) + 4) if deeper else len(ind) + 2) + after[2:]]
                if after.startswith("> "):
                    hexpart, c2 = split_comment(after[2:])
                    if hexpart.startswith("#"):
                        hexpart, c2 = "", " " + hexpart
                    out[-1] = ind + "payload: >" + c2
                    pay = [" " * (len(ind) + 4) + hexpart] if hexpart else []
                lines[i + 1:i + 1] = pay
                shift = 4
            else:
                out.append(ind + "payload:" + (" " + after if after else ""))
            if whole:
                shift = 2
            k = i + 1 + len(pay)
            while k < len(lines) and (lines[k] == "" or indent(lines[k]) > thr):
                if lines[k]:
                    lines[k] = " " * shift + lines[k]
                k += 1
            # reprocess the payload line, which may carry more tags
            lines[i] = out.pop()
            continue
        r = tagged(rest, 0, True, meh) if rest else None
        if r is not None and r[1] == len(rest):
            out.append(head + r[0] + comment)
        else:
            out.append(line)
        i += 1
    return "\n".join(out)


if __name__ == "__main__":
    path = sys.argv[1]
    sys.stdout.write(desugar(open(path).read(), meh=path.endswith(".meh")))
