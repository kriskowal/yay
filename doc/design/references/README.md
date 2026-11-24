# References prototype

Throwaway, not a port: the evidence for [references.md](../references.md).
`desugar.py` rewrites each `<scheme:...>` into `"..." (yay-proto:reference)`
and checks rule R1; `check.py <binyay 3.0.0>` reads the corpus and the
design's cases with the unmodified Python port and with binyay as YAY and
MEH.

The design's cases now live only in the corpus, each named `reference-*`:
`test/yay` and `test/js` for the accepted cases (each `.yson` became each
port's expected value), `test/meh` for the two read only as MEH, and
`test/nay` with each refusal's message. `cases/` is gone, and `check.py`
holds only on a tree without references, so run it from a checkout of the
design's commit, the one that added `cases/`:

```sh
rev=$(git log --diff-filter=A --format=%H -- doc/design/references/cases | tail -1)
git worktree add /tmp/references-design "$rev"
cd /tmp/references-design
(cd rust && cargo build --release -p binyay)
python3 -I doc/design/references/check.py rust/target/release/yay
```
