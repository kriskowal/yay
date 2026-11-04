#!/usr/bin/env python3
"""
References, against the cases of the design
(doc/design/references/cases/), and the inline bytes they sit beside.
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import libyay as yay


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
        self.assertRefused("a: <1a:b>\n", "Invalid hex digit", 1, 7)
        self.assertRefused("a: <ffz>\n", "Invalid hex digit", 1, 7)

    def test_still_read(self):
        self.assertEqual(yay.loads("a: <ff 00>\n"), {"a": b"\xff\x00"})
        self.assertEqual(yay.loads("a: <>\n"), {"a": b""})


if __name__ == "__main__":
    unittest.main()
