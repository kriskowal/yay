# Changelog

## binyay 3.0.0

**Tagged values.** YAY gains a ninth kind of value: a tag, which is any
string, and one payload, which is any value, as Endo's `makeTagged`,
OCapN and CBOR have it. A tag is always in parentheses: after an inline
value with one space (`10 (s)`, `<ff8800ff> (rgba)`, `'X-1' (com.example/sku)`),
or before a block where the block is introduced (`color: (rgb)` over its
items, `- (outer) (inner)`, `(point)` alone on the root's first line). Inside
the parentheses a tag reads as a double-quoted string does, with `\)` in the
place of `\"`; `()` is the empty tag. With several tags the nearest the
payload is innermost. YAY has exactly one space before each tag; MEH accepts
any number, or none, so `yay -f meh -t yay` normalizes tags and
`yay --check -t yay` flags them. YAY gives no tag a meaning and names no
tags. SHON gains `-T tag value`. `-t json` writes a tagged value as the
smallcaps object, which `-f json` reads back as a plain object, so the
lossless round trip is YSON; YAML, TOML and CBOR refuse one.

Every port reads tagged values, as the model its language suits: `Value::Tagged`
in Rust, a frozen `tagged(tag, payload)` in JavaScript, `yay.Tagged` in Go, a
frozen dataclass `Tagged` in Python, a record `Tagged` in Java, `YAY_TAGGED`
in C and a record `tagged` in Scheme, and `yay -t js`, `-t go`, `-t python`,
`-t java`, `-t c` and `-t scheme` write code that builds it. All seven read
the same shared cases and refuse the same malformed tags. The Rust and Python
ports also write tags; only Rust has a SHON reader and a YSON codec.

**References.** YAY gains a tenth kind of value: a reference, a URI or IRI
in angle brackets with its scheme (`<https://example.com/cat.png>`,
`<ocapn://…>`, `<urn:ietf:rfc:3986>`), held as written and never
dereferenced. After `<`, a scheme and a colon make a reference and anything
else is bytes, so `<ff8800>` and `<>` read as before and the bytes typos
`<deadbeefXX>` and `<DEADBEEF>` stay refused. Nothing is escaped: a URI
cannot hold a space, `<`, `>`, `"`, `{`, `}`, `|`, `\`, `^` or a backtick,
so `>` always closes it, and the brackets shield their text, `#` and the
inline delimiters included. The scheme is lowercase; MEH lowercases it, and
reads `a: <https://x/#f>` as a reference where 3.0.0's MEH read the string
`"<https://x/"`. A reference equals another by its text and never equals a
string. A tagged reference (`<ocapn://…> (Counter)`) is a tagged value.
YSON writes a reference as smallcaps writes a remotable, `"$"` and the URI
in place of the slot (`"$https://example.com/"`), and reads `"$"` without a
scheme as an error; `-t json` writes the same string, which `-f json` reads
back as a string. YAML, TOML and CBOR refuse a reference. SHON gains
`-r uri`, which requires a scheme. `yay -t js`, `-t go`, `-t python`,
`-t rust`, `-t c`, `-t java` and `-t scheme` write `reference(uri)`,
`yay.Reference(uri)`, `Reference(uri)`, `Value::Reference(uri)`,
`yay_reference(uri)`, `new Reference(uri)` and `(make-reference uri)`.
The Rust port reads and writes references (`Value::Reference`), YAY, MEH
and YSON. The JavaScript port reads them as a frozen `reference(uri)`, and
its CapData bridge, `ysonToCapData` and `capDataToYson`, moves each distinct
URI of a YSON document into a slot, `$0`, `$1`, ..., for `@endo/marshal`'s
`makeMarshal`, and back. The Python port reads and writes them as a frozen
dataclass `Reference(uri)`. The Go port reads them as `yay.Reference`, a
string type of its own, the C port as `YAY_REFERENCE`, its text in
`data.reference`, the Java port as `record Reference(String uri)`, and
the Scheme port as a record `reference`, `(make-reference uri)`, read with
`reference-uri`.
All seven read the shared cases, `reference-*` in `test/`, and refuse the
malformed ones with the Rust port's messages.

**Fixed: a space after `<` in bytes.** GRAMMAR.md forbids a space after
the opening `<` of inline bytes, as it forbids one before `>`, but the
Rust, Python, Go and Java ports read `a: < ff>` as one byte, and the Rust
port read `[< ff>]` and `{a: < ff>}` too. YAY now refuses it in every port,
in inline arrays and objects as well, as `Unexpected space after "<"`. MEH
still reads it, and `yay -f meh -t yay` writes `a: <ff>`.

**Fixed: bytes as a key.** The Rust port read `<ff>: 1` as an object with
the key `"<ff>"`, and the other ports refused it with differing messages.
Bytes are a value, never a key, so every port now refuses it as
`Unexpected character ":"` at the colon, as it refuses a reference there,
in a list item (`- <ff>: 1`) as well as at the root.

**Fixed: an inline array that holds a colon.** Every port but Python took
a root inline array or a list item's inline array that held a colon, as in
a reference (`[< https://x>]`), for a property, and refused it as a bad
key; Rust, JavaScript and Scheme refused the well-formed `[{a: 1}]` so. An
inline array is never a key, so each now reads it as an inline array, and
`[< https://x>]` is refused as `Unexpected space after "<"`, as everywhere
else.

**Fixed: an inline array as a key.** `[1]: 2` is refused by every port as
`Unexpected character ":"` at the colon. An inline array is a value, never a
key; the bracket is matched, so neither an unmatched-bracket nor an
unexpected-newline error describes it.

**Fixed: bytes as an inline object's key.** `{<ff>: 1}` is refused in
every port as `Invalid key` at the brace, as `{@: 1}` is; JavaScript had
said `Unexpected character ":"` and Python "Expected key, got BYTES".

**Fixed: an inline object as a list item.** The JavaScript, C and Java
ports took a list item that opened an inline object (`- {a: 1}`) for a
block object, because it held a colon, and refused it as a bad key. Each
now reads it as an inline object, and `- {a: 1 }` is refused as
`Unexpected space before "}"`, as in every other port.

**Fixed: columns inside an inline array.** The Rust port placed an inline
object's errors at the inline array's opening bracket, one column before the
object. It now counts the array opener and any preceding items, as every other
port does.

**Fixed: spaces around a nested key's colon.** The Rust and Go ports
checked the space after a property's colon only at the root, so they read
`- a:  1`, `- a : 1` and a nested object's `b:  1` or `b : 1`. Every port
now refuses them, as at the root, with `Unexpected space after ":"` or
`Unexpected space before ":"`.

**Fixed: columns in a list item.** An error in a list item's text was
placed as if the `- ` before it were not there: `- < ff>` reported 1:2
for the space at 1:4, and `- - a` reported the bare word at 1:3, not 1:5.
The Rust, JavaScript, Go, C and Java ports now count the list leader, as
the Python port already did, and the shared cases expect the true column.

**Breaking: two spaces before an inline comment.** The grammar's
inline comment is two spaces and `#` (`inline-comment = SP SP comment`),
but every port took one space after a property's value, and some after a
list item's or a block-bytes line's. A comment after any inline value or
hex line with one space before its `#` is now refused in every port,
`Expected two spaces before a comment` at the `#`. A comment alone after
a block leader (`> # note`, `data: > # note`) is unchanged.
`test/yay/bytearray-block-hex-and-comment.yay` and the README examples
now put two spaces there.

**Fixed: a comment after a list item's value.** The ports read the root
value and list items by their own route, apart from a property's value,
and disagreed on `- 'abc'  # c` (JavaScript and Go read `abc'  # it'`),
`- "abc"  # c`, `- [1]  # c`, `- {a: 1}  # c` and `- <ff>  # c`, and Java
refused `- 42  # c`. Every port now reads every inline value one way,
wherever it stands, and the shared cases cover a comment after each kind.

**Fixed: escapes in single-quoted strings.** `\'` and `\\` are the only
escapes, in values and keys, and any other backslash is literal.
JavaScript and Go kept both backslashes, Python ended the string at `\'`
and took `''` as an escaped quote, which the grammar does not have (a
breaking change for Python documents that relied on it), and the Scheme
port kept `\\` as two. An unclosed single-quoted string is refused,
`Unterminated string` at the line's last character, where JavaScript and
Go read `- 'abc` as `ab`; an unclosed double-quoted property value is
reported the same way.

**Fixed: block bytes.** Uppercase hex is refused in block bytes as it is
inline; JavaScript, Go, C and Java lowercased it.

**Fixed: block strings agree at every depth.** A property's empty block
string is refused at the root, nested and in a list item (JavaScript took
`a: `` at the root as `""`, Rust nested), and so is a comment after a
property's backtick (`a: `  # c`), which JavaScript and Go took at the
root only. The grammar's property block string is the backtick alone.

**Fixed: a lone `"` under a key** is refused, `Unexpected indent`, as any
one quoted line under a key is. JavaScript read it as an old block-string
leader and Scheme as an empty object.

**Fixed: `__proto__` is an ordinary key.** JavaScript set the object's
prototype from a `__proto__` key, block or inline; every property is now
an own data property. Rust's `-t js` writes the key as `["__proto__"]`.

**Breaking: JavaScript references are checked.** `new Reference(uri)` and
`reference(uri)` refuse, with a `TypeError`, a string that is not a
reference's text by the parser's rule (a lowercase scheme, a colon, URI
or IRI characters, well-formed percent-encodings), and `capDataToYson`
refuses such a slot, so CapData round-trips through YSON.

**JavaScript: an inline nesting limit.** Inline arrays and objects nest
at most 256 levels; deeper is refused, `Inline nesting deeper than 256
levels`, where it overflowed the stack with a `RangeError`.

**Breaking: YSON follows smallcaps.** A tagged value is
`{"#tag": tag, "payload": value}`. An integer is `"+42"` or `"-7"`, not
`"#42"`, which YSON no longer reads. An object key that begins with `#`, `*`
or `!` takes the `!` escape (`"!#x"`), so no plain object reads back as a
tagged value.

```bash
echo 42 | yay -t yson
# 2.0: "#42"
# 3.0: "+42"
```

**YSON is smallcaps.** `@endo/marshal`'s smallcaps decodes the YSON
`yay -t yson` writes, tagged values and the `!` escapes included, and
writes it back as the same text; the JavaScript port's tests run live
binyay output through it, in CI. Bytes are written `"*<hex>"`, the form
[endojs/endo#2871](https://github.com/endojs/endo/pull/2871) gives
byteArrays, which `@endo/marshal` 1.10.0 does not yet decode. A reference
decodes through the CapData bridge, which gives its URI a slot.

**Breaking:** a Markdown body ends with a newline. Yaydown (`-f yd`) refuses
a non-empty body without one, and `-t md` and `-t yd` write a `content`
without a final newline in the front matter, not as the body. Two additions
come with it: `-s -` and `-b -` read stdin, and npm installs the native
`yay` as its bin.

**A body ends with a newline.** Before, a body could end without a newline,
and an object whose `content` lacked one wrote that body as is, so
`yay [ --content hi ] -t md` wrote the two bytes `hi`. Such a file is easy to
make by accident and hard to see. Now:

- `-f yd`, and `-f yd --check`, refuse a non-empty body without a final
  newline, naming the line and the fix: `the content does not end with a
  newline; end the file with a newline, or put "content" in the front matter
  as a string`.
- `-f md` (Mehdown) reads such a body and adds the newline to `content`.
- `-t md` and `-t yd` write a `content` that ends with a newline as the body,
  exactly, as before. A non-empty `content` without one stays in the front
  matter as a quoted string, with or without other properties.

```bash
yay [ --content hi ] -t md
# 2.0: hi (no newline)
# 3.0:
# ---
# content: "hi"
# ---

yay [ --content $'hi\n' ] -t md
# 2.0 and 3.0: hi, then a newline
```

The mapping between objects and documents stays exact both ways, and Mehdown
and Yaydown still read every document Yaydown accepts as the same object. A
front matter `content` without a final newline now stays in the front matter
when `-f md -t md` normalizes a document; one with a final newline still
moves to the body.

Migration: a document that 2.0 read under `-f yd` and 3.0 refuses is fixed
by adding the final newline (`yay -f md -t yd F` writes it), and that adds
`\n` to its `content`. A program that writes `content` meant as a body
should end it with a newline.

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

**Versions.** Each library that gains tagged values gains a new kind of
value, which breaks code that handles every kind (an exhaustive `match` over
Rust's `Value`, which is not `#[non_exhaustive]`), so each is a major
version:

| Package | 2.0.0 release | 3.0.0 release |
|---------|---------------|---------------|
| binyay (crate, npm and its platform packages) | 2.0.0 | 3.0.0 |
| libyay (Rust crate) | 1.1.1 | 2.0.0 |
| libyay (npm) | 1.1.1 | 2.0.0 |
| libyay (Python) | 1.1.1 | 2.0.0 |
| yay (Java, `com.kriskowal:yay`) | 1.1.1 | 2.0.0 |

The Go, C and Scheme ports carry no version of their own: they are released
with the repository's tag.

The Rust `libyay` also gains, for SHON: `-` as stdin, `-T`, `parse_shon`,
which takes any SHON trigger, and `scan_shon`, which counts an expression's
reads of stdin without reading anything. The final newline is binyay's alone
(its Markdown reader and writer), so it does not touch libyay.

**Not yet proved.** The native `bin/yay` install was tested on Linux only,
with packages packed from this release and installed into a scratch prefix.
The case it exists for, a global install on macOS, waits on a check on a Mac.
Windows keeps the Node wrapper and is not tested.

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
