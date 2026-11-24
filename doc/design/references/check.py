#!/usr/bin/env python3
"""
Runs the references prototype (desugar.py) over the 3.0.0 corpus and its cases.

1. Every test/yay/*.yay and test/meh/*.meh desugars to itself, byte for
   byte: references change no valid document, the tagged corpus included.
2. No test/nay/*.nay becomes valid, read as YAY by the Python port or as
   YAY or MEH by binyay 3.0.0, the two bytes typos included.
3. Each cases/*.yay reads, through the prototype, to its cases/*.yson (the
   design's YSON: a reference is "$uri") in the Python port and in binyay as
   YAY and as MEH, and is refused by both readers without the prototype;
   each cases/*.meh does so as MEH and is refused as YAY; each cases/*.nay
   is refused as YAY and as MEH.

Usage: check.py <binyay 3.0.0>

This is the design's evidence, and it holds only on the design's tree: a
corpus, a Python port and a binyay without references. The cases have since
moved into the corpus (test/yay, test/meh and test/nay, each named
reference-*, with each port's expected value) and cases/ is gone, so run
this from a checkout of the design's commit, the one that added cases/:

    git log --diff-filter=A --format=%H -- doc/design/references/cases | tail -1
"""

import glob
import json
import os
import subprocess
import sys

here = os.path.dirname(os.path.abspath(__file__))
root = os.path.join(here, "..", "..", "..")
sys.path.insert(0, os.path.join(root, "python"))
sys.path.insert(0, here)
import libyay as yay
from libyay.tagged import Tagged
from desugar import desugar, RSyntaxError, PROTO_TAG

BINYAY = sys.argv[1]

if not os.path.isdir(os.path.join(here, "cases")):
    sys.exit(
        "check.py: cases/ moved into the corpus; run this from a checkout of "
        "the design's commit (see the docstring)"
    )


def read(path):
    return open(path, newline="", encoding="utf-8").read()


def yson_string(s):
    return "!" + s if s and "!" <= s[0] <= "/" else s


def py_yson(v):
    """The design's YSON for a value the Python port read."""
    if isinstance(v, Tagged):
        if v.tag == PROTO_TAG:
            return "$" + v.payload
        return {"#tag": yson_string(v.tag), "payload": py_yson(v.payload)}
    if isinstance(v, bool) or v is None:
        return v
    if isinstance(v, int):
        return ("+" if v >= 0 else "") + str(v)
    if isinstance(v, float):
        return "#NaN" if v != v else v
    if isinstance(v, bytes):
        return "*" + v.hex()
    if isinstance(v, str):
        return yson_string(v)
    if isinstance(v, list):
        return [py_yson(x) for x in v]
    return {("!" + k if k[:1] in "#*!" else k): py_yson(x) for k, x in v.items()}


def bin_yson(v):
    """binyay's YSON with the prototype's tag turned into the design's "$uri"."""
    if isinstance(v, dict):
        if set(v) == {"#tag", "payload"} and v["#tag"] == PROTO_TAG:
            return "$" + v["payload"]
        return {k: bin_yson(x) for k, x in v.items()}
    if isinstance(v, list):
        return [bin_yson(x) for x in v]
    return v


def binyay(src, fmt, sugar=False):
    if sugar:
        src = desugar(src, fmt == "meh")
    r = subprocess.run([BINYAY, "-f", fmt, "-t", "yson"], input=src.encode(), capture_output=True)
    if r.returncode:
        raise ValueError(r.stderr.decode().strip())
    return bin_yson(json.loads(r.stdout))


def python(src):
    return py_yson(yay.loads(src))


failures = 0


def fail(msg):
    global failures
    failures += 1
    print("FAIL " + msg)


def valid(reader, src):
    try:
        reader(src)
        return True
    except Exception:
        return False


# Each reader takes a document's text; `sugar` reads it with references.
READERS = {
    "python": lambda s, sugar: python(desugar(s) if sugar else s),
    "binyay -f yay": lambda s, sugar: binyay(s, "yay", sugar),
    "binyay -f meh": lambda s, sugar: binyay(s, "meh", sugar),
}

for kind in ("yay", "meh"):
    paths = sorted(glob.glob(os.path.join(root, "test", kind, f"*.{kind}")))
    changed = [os.path.basename(p) for p in paths
               if desugar(read(p)) != read(p) or desugar(read(p), meh=True) != read(p)]
    print(f"corpus .{kind}: {len(paths)} files, changed by references: {changed or 'none'}")
    failures += len(changed)

nays = sorted(glob.glob(os.path.join(root, "test", "nay", "*.nay")))
becomes = []
for path in nays:
    orig = read(path)
    for name, reader in READERS.items():
        # A loose form MEH already reads is not one references made valid.
        if valid(lambda s: reader(s, True), orig) and not valid(lambda s: reader(s, False), orig):
            becomes.append(f"{os.path.basename(path)} ({name})")
print(f"corpus .nay: {len(nays)} files, made valid by references: {becomes or 'none'}")
failures += len(becomes)
for typo in ("bytearray-inline-invalid-hex-digit", "bytearray-invalid-uppercase-hex"):
    src = read(os.path.join(root, "test", "nay", typo + ".nay"))
    for name, reader in READERS.items():
        if not valid(lambda s: reader(s, True), src):
            print(f"ok   {typo} [{name}]: still refused")
        elif valid(lambda s: reader(s, False), src) and reader(src, True) == reader(src, False):
            print(f"ok   {typo} [{name}]: read as today, {json.dumps(reader(src, True))}")
        else:
            fail(f"{typo} [{name}]: made valid")


def before(case, name, reader, src):
    """What 3.0.0 makes of a case without references."""
    try:
        got = reader(src, False)
    except Exception:
        return
    if name == "binyay -f meh":
        print(f"note {case} [{name}]: MEH reads it today as {json.dumps(got)}")
    else:
        fail(f"{case} [{name}]: valid in 3.0.0 without references")


def accepts(path, names):
    name = os.path.basename(path)
    src = read(path)
    want = json.load(open(path[:-4] + ".yson", encoding="utf-8"))
    for rname in names:
        reader = READERS[rname]
        try:
            got = reader(src, True)
        except Exception as e:
            fail(f"{name} [{rname}]: {e}")
            continue
        if got != want:
            fail(f"{name} [{rname}]: got {json.dumps(got)}")
        else:
            print(f"ok   {name} [{rname}]")
        if not name.startswith("unchanged-"):
            before(name, rname, reader, src)


def refuses(path, names):
    name = os.path.basename(path)
    src = read(path)
    for rname in names:
        reader = READERS[rname]
        try:
            got = reader(src, True)
        except Exception as e:
            print(f"ok   {name} [{rname}]: refused ({str(e).splitlines()[0][:70]})")
            continue
        if rname == "binyay -f meh" and valid(lambda s: reader(s, False), src) and reader(src, False) == got:
            print(f"ok   {name} [{rname}]: read as today, {json.dumps(got)}")
        else:
            fail(f"{name} [{rname}]: accepted as {json.dumps(got)}")


ALL = list(READERS)
STRICT = ["python", "binyay -f yay"]
for path in sorted(glob.glob(os.path.join(here, "cases", "*.yay"))):
    accepts(path, ALL)
for path in sorted(glob.glob(os.path.join(here, "cases", "*.meh"))):
    accepts(path, ["binyay -f meh"])
    refuses(path, STRICT)
for path in sorted(glob.glob(os.path.join(here, "cases", "*.nay"))):
    refuses(path, ALL)

print(f"{failures} failures")
sys.exit(1 if failures else 0)
