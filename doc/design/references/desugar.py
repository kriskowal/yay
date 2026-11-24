#!/usr/bin/env python3
"""
Throwaway prototype of references (doc/design/references.md), not a port.

It rewrites each reference, `<scheme:...>`, into YAY 3.0.0 text that spells
it as a string under a tag no document uses, `"..." (yay-proto:reference)`,
then hands the result to an unmodified 3.0.0 reader. A tag written after the
reference follows the prototype's tag, so `<https://x> (t)` becomes
`"https://x" (yay-proto:reference) (t)`, a tagged value whose payload is the
reference, as the design has it. A document with no `<scheme:` comes back
byte for byte, which is how `check.py` shows that references change no
existing document.

The scan skips quoted strings, tags, comments and block strings, and checks
rule R1 as it goes: a scheme, lowercase, then the URI to the `>` on the same
line, with no space and none of RFC 3986's excluded characters.
"""

import re
import sys
import unicodedata

PROTO_TAG = "yay-proto:reference"
SCHEME = re.compile(r"[A-Za-z][A-Za-z0-9+.-]*:")
# Excluded from a URI by RFC 3986 (and from an IRI by RFC 3987).
EXCLUDED = set(' <>"{}|\\^`')
HEX = set("0123456789abcdefABCDEF")


class RSyntaxError(Exception):
    pass


def skip_quoted(s, i):
    q = s[i]
    j = i + 1
    while j < len(s):
        if s[j] == "\\":
            j += 2
            continue
        if s[j] == q:
            return j + 1
        j += 1
    return len(s)


def skip_tag(s, i):
    j = i + 1
    while j < len(s):
        if s[j] == "\\":
            j += 2
            continue
        if s[j] == ")":
            return j + 1
        j += 1
    return len(s)


def check_uri(uri, scheme):
    if scheme != scheme.lower():
        raise RSyntaxError(f"a reference's scheme is lowercase: {scheme!r}")
    j = 0
    while j < len(uri):
        c = uri[j]
        if c in EXCLUDED:
            raise RSyntaxError(f"{c!r} in a reference; percent-encode it")
        if c == "%":
            if len(uri) < j + 3 or uri[j + 1] not in HEX or uri[j + 2] not in HEX:
                raise RSyntaxError("% in a reference begins a percent-encoding")
            j += 3
            continue
        cat = unicodedata.category(c)
        if cat in ("Cc", "Cs", "Zs", "Zl", "Zp") or ord(c) in (0xFFFE, 0xFFFF):
            raise RSyntaxError(f"U+{ord(c):04X} in a reference")
        j += 1


def rewrite_line(line, meh=False):
    out = []
    i = 0
    n = len(line)
    while i < n:
        c = line[i]
        if c in "'\"":
            j = skip_quoted(line, i)
            out.append(line[i:j])
            i = j
        elif c == "(":
            j = skip_tag(line, i)
            out.append(line[i:j])
            i = j
        elif c == "#" and (i == 0 or line[:i].strip() == "" or line[i - 1] == " "):
            out.append(line[i:])
            break
        elif c == "<":
            m = SCHEME.match(line, i + 1)
            if not m:
                # Bytes, held to today's rules by the reader.
                j = line.find(">", i)
                j = n if j < 0 else j + 1
                out.append(line[i:j])
                i = j
                continue
            end = line.find(">", m.end())
            if end < 0:
                raise RSyntaxError("a reference is closed on its line")
            uri = line[i + 1 : end]
            if meh:
                # MEH: an uppercase scheme reads lowercased, as hex does.
                uri = m[0].lower() + uri[len(m[0]) :]
            check_uri(uri[len(m[0]) - 1 :], uri[: len(m[0]) - 1])
            after = line[end + 1 : end + 2]
            if meh and after == "(":
                pass  # MEH: a tag may touch its value
            elif meh and after not in ("", " ", ",", "]", "}"):
                # MEH: glued text is a bare string, as `<ff>rgb` is today.
                j = line.find(" ", end)
                j = n if j < 0 else j
                out.append(line[i:j])
                i = j
                continue
            elif after not in ("", " ", ",", "]", "}"):
                raise RSyntaxError(f"{after!r} glued to a reference")
            escaped = uri.replace("\\", "\\\\").replace('"', '\\"')
            out.append(f'"{escaped}" ({PROTO_TAG})')
            i = end + 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def opens_block_string(line):
    """Whether a line introduces a block string whose content follows."""
    code = line
    i = 0
    while i < len(code):
        if code[i] in "'\"":
            i = skip_quoted(code, i)
        elif code[i] == "(":
            i = skip_tag(code, i)
        elif code[i] == "#" and (i == 0 or code[i - 1] == " "):
            code = code[:i].rstrip()
            break
        elif code[i] == "`":
            return True
        else:
            i += 1
    return False


def desugar(src, meh=False):
    lines = src.split("\n")
    out = []
    block = None  # the indent of a block string's introducer
    for line in lines:
        indent = len(line) - len(line.lstrip(" "))
        if block is not None:
            if line.strip() == "" or indent > block:
                out.append(line)
                continue
            block = None
        out.append(rewrite_line(line, meh))
        if opens_block_string(line):
            block = indent
    return "\n".join(out)


if __name__ == "__main__":
    sys.stdout.write(desugar(sys.stdin.read()))
