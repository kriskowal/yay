"""
YAY references: a URI or IRI in angle brackets, `<scheme:...>`.
"""

import re
from dataclasses import dataclass


@dataclass(frozen=True)
class Reference:
    """A reference: a URI or IRI, held as written. It designates, and is
    never dereferenced.

    Two references are equal when their texts are, character for
    character, and a reference never equals a string.
    """

    uri: str

    def __post_init__(self):
        if not isinstance(self.uri, str):
            raise TypeError("A reference must be a string")


_SCHEME = re.compile(r"[A-Za-z][A-Za-z0-9+.-]*:")
_URI_ASCII = frozenset(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
    "-._~:/?#[]@!$&'()*+,;="
)
_HEX = frozenset("0123456789ABCDEFabcdef")


def scheme_len(s: str, pos: int = 0) -> int | None:
    """The length of the scheme, colon included, if s at pos (the text
    after "<") begins [A-Za-z][A-Za-z0-9+.-]*":". Uppercase letters are
    matched so that the reader can name an uppercase scheme rather than call
    it bad hex."""
    m = _SCHEME.match(s, pos)
    return m.end() - pos if m else None


def is_uri_char(c: str) -> bool:
    """Whether c may appear in a reference (RFC 3986 and RFC 3987), "%"
    aside, which must begin a percent-encoding."""
    cp = ord(c)
    if cp < 0x80:
        return c in _URI_ASCII
    return cp >= 0xA0 and not c.isspace() and cp not in (0xFFFE, 0xFFFF)


def check_uri(uri: str) -> tuple[str, int] | None:
    """Why uri cannot be a reference's text, and the offset of the fault, or
    None if it can: a scheme, then characters a URI may hold."""
    n = scheme_len(uri)
    if n is None:
        return ("Expected a scheme in reference", 0)
    if uri[: n - 1] != uri[: n - 1].lower():
        return (f'Uppercase scheme "{uri[: n - 1]}" in reference (use lowercase)', 0)
    i = n
    while i < len(uri):
        c = uri[i]
        if c == "%":
            if not (i + 2 < len(uri) and uri[i + 1] in _HEX and uri[i + 2] in _HEX):
                return (
                    "Bad percent-encoding in reference (use % and two hex digits)",
                    i,
                )
        elif not is_uri_char(c):
            if c == " ":
                return ("Unexpected space in reference", i)
            if c == "\t":
                return ("Tab not allowed in reference", i)
            return (f'Unexpected character "{c}" in reference', i)
        i += 1
    return None


def mask_references(s: str) -> str:
    """s with each reference's text, outside strings and tags, replaced by
    "x", so that scanners for commas and brackets do not read it as
    syntax."""
    out = []
    i = 0
    close = None
    while i < len(s):
        c = s[i]
        if close is not None:
            out.append(c)
            if c == "\\" and i + 1 < len(s):
                out.append(s[i + 1])
                i += 2
                continue
            if c == close:
                close = None
            i += 1
            continue
        if c in "\"'":
            close = c
        elif c == "(":
            close = ")"
        elif c == "<" and scheme_len(s, i + 1) is not None:
            end = s.find(">", i)
            if end >= 0:
                out.append("<" + "x" * (end - i - 1) + ">")
                i = end + 1
                continue
        out.append(c)
        i += 1
    return "".join(out)
