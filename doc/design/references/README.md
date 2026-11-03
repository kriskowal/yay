# References prototype

Throwaway, not a port: the evidence for [references.md](../references.md).
`desugar.py` rewrites each `<scheme:...>` into `"..." (yay-proto:reference)`
and checks rule R1; `check.py <binyay 3.0.0>` reads the corpus and
`cases/` with the unmodified Python port and with binyay as YAY and MEH.

```sh
(cd rust && cargo build --release -p binyay)
python3 -I doc/design/references/check.py rust/target/release/yay
```
