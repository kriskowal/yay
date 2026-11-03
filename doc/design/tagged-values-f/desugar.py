#!/usr/bin/env python3
"""
Throwaway prototype of option F (doc/design/tagged-values.md), not a port.

It rewrites option F's tagged values into YAY 2.0.0 text that spells each one
as the reserved object {"#tag": tag, payload: value}, then hands the result to
the unmodified Python port. A document with no F syntax comes back byte for
byte, which is how `check.py` shows that F changes no existing document.
"""

import json
import re
import sys

NAME = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
KEY = re.compile(r'([A-Za-z0-9_-]+|"(?:[^"\\]|\\.)*"|\'(?:[^\'\\]|\\.)*\'):(?= |$)')
# What may follow a value and its tags inline: the end, a space, a separator.
AFTER = " ,]}"


class FSyntaxError(Exception):
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
    raise FSyntaxError("unterminated string")


def unquote(text):
    if text[0] == "'":
        return re.sub(r"\\(['\\])", r"\1", text[1:-1])
    try:
        return json.loads(re.sub(r"\\u\{([0-9a-fA-F]+)\}", lambda m: "\\u%04x" % int(m[1], 16), text))
    except ValueError as e:
        raise FSyntaxError(f"bad string {text}: {e}")


def paren_tag(s, i):
    """A parenthesized tag at s[i] == "(": raw text up to the first ")",
    or one quoted string. Returns (tag, end) or raises."""
    j = i + 1
    if j < len(s) and s[j] in "\"'":
        k = quoted(s, j)
        if k >= len(s) or s[k] != ")":
            raise FSyntaxError("a quoted tag closes with ) at once")
        return unquote(s[j:k]), k + 1
    k = s.find(")", j)
    if k < 0:
        raise FSyntaxError("unclosed (")
    raw = s[j:k]
    if raw == "":
        raise FSyntaxError("empty () ; write ('') for the empty tag")
    if raw != raw.strip(" "):
        raise FSyntaxError("no space at the edges of a raw tag; quote it")
    if "\t" in raw:
        raise FSyntaxError("no tab in a raw tag; quote it")
    return raw, k + 1


def wrap(tag, text):
    return '{"#tag": %s, payload: %s}' % (json.dumps(tag, ensure_ascii=False), text)


NUM_BLOCK = re.compile(r"-?(?:\d(?: ?\d)*)?(?:\.(?:\d(?: ?\d)*)?)?(?:e[+-]?\d(?: ?\d)*)?")
NUM_INLINE = re.compile(r"-?\d*(?:\.\d*)?(?:e[+-]?\d+)?")
KEYWORDS = ("-infinity", "null", "true", "false", "nan", "infinity")


def atom(s, i, block):
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
            r = tagged(s, j, False)
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


def tagged(s, i, block):
    """An inline value, an optional fused simple name, then each
    parenthesized tag, innermost first."""
    r = atom(s, i, block)
    if r is None:
        return None
    out, j, kind = r
    m = NAME.match(s, j)
    if m:
        name = m[0]
        if kind == "keyword":
            raise FSyntaxError(f"no fused tag on a keyword: write {out} ({name})")
        if kind == "number" and name[0] in "eE":
            raise FSyntaxError(f"a fused tag after a number does not begin with e: write {out} ({name})")
        if kind == "number" and re.fullmatch(r"-?0", out) and name[0] in "xXoObB":
            raise FSyntaxError(f"{out}{name} is not a number; YAY has no 0x, 0o or 0b")
        out, j = wrap(name, out), m.end()
    if j < len(s) and s[j] not in AFTER + "(":
        raise FSyntaxError(f"unexpected {s[j]!r} after a value")
    while True:
        if s.startswith(" (", j):
            k = j + 1
        elif s.startswith("(", j):
            k = j  # fused "(": accepted, and the formatter spaces it
        else:
            break
        tag, j = paren_tag(s, k)
        out = wrap(tag, out)
        if j < len(s) and s[j] not in AFTER + "(":
            raise FSyntaxError(f"unexpected {s[j]!r} after a tag")
    return out, j


def prefix_tags(rest):
    """rest is one or more (tag) one space apart, perhaps then " `" or " >"
    and what follows them: (first tag, the rest after it) or None."""
    if not rest.startswith("("):
        return None
    tag, j = paren_tag(rest, 0)
    after = rest[j:]
    if after == "":
        return tag, ""
    if not after.startswith(" "):
        raise FSyntaxError("one space after a prefix tag")
    after = after[1:]
    if after.startswith("(") or is_block_intro(after):
        return tag, after
    raise FSyntaxError("only a block follows a prefix tag; an inline value takes its tag after it")


def split_comment(rest):
    i = 0
    while i < len(rest):
        if rest[i] in "\"'":
            i = quoted(rest, i)
            continue
        if rest[i] == "(":
            k = rest.find(")", i)
            if k > 0:
                if rest[i + 1:i + 2] in ("'", '"'):
                    k = quoted(rest, i + 1)
                i = k + 1
                continue
        if rest.startswith("  #", i):
            return rest[:i], rest[i:]
        i += 1
    return rest, ""


def is_block_intro(rest):
    return rest == "`" or rest.startswith("` ") or rest == ">" or rest.startswith("> ")


def indent(line):
    return len(line) - len(line.lstrip(" "))


def desugar(src):
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
        p = prefix_tags(rest)
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
                raise FSyntaxError(f"({tag}) has no block below it")
            if whole and below is not None and indent(below) != 0 and not same_line:
                raise FSyntaxError("a tagged root block sits at column 0")
            if key and after.startswith("` "):
                raise FSyntaxError("a property's block string opens alone on its line")
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
        r = tagged(rest, 0, True) if rest else None
        if r is not None and r[1] == len(rest):
            out.append(head + r[0] + comment)
        else:
            out.append(line)
        i += 1
    return "\n".join(out)


if __name__ == "__main__":
    sys.stdout.write(desugar(open(sys.argv[1]).read()))
