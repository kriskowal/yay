# Changelog

## binyay 2.1.0

Two additions; nothing is removed or changed in meaning.

**`-s -` and `-b -` read stdin.** In a SHON expression, the path `-` after
`-s` reads stdin as a UTF-8 string, and after `-b` as raw bytes, so a pipe
can supply one value of a document built on the command line:

```bash
echo hi | yay [ --content -s - ] -t md
```

Stdin can be read only once: a command line that names `-` after `-s` or
`-b` more than once, or that also names `-` as its input (`yay - -s -`), is
refused before anything is read. A file named `-` is reached as `./-`.
Before, `-s -` failed with `Cannot read file '-'`.

**npm installs the native `yay` as its bin, as esbuild does.** binyay's
install script checks that the platform package's binary reports binyay's
version and puts it at `bin/yay`, in place of the Node wrapper that ran it,
so the `yay` command starts no Node process. On macOS that wrapper lost
every file descriptor beyond stdio, so `yay -s <(echo hi)` failed with
`Bad file descriptor` from a global install; it now works. If the step
fails, or on Windows, under Yarn or with `--ignore-scripts`, the wrapper
stays, with one warning, and installation still succeeds. `npx binyay` on
macOS still cannot pass `<(…)`, since npm's own launcher drops the
descriptor: use `-s -`.

libyay (Rust) is 1.2.0: SHON reading treats `-` as stdin, and gains
`parse_shon`, which takes any SHON trigger, and `scan_shon`, which counts an
expression's reads of stdin without reading anything. The other ports have
no SHON reader, do not change, and stay at 1.1.1.

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
