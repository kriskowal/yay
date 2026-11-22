#!/usr/bin/env python3
"""
References: MEH's leniency is not YAY's, so the corpus's MEH reference cases
are refused as YAY; and the corpus's reference documents survive the writer.
Then the messages, the model, and the inline bytes references sit beside.
The corpus reference cases in test/yay and test/nay are read with the rest
of the corpus by test_yay.py.
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import libyay as yay
from libyay import Reference, Tagged

TEST = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "test")


def cases(kind: str, ext: str) -> list[str]:
    """The corpus's reference cases in test/<kind>, as paths."""
    folder = os.path.join(TEST, kind)
    return sorted(
        os.path.join(folder, f)
        for f in os.listdir(folder)
        if f.startswith("reference-") and f.endswith("." + ext)
    )


def read(path: str) -> str:
    with open(path, encoding="utf-8") as fp:
        return fp.read()


class Meh(unittest.TestCase):
    def test_refused(self):
        paths = cases("meh", "meh")
        self.assertGreaterEqual(len(paths), 2)
        for path in paths:
            with self.subTest(os.path.basename(path)):
                with self.assertRaises(yay.YaySyntaxError):
                    yay.loads(read(path))


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

    def test_refuses_text_that_is_not_a_reference(self):
        bad = [
            "",
            "not a uri",
            "HTTPS://x",
            "urn:a b",
            "https://a>b",
            "https://x/%zz",
            "./a.yay",
        ]
        for uri in bad:
            with self.subTest(uri):
                with self.assertRaises(ValueError) as cm:
                    Reference(uri)
                self.assertTrue(str(cm.exception).startswith("Not a reference: "))

    def test_accepts_what_the_reader_reads(self):
        for uri in ("urn:x", "https://example.com/a?b=c#d", "urn:%C3%A9", "urn:é"):
            with self.subTest(uri):
                self.assertEqual(Reference(uri).uri, uri)
                self.assertEqual(yay.loads(f"<{uri}>\n"), Reference(uri))

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
        paths = cases("yay", "yay")
        self.assertGreaterEqual(len(paths), 15)
        for path in paths:
            value = yay.loads(read(path))
            with self.subTest(os.path.basename(path)):
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
