#!/usr/bin/env python3
"""
Tagged values, against the cases of the design
(doc/design/tagged-values-g/cases/): each .yay parses to its .json, with a
tagged value as {"#tag": tag, "payload": payload} and bytes as "*hex", and
each .nay and .meh is refused, MEH's leniency not being YAY's.
"""

import json
import math
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))
import libyay as yay
from libyay import Tagged

CASES = os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "..",
    "doc",
    "design",
    "tagged-values-g",
    "cases",
)


def cases(ext: str) -> list[str]:
    return sorted(f for f in os.listdir(CASES) if f.endswith("." + ext))


def read(name: str) -> str:
    with open(os.path.join(CASES, name), encoding="utf-8") as fp:
        return fp.read()


def desugar(value):
    """The value as the cases' JSON spells it."""
    if isinstance(value, Tagged):
        return {"#tag": value.tag, "payload": desugar(value.payload)}
    if isinstance(value, bytes):
        return "*" + value.hex()
    if isinstance(value, list):
        return [desugar(v) for v in value]
    if isinstance(value, dict):
        return {k: desugar(v) for k, v in value.items()}
    # The prototype's JSON spells these as strings or bare words
    if isinstance(value, float) and math.isnan(value):
        return "NaN"
    if isinstance(value, float) and math.isinf(value):
        return "Infinity" if value > 0 else "-Infinity"
    return value


class AcceptedCases(unittest.TestCase):
    def test_cases(self):
        names = cases("yay")
        self.assertTrue(names)
        for name in names:
            with self.subTest(name):
                expected = json.loads(
                    read(name[:-4] + ".json").replace("-Infinity", '"-Infinity"')
                )
                self.assertEqual(desugar(yay.loads(read(name))), expected)


class RefusedCases(unittest.TestCase):
    def test_cases(self):
        names = cases("nay") + cases("meh")
        self.assertTrue(names)
        for name in names:
            with self.subTest(name):
                with self.assertRaises(yay.YayError):
                    yay.loads(read(name))


class Model(unittest.TestCase):
    def test_fields(self):
        t = Tagged("rgb", [255, 136, 0])
        self.assertEqual(t.tag, "rgb")
        self.assertEqual(t.payload, [255, 136, 0])

    def test_frozen(self):
        t = Tagged("s", 10)
        with self.assertRaises(AttributeError):
            t.tag = "ms"

    def test_equality(self):
        self.assertEqual(Tagged("s", 10), Tagged("s", 10))
        self.assertEqual(
            Tagged("t", [1, {"a": b"\x01"}]),
            Tagged("t", [1, {"a": b"\x01"}]),
        )
        self.assertNotEqual(Tagged("s", 10), Tagged("ms", 10))
        self.assertNotEqual(Tagged("s", 10), Tagged("s", 11))
        # A tagged value never equals its payload, nor its YSON spelling
        self.assertNotEqual(Tagged("s", 10), 10)
        self.assertNotEqual(Tagged("s", 10), {"#tag": "s", "payload": 10})
        self.assertNotEqual(Tagged("a", Tagged("b", 1)), Tagged("b", Tagged("a", 1)))


class Parse(unittest.TestCase):
    def test_suffix_innermost_first(self):
        self.assertEqual(yay.loads("1 (a) (b)\n"), Tagged("b", Tagged("a", 1)))

    def test_prefix_outermost_first(self):
        self.assertEqual(
            yay.loads("- (a) (b)\n  - 1\n"), [Tagged("a", Tagged("b", [1]))]
        )

    def test_escapes(self):
        self.assertEqual(yay.loads("1 (a\\)b\\\\c)\n"), Tagged("a)b\\c", 1))
        self.assertEqual(yay.loads("1 (\\u{1F600})\n"), Tagged("\U0001F600", 1))
        for bad in (
            '1 (\\")\n',
            "1 (\\()\n",
            "1 (\\u{D800})\n",
            "1 (\\u{110000})\n",
            "1 (\\x)\n",
            "1 (\\u{12",
            "1 (\\u{}",
        ):
            with self.subTest(bad):
                with self.assertRaises(yay.YayError):
                    yay.loads(bad)

    def test_tags_in_inline_containers(self):
        self.assertEqual(
            yay.loads("a: [[1] (x), {b: 2} (y)] (z)\n"),
            {"a": Tagged("z", [Tagged("x", [1]), Tagged("y", {"b": 2})])},
        )
        self.assertEqual(yay.loads("a: {} (e)\n"), {"a": Tagged("e", {})})

    def test_tagged_block_string_property(self):
        self.assertEqual(
            yay.loads("a: (t) `\n  x\nb: 1\n"), {"a": Tagged("t", "x\n"), "b": 1}
        )

    def test_tagged_block_string_below_item(self):
        self.assertEqual(yay.loads("- (t)\n  `\n    x\n"), [Tagged("t", "\nx\n")])
        self.assertEqual(yay.loads("- (t)\n  ` x\n    y\n"), [Tagged("t", "x\ny\n")])

    def test_tagged_concatenated_string_item(self):
        self.assertEqual(
            yay.loads("- (t)\n  'a'\n  'b'\n"), [Tagged("t", "ab")]
        )

    def test_refused(self):
        for bad in (
            "(t)\n",
            "(t) 1\n",
            "(t)\n1\n",
            "- (t)\n- 1\n",
            "a: (t)\n",
            "a: (t)\nb:\n  - 1\n",
            "a:\n  b: 1 (t)\n    - 1\n",
            "{a: (t) 1}\n",
            "[(t)]\n",
            "1 (t) (u)  \n",
        ):
            with self.subTest(bad):
                with self.assertRaises(yay.YayError):
                    yay.loads(bad)


class Write(unittest.TestCase):
    def test_suffix_one_space_innermost_first(self):
        self.assertEqual(yay.dumps(Tagged("s", 10)), "10 (s)")
        self.assertEqual(yay.dumps(Tagged("b", Tagged("a", "x"))), "'x' (a) (b)")
        self.assertEqual(
            yay.dumps({"a": [Tagged("x", 1), Tagged("y", [])]}, indent=False),
            "{a: [1 (x), [] (y)]}",
        )

    def test_tag_escapes(self):
        self.assertEqual(yay.dumps(Tagged("a)b\\c", 1)), "1 (a\\)b\\\\c)")
        self.assertEqual(yay.dumps(Tagged("\t\x01\x7f", 1)), "1 (\\t\\u{1}\\u{7F})")
        self.assertEqual(yay.dumps(Tagged(" (\"') ", 1)), "1 ( (\"'\\) )")
        self.assertEqual(yay.dumps(Tagged("", 1)), "1 ()")

    def test_prefix_exactly_over_a_block(self):
        self.assertEqual(
            yay.dumps({"a": Tagged("o", Tagged("i", [1, 2])), "b": Tagged("e", {})}),
            "a: (o) (i)\n  - 1\n  - 2\nb: {} (e)",
        )
        self.assertEqual(yay.dumps(Tagged("p", {"x": 1})), "(p)\nx: 1")
        self.assertEqual(yay.dumps([Tagged("p", {"x": 1})]), "- (p)\n  x: 1")

    def test_round_trip(self):
        values = [
            Tagged("s", 10),
            {"a": Tagged("rgb", [255, 136, 0]), "b": Tagged("x)y\\", b"\xff")},
            {"c": [Tagged("t", {"k": 1}), Tagged("u", [1, 2]), Tagged("e", [])]},
            Tagged("p", {"x": 1, "y": Tagged("q", {"z": Tagged("r", None)})}),
            [Tagged("a", Tagged("b", {"k": "v"}))],
            {"d": Tagged("", Tagged(" edge ", 1.5))},
        ]
        for value in values:
            with self.subTest(value):
                self.assertEqual(yay.loads(yay.dumps(value) + "\n"), value)
                self.assertEqual(
                    yay.loads(yay.dumps(value, indent=False) + "\n"), value
                )


if __name__ == "__main__":
    unittest.main()
