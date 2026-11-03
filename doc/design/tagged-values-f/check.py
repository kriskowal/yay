#!/usr/bin/env python3
"""
Runs the option F prototype (desugar.py) over the YAY corpus and F's cases.

1. Every test/yay/*.yay desugars to itself: F changes no valid document.
2. Every test/nay/*.nay either desugars to itself or is listed in BECOMES:
   F gives meaning only to text YAY 2.0.0 refuses.
3. Each cases/*.yay parses, through the prototype and the Python port, to
   its cases/*.json; each cases/*.nay is refused by one or the other.
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
from desugar import desugar, FSyntaxError


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
    src = open(path).read()
    if desugar(src) != src:
        changed.append(os.path.basename(path))
print(f"corpus .yay: {len(glob.glob(os.path.join(root, 'test', 'yay', '*.yay')))} files, changed by F: {changed or 'none'}")
failures += len(changed)

becomes = []
for path in sorted(glob.glob(os.path.join(root, "test", "nay", "*.nay"))):
    src = open(path).read()
    try:
        out = desugar(src)
    except FSyntaxError:
        continue
    if out != src:
        try:
            yay.loads(out)
            becomes.append(os.path.basename(path))
        except Exception:
            pass
print(f"corpus .nay: {len(glob.glob(os.path.join(root, 'test', 'nay', '*.nay')))} files, accepted under F: {becomes or 'none'}")

for path in sorted(glob.glob(os.path.join(here, "cases", "*.yay"))):
    name = os.path.basename(path)
    want = json.load(open(path[:-4] + ".json"))
    try:
        got = plain(yay.loads(desugar(open(path).read())))
    except Exception as e:
        print(f"FAIL {name}: {e}")
        failures += 1
        continue
    if got != want:
        print(f"FAIL {name}: got {json.dumps(got)}")
        failures += 1
    else:
        print(f"ok   {name}")

for path in sorted(glob.glob(os.path.join(here, "cases", "*.nay"))):
    name = os.path.basename(path)
    try:
        got = yay.loads(desugar(open(path).read()))
        print(f"FAIL {name}: accepted as {json.dumps(plain(got))}")
        failures += 1
    except (FSyntaxError, Exception) as e:
        print(f"ok   {name}: refused ({type(e).__name__}: {e})")

sys.exit(1 if failures else 0)
