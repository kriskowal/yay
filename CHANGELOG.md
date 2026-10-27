# Changelog

## binyay 2.0.0

**Breaking:** `--canonical` is removed. `--check` takes its place when `-t`
is given.

`--canonical` checked that `-t yay` (or `-t md`, `-t yd`) would write a
document unchanged. That is a formatting check, not canonicalization: two
documents with the same value can both pass and still differ in key order,
quoting or comments. The check is now spelled with the output format it
compares against, and `--check` means one of two things:

- `--check [-f F]`, without `-t`: check that the input is valid, strict YAY
  by default. Unchanged.
- `--check [-f F] -t T`: check that the input is valid and that writing it as
  `T` leaves it unchanged, byte for byte, naming the first line that differs.
  `-f` defaults to strict `yay`. The pairs are `-f yay` or `meh` with `-t yay`,
  and `-f md` or `yd` with `-t md` or `yd`; any other pair is refused with an
  error.

Migration:

| 1.x | 2.0 |
|-----|-----|
| `yay --canonical F` | `yay --check -t yay F` |
| `yay --canonical -f meh F` | `yay --check -f meh -t yay F` |
| `yay --canonical -f yd F` | `yay --check -f yd -t yd F` |
| `yay --canonical -f md F` | `yay --check -f md -t md F` |

Also: a document missing only its final newline is reported as such, rather
than as `expected "a: 1", found "a: 1"`, and the messages say "not formatted"
for "not canonical".

libyay and the other ports are unchanged, and stay at 1.1.1.
