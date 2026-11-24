#!/usr/bin/env python3
"""
Runs the option G prototype (desugar.py) over the YAY corpus and G's cases.

1. Every test/yay/*.yay desugars to itself, read as YAY and as MEH, and
   every test/meh/*.meh read as MEH: G changes no valid document.
2. No test/nay/*.nay becomes valid, read as YAY or as MEH: G gives meaning
   only to text YAY 2.0.0 refuses.
3. Each cases/*.yay parses, through the prototype and the Python port, to
   its cases/*.json, as YAY and as MEH; each cases/*.meh does so as MEH and
   is refused as YAY; each cases/*.nay is refused as YAY.

This is the design's evidence, and it holds only on the design's tree: a
corpus and a Python port without tagged values. The cases have since moved
into the corpus (test/yay, test/meh and test/nay, each named tagged-*, with
each port's expected value) and cases/ is gone, so run this from a checkout
of the design's commit, the one that added cases/:

    git log --diff-filter=A --format=%H -- doc/design/tagged-values-g/cases | tail -1
"""

import glob
import json
import os
import sys

here = os.path.dirname(os.path.abspath(__file__))
root = os.path.join(here, "..", "..", "..")
sys.path.insert(0, os.path.join(root, "python"))
sys.path.insert(0, here)
import libyay as yay
from desugar import desugar, GSyntaxError

if not os.path.isdir(os.path.join(here, "cases")):
    sys.exit(
        "check.py: cases/ moved into the corpus; run this from a checkout of "
        "the design's commit (see the docstring)"
    )


def plain(v):
    if isinstance(v, bytes):
        return "*" + v.hex()
    if isinstance(v, list):
        return [plain(x) for x in v]
    if isinstance(v, dict):
        return {k: plain(x) for k, x in v.items()}
    if isinstance(v, float) and v != v:
        return "NaN"
    return v


failures = 0
changed = []
for path in sorted(glob.glob(os.path.join(root, "test", "yay", "*.yay"))):
    src = open(path, newline="").read()
    if desugar(src) != src or desugar(src, meh=True) != src:
        changed.append(os.path.basename(path))
print(f"corpus .yay: {len(glob.glob(os.path.join(root, 'test', 'yay', '*.yay')))} files, changed by G: {changed or 'none'}")
failures += len(changed)

mehs = sorted(glob.glob(os.path.join(root, "test", "meh", "*.meh")))
changed = [os.path.basename(p) for p in mehs if desugar(open(p, newline="").read(), meh=True) != open(p, newline="").read()]
print(f"corpus .meh: {len(mehs)} files, changed by G as MEH: {changed or 'none'}")
failures += len(changed)

becomes = []
for path in sorted(glob.glob(os.path.join(root, "test", "nay", "*.nay"))):
    src = open(path, newline="").read()
    for meh in (False, True):
        try:
            yay.loads(desugar(src, meh))
            becomes.append(os.path.basename(path) + (" (MEH)" if meh else ""))
        except Exception:
            pass
print(f"corpus .nay: {len(glob.glob(os.path.join(root, 'test', 'nay', '*.nay')))} files, accepted under G: {becomes or 'none'}")
failures += len(becomes)

def accepts(path, meh):
    global failures
    name = os.path.basename(path) + (" as MEH" if meh else "")
    want = json.load(open(path[:-4] + ".json"))
    try:
        got = plain(yay.loads(desugar(open(path, newline="").read(), meh)))
    except Exception as e:
        print(f"FAIL {name}: {e}")
        failures += 1
        return
    if got != want:
        print(f"FAIL {name}: got {json.dumps(got)}")
        failures += 1
    else:
        print(f"ok   {name}")


def refuses(path, meh=False):
    global failures
    name = os.path.basename(path) + (" as MEH" if meh else " as YAY")
    try:
        got = yay.loads(desugar(open(path, newline="").read(), meh))
        print(f"FAIL {name}: accepted as {json.dumps(plain(got))}")
        failures += 1
    except Exception as e:
        print(f"ok   {name}: refused ({type(e).__name__}: {e})")


for path in sorted(glob.glob(os.path.join(here, "cases", "*.yay"))):
    accepts(path, False)
    accepts(path, True)

for path in sorted(glob.glob(os.path.join(here, "cases", "*.meh"))):
    accepts(path, True)
    refuses(path, False)

for path in sorted(glob.glob(os.path.join(here, "cases", "*.nay"))):
    refuses(path)

sys.exit(1 if failures else 0)
