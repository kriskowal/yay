#!/usr/bin/env python3
"""
References, against the cases of the design
(doc/design/references/cases/): each .yay parses to its .yson, with a
reference as "$uri", and each .nay and .meh is refused, MEH's leniency not
being YAY's. Then the model, and the inline bytes references sit beside.
"""

import json
import math
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import libyay as yay
from libyay import Reference, Tagged

CASES = os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "..",
    "doc",
    "design",
    "references",
    "cases",
)


def cases(ext: str) -> list[str]:
    return sorted(f for f in os.listdir(CASES) if f.endswith("." + ext))


def read(name: str) -> str:
    with open(os.path.join(CASES, name), encoding="utf-8") as fp:
        return fp.read()


def yson_string(s: str) -> str:
    """A string as YSON writes it: escaped when it begins with ! through /."""
    return "!" + s if s and "!" <= s[0] <= "/" else s


def to_yson(value):
    """The value as the cases' YSON spells it."""
    if isinstance(value, Reference):
        return "$" + value.uri
    if isinstance(value, Tagged):
        return {"#tag": yson_string(value.tag), "payload": to_yson(value.payload)}
    if isinstance(value, bytes):
        return "*" + value.hex()
    if isinstance(value, bool) or value is None:
        return value
    if isinstance(value, int):
        return str(value) if value < 0 else f"+{value}"
    if isinstance(value, float):
        if math.isnan(value):
            return "#nan"
        if math.isinf(value):
            return "#+inf" if value > 0 else "#-inf"
        return value
    if isinstance(value, str):
        return yson_string(value)
    if isinstance(value, list):
        return [to_yson(v) for v in value]
    if isinstance(value, dict):
        return {
            ("!" + k if k[:1] in ("#", "*", "!") else k): to_yson(v)
            for k, v in value.items()
        }
    raise TypeError(type(value))


class Cases(unittest.TestCase):
    def test_accepted(self):
        names = cases("yay")
        self.assertTrue(names)
        for name in names:
            with self.subTest(name):
                expected = json.loads(read(name[: -len(".yay")] + ".yson"))
                self.assertEqual(to_yson(yay.loads(read(name))), expected)

    def test_refused(self):
        names = cases("nay") + cases("meh")
        self.assertTrue(names)
        for name in names:
            with self.subTest(name):
                with self.assertRaises(yay.YaySyntaxError):
                    yay.loads(read(name))


class Errors(unittest.TestCase):
    def test_refused(self):
        refused = [
            ("a: <HTTPS://x>\n", 'Uppercase scheme "HTTPS" in reference', 1, 4),
            ("a: <https://x y>\n", "Unexpected space in reference", 1, 14),
            ('a: <https://x"y>\n', 'Unexpected character """ in reference', 1, 14),
            ("a: <https://x/%2g>\n", "Bad percent-encoding in reference", 1, 15),
            ("a: <https://x/%2>\n", "Bad percent-encoding in reference", 1, 15),
            ("a: <https://x\n", "Unmatched angle bracket", 1, 4),
            ("a: <https://x>(t)\n", "Expected one space before a tag", 1, 15),
            ("a: <https://x>rgb\n", 'Unexpected character "r"', 1, 15),
            ("<https://x>: 1\n", 'Unexpected character ":"', 1, 12),
            ("a: <./x>\n", "Odd number of hex digits in byte literal", 1, 4),
        ]
        for source, message, line, col in refused:
            with self.subTest(source):
                with self.assertRaises(yay.YaySyntaxError) as cm:
                    yay.loads(source)
                self.assertTrue(str(cm.exception).startswith(message), cm.exception)
                self.assertEqual((cm.exception.line, cm.exception.col), (line, col))


class Model(unittest.TestCase):
    def test_frozen(self):
        r = Reference("https://x")
        self.assertEqual(r.uri, "https://x")
        with self.assertRaises(AttributeError):
            r.uri = "https://y"
        with self.assertRaises(TypeError):
            Reference(b"https://x")

    def test_equal_by_text_never_to_a_string(self):
        self.assertEqual(yay.loads("<https://x>\n"), Reference("https://x"))
        self.assertNotEqual(Reference("https://x/"), Reference("https://x"))
        self.assertNotEqual(Reference("a:%2f"), Reference("a:%2F"))
        self.assertNotEqual(yay.loads("<https://x>\n"), "https://x")
        self.assertNotEqual(Reference("https://x"), Tagged("https://x", None))
        self.assertEqual(hash(Reference("a:b")), hash(Reference("a:b")))

    def test_tagged(self):
        self.assertEqual(
            yay.loads("<ocapn://a.tcp> (Counter)\n"),
            Tagged("Counter", Reference("ocapn://a.tcp")),
        )

    def test_unchanged_elsewhere(self):
        # In strings and tags, and as bytes without a scheme
        self.assertEqual(
            yay.loads("a: '<https://x>' (<b:c>)\nb: <ff>\n"),
            {"a": Tagged("<b:c>", "<https://x>"), "b": b"\xff"},
        )


class Writer(unittest.TestCase):
    def test_written_as_held(self):
        self.assertEqual(yay.dumps(Reference("https://x/a,b]c")), "<https://x/a,b]c>")
        self.assertEqual(
            yay.dumps({"a": [Reference("a:b"), Tagged("t", Reference("c:d"))]}),
            "a:\n  - <a:b>\n  - <c:d> (t)",
        )
        self.assertEqual(
            yay.dumps({"a": Reference("a:b")}, indent=False), "{a: <a:b>}"
        )

    def test_refused(self):
        for uri in ("", "./x", "HTTPS://x", "https://x>", "https://x y", "a:%zz"):
            with self.subTest(uri):
                with self.assertRaises(ValueError):
                    yay.dumps(Reference(uri))

    def test_round_trip(self):
        for name in cases("yay"):
            value = yay.loads(read(name))
            with self.subTest(name):
                self.assertEqual(yay.loads(yay.dumps(value) + "\n"), value)
                self.assertEqual(
                    yay.loads(yay.dumps(value, indent=False) + "\n"), value
                )


class InlineBytes(unittest.TestCase):
    def assertRefused(self, source, message, line, col):
        with self.assertRaises(yay.YaySyntaxError) as cm:
            yay.loads(source)
        self.assertEqual(str(cm.exception), f"{message} (line {line}, col {col})")

    def test_space_after_open(self):
        # GRAMMAR.md: no space after "<", as before ">"
        self.assertRefused("a: < ff>\n", 'Unexpected space after "<"', 1, 5)
        self.assertRefused("a: < https://x>\n", 'Unexpected space after "<"', 1, 5)

    def test_not_hex(self):
        # At the "<", as the shared corpus has it, and an odd count first
        self.assertRefused("a: <1a:b>\n", "Invalid hex digit", 1, 4)
        self.assertRefused("a: <ffzz>\n", "Invalid hex digit", 1, 4)
        self.assertRefused(
            "a: <ffz>\n", "Odd number of hex digits in byte literal", 1, 4
        )

    def test_still_read(self):
        self.assertEqual(yay.loads("a: <ff 00>\n"), {"a": b"\xff\x00"})
        self.assertEqual(yay.loads("a: <>\n"), {"a": b""})


if __name__ == "__main__":
    unittest.main()
