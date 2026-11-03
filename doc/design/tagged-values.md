# Tagged values

Status: proposal, 2026-11-01. Revised 2026-11-02: Kris chose option E,
and YAY Schema held as Endo patterns. Revised 2026-11-03, against binyay
2.0.0, for the 3.0.0 line: Kris walked back E's bare-word tags for option F,
a simple name fused after an inline value, any other name in parentheses
after it, and a parenthesized tag introducing a block. The open questions
are F's details, each with a recommendation; the choices are the
maintainer's.

## Problem

YAY has eight kinds of value: null, boolean, integer, float, string, bytes,
array and object.
A configuration file that means "ten seconds", "the first of November" or
"this shade of blue" has to say it with a string or a number and leave the
meaning to a convention outside the document: `timeout: 10` does not say
its unit, and `color: "ffffff"` does not say it is a color.

OCapN and Endo already have the missing kind.
Endo's `makeTagged(tag, payload)` makes a value whose `passStyleOf` is
`"tagged"`, and `@endo/marshal`'s smallcaps encoding writes it as
`{"#tag": tag, "payload": payload}`.
OCapN's data model has the same thing: a tag and one payload.
CBOR has had it from the start (RFC 8949 § 3.4: major type 6, a number and
one data item), and its diagnostic notation writes it `0("2026-11-01T…Z")`.
YSON is already a subset of smallcaps, so YAY is one kind away from carrying
all of Endo's passable data that is not a capability, a promise or an error.

This document proposes the model, five written forms and their mappings,
and a first set of conventional tags: dates, times, durations and the web's
colors.

## Scope

In scope: the data model, the written form in YAY and MEH, YSON, JSON,
SHON, binyay's outputs, YAY Schema's keyword, and the conventional tags.

Not in scope: units in general (lengths, masses, currencies), arithmetic on
tagged values, CSS color expressions (`color-mix()`, relative colors,
`light-dark()`), and custom color profiles (`@color-profile`).
Capabilities, promises and errors stay outside YAY.

## 1. The model

A **tagged value** is a pair of a **tag** and one **payload**.

- **The tag is a string.** Endo requires a string (`makeTagged` refuses
  anything else) and OCapN's tag is a string as well (its `Model.md`
  § Tagged: "The tag must be a String"; Endo's OCapN codec writes it on the
  wire as a selector), so any narrower model would fail to round-trip with
  them.
  The written forms below spell a tag bare when it is an *identifier*
  (`tag-name`, below) and quoted otherwise, so the model stays as wide as
  Endo's while ordinary documents stay plain.
- **The payload is any YAY value,** a tagged value included, so tags nest:
  `!utc !datetime "…"` is a tagged value whose payload is tagged.
  Endo allows nesting, and CBOR does (tag 55799, the self-described CBOR
  marker, routinely wraps another tag).
- **Namespaces by convention, not by syntax.** Tags with no dot or colon
  (`date`, `rgb`, `s`) are reserved to this specification, which defines the
  conventional set (§ 4). Others name their owner, reversed-domain style:
  `com.example.sku`. Endo's own tags (`copySet`, `copyBag`, `copyMap`,
  `match:…`) are grandfathered: they pass through unchanged and their meaning
  is Endo's.
- **Equality and hashing.** Two tagged values are equal when their tags are
  equal strings and their payloads are equal YAY values. A tagged value never
  equals its payload, nor any value of another kind: `!s 10` is not `10`, and
  `!s 60` is not `!m 1`. A tag is not a unit conversion. Hash the tag and the
  payload together with a kind marker that no other kind uses.
- **Ordering**, where a port orders values (Endo's `rankOrder` does), puts
  tagged after the other kinds, then by tag, then by payload, as `rankOrder`
  does.
- **Round trip.** YAY tagged ↔ Endo `makeTagged(tag, payload)` ↔ smallcaps
  `{"#tag": tag, "payload": …}` ↔ OCapN tagged, losslessly, for every tag
  and every payload YAY can hold.
  Endo payloads YAY cannot hold (`undefined`, remotables, promises, errors)
  are refused on the way in, as they are today.

The data model table in `GRAMMAR.md` grows a ninth row:

| Type | Description |
|------|-------------|
| Tagged | A tag (a string) and one payload (any value) |

## 2. Written forms

Options A to D share the tag's lexical rule (E read keywords and strings
as tags by position; F has a fused simple name and parentheses for the
rest, § F):

```
tag-name   = tag-start { tag-char } ;
tag-start  = "a".."z" | "A".."Z" | "_" ;
tag-char   = tag-start | "0".."9" | "-" | "." | ":" ;
tag        = tag-name | double-quoted ;
```

A tag that is not a `tag-name` (an empty string, one with a space) is
written as a double-quoted string in the tag position.
`null`, `true`, `false`, `nan` and `infinity` are not refused as tags,
but the position always disambiguates them from the keywords.

The examples use one value of each kind of payload:

| Payload | Meaning |
|---|---|
| integer | `10` seconds |
| float | `1.5` milliseconds |
| string | the date `"2026-11-01"` |
| bytes | the color `<ff8800>` |
| array | the color `[0.7, 0.1, 200]` in OKLCH |
| object | a `com.example.point` with `x` and `y` |
| block string | a Markdown note |
| block bytes | a PNG thumbnail |
| tagged | an instant whose payload is itself tagged |

### A. Call form

The CBOR diagnostic style, and CSS's function style: the tag, then the
payload in parentheses, with no space between.

```yay
timeout: s(10)
debounce: ms(1.5)
due: date("2026-11-01")
accent: rgb(<ff8800>)
ink: oklch(0.7, 0.1, 200)
at: com.example.point({x: 1, y: 2})
pair: [s(10), m(20)]
nested: utc(datetime("2026-11-01T15:00:00"))
```

**Argument-list sugar.** CSS writes `oklch(70% 0.1 200)` with several
arguments, and YAY's call has one payload.
Without sugar the array payload doubles its brackets, `oklch([0.7, 0.1,
200])`.
With it, two or more comma-separated arguments are the array:
`oklch(0.7, 0.1, 200)` is `oklch([0.7, 0.1, 200])`.
One argument is the payload itself, so a one-element array keeps its
brackets, `f([1])`, and `f()` is refused (write `f([])` or `f(null)`).
The canonical form uses the sugar for arrays of two or more items.

**Blocks.** A call must close, and YAY closes nothing else.
The block form opens the call at the end of the property line and closes it
alone at the property's indent:

```yay
note: markdown(
  `
    # Release notes

    Tagged values arrive.
)
thumb: image.png(
  > 89 50 4e 47 0d 0a 1a 0a
    00 00 00 0d 49 48 44 52
)
point: com.example.point(
  x: 1
  y: 2
)
```

Grammar sketch, against `GRAMMAR.md`:

```
value⟨n⟩       =/ tagged-call⟨n⟩ ;
inline-value   =/ inline-call ;
inline-call    = tag "(" inline-value { "," SP inline-value } ")" ;
tagged-call⟨n⟩ = inline-call
               | tag "(" LF nested-content⟨n⟩ INDENT(n) ")" ;
```

The root-object rule must learn that `tag(` at indent 0 is a value, not a
key: a colon inside the parentheses (`time("15:00")`) is inside quotes
already, but `com.example:thing(…)` puts one outside them.
The fix is to try `tag-name "("` before the colon scan.

Costs:

- **Familiar.** It reads as CSS reads (`rgb(…)`, `oklch(…)`) and as CBOR's
  diagnostic notation does.
- **Blocks need a closing line,** the only closer in YAY, and a reader has
  to match it by indent.
  A tagged value nested in a tagged block grows a ladder of `)` lines.
- **Looks like computation.** `max(1, 2)` reads as a call that returns 2.
  A configuration language whose values look like calls invites people to
  expect calls.
- **Ports:** a new token after a bare word, a paren-matching state for the
  block form, and the root-object rule above.

### B. Suffix labels

The unit after the number, as CSS (`10s`, `200ms`), Go and SOML (`1h30m`)
write durations.

```yay
timeout: 10s
debounce: 1.5ms
window: 20m
grouped: 10 000ms
```

A suffix is `tag-name` immediately after the last digit, with no space.
The longest number wins, then the suffix: `1e5m` is `m(100000.0)`, and
`1em` is `em(1)` because an exponent needs a digit.
`nan` and `infinity` take no suffix.

**On anything but a number it fails.**
On a string, `"ffffffff"rgba` puts the meaning after the payload, where a
reader of a long string finds it last, and on a concatenated string it is
unclear which line carries it.
On bytes (`<ff8800>rgb`), arrays and objects it is no better.
On a block there is no place for it: the block's last line is content.
So a suffix can only ever be sugar for another form, for numbers.

**The open space Kris named.** If any `tag-name` can be a suffix, `m` means
minutes to Go and meters to everyone else, and `10d` could be days or a
pixel density.
A closed set fixes the meaning but makes every new unit a change to the
specification.

Grammar sketch:

```
value⟨n⟩     =/ suffixed ;
inline-value =/ suffixed ;
suffixed     = ( integer | decimal-float ) unit ;
unit         = /* one of the registered units, § 4.3 */ ;
```

Costs: the most readable form for durations, and the weakest for anything
else. As the only form it cannot carry dates, colors or Endo's tags.
Ports: the number scanner reads on into letters and looks the unit up.

### C. Prefix tags

Considered and rejected (2026-11-02, for E, and again for F): it puts the
meaning before the payload behind a sigil (`!rgb <ff8800>`), where Kris's
forms write the payload first, as CSS does (`10s`).

YAML's `!tag value`, without YAML's `!!` or its URI resolution: the tag,
one space, then any value, a block included.

```yay
timeout: !s 10
debounce: !ms 1.5
due: !date "2026-11-01"
accent: !rgb <ff8800>
ink: !oklch [0.7, 0.1, 200]
at: !com.example.point {x: 1, y: 2}
pair: [!s 10, !m 20]
nested: !utc !datetime "2026-11-01T15:00:00"
note: !markdown `
  # Release notes

  Tagged values arrive.

thumb: !image.png >
  89 50 4e 47 0d 0a 1a 0a
  00 00 00 0d 49 48 44 52
point: !com.example.point
  x: 1
  y: 2
list:
- !date "2026-11-01"
- !s 10
```

The block forms keep the property rules they already have: the block
introducer stays last on the line (`` !markdown ` ``, `!image.png >`), and a
tag with nothing after it takes its payload from the nested content below,
exactly as `key:` does.

Grammar sketch:

```
value⟨n⟩          =/ "!" tag SP value⟨n⟩ ;
inline-value      =/ "!" tag SP inline-value ;
property-value⟨n⟩ =/ SP "!" tag SP block-string-prop⟨n⟩
                   | SP "!" tag SP block-bytes-prop⟨n⟩
                   | SP "!" tag [ inline-comment ] LF nested-content⟨n⟩ ;
item-value⟨n⟩     =/ "!" tag SP item-value⟨n⟩
                   | "!" tag LF nested-content⟨n⟩ ;
```

Costs:

- **Reaches every payload,** blocks and nesting included, and closes
  nothing, which is how the rest of YAY works.
- **YAML's baggage.** A YAML reader expects `!!str` and global tags, and
  YAML parsers that see a YAY file will misread `!s 10` as a local tag they
  do not know. YAY already is not YAML, and the strict grammar would refuse
  `!!`.
- **Less familiar for colors** than `rgb(…)`: `!oklch [0.7, 0.1, 200]`
  carries its brackets.
- **Ports:** one new leading character in the value scanner, which `!`
  takes today in no position, and the tag rule.

### D. Hybrid: one general form, suffixes for durations

Considered and rejected (2026-11-02): C's prefix sigil for all but the
units; E and then F keep its units and drop the sigil.

C (or A) for every tagged value, plus the suffix form of B for a closed
registry of duration units on numbers, which are sugar: `10s` *is* `!s 10`,
and the two are the same value.

```yay
timeout: 10s
debounce: 1.5ms
due: !date "2026-11-01"
accent: !rgb <ff8800>
note: !markdown `
  Tagged values arrive.
```

The canonical form writes the sugar wherever it applies (a registered unit
on a number) and the general form everywhere else, so there is still one
spelling per value.

### E. Bare or string tags: after an inline value, before a block

Chosen 2026-11-02, then walked back 2026-11-03 in favor of F. A bare word
(or a quoted string) after an inline value and alone before a block, with
glued units. Considered and rejected: a bare word cannot be told from a
value without looking at the next line, so a misindented sibling
(`enabled: true` over `  retries: 3`) silently became a tagged block, and
keywords and strings in tag position needed a writer rule and a lint to
answer it. Kris, 2026-11-03: "There are too many hazards to a bare word for
tags." E's text, prototype and cases are at `7df077b` on
`design-tagged-values`.

### F. Fused names inline, parentheses before a block (chosen)

Kris's form, confirmed 2026-11-03: "Let's use parentheses for the
introduction of a tagged block and keep the fused suffix notation for
inline only. We can also use parentheses in suffix for any name that
contains a closing parenthesis and otherwise treat them like strings", and
"let's put a space before the open paren, so not fused when parenthesized,
in normal form."

```yay
timeout: 10s
debounce: 1.5ms
due: '2026-11-01'date
accent: <ff8800ff>rgba
ink: [0.7, 0.1, 200]oklch
at: {x: 1, y: 2} (com.example.point)
nested: '2026-11-01T15:00:00'datetime (utc)
sku: 'X-1' (com.example/sku)
odd: 3 ('a)b')
flag: true (t)
note: (markdown) `
  # Release notes

  Tagged values arrive.

thumb: (image/png) >
  89 50 4e 47 0d 0a 1a 0a
color: (rgb)
  - 255
  - 136
  - 0
patterns:
- (match:splitRecord)
  name: 'x'
```

**Inline: a tag follows its value.**

- **A fused simple name.** A *simple name* directly after a value, with no
  space, is its tag: `10s`, `'2026-11-01'date`, `<ff8800ff>rgba`,
  `[255, 136, 0]rgb`, `{x: 1}point`. It may follow any inline value but a
  keyword. A simple name is an ASCII letter or `_`, then letters, digits and
  `_`:

  ```
  simple-name = ( letter | "_" ) { letter | digit | "_" } ;
  ```

  What it leaves out keeps everything else readable: no `.` (a decimal point,
  and `1.5e3` is one float), no `-` or `+` (`-3h`, an exponent's sign), no
  `:` (a key's colon), no `/`, no `,`, `]`, `}` (inline separators), no `#`
  (comments), no space, and no leading digit. Two more rules guard the
  numbers: the longest number is read first, so `1e5m` is `m(100000.0)`,
  and after a number a fused name may not begin with `e` or `E` (`10em` is
  refused, write `10 (em)`); after the integer `0` it may not begin with
  `x`, `o` or `b` (`0xff` is refused, not `xff(0)`; YAY has no radix
  prefixes, question 3). No fused name on a keyword: `nanms` and `truex`
  are words, refused as today; write `nan (ms)`, `true (t)`.
- **A parenthesized name, for any other.** One space, then the tag in
  parentheses: `'X-1' (com.example/sku)`, `{} (match:any)`, `1 (my tag)`.
  Inside the parentheses the tag is **raw text up to the first `)`**: spaces
  inside are kept, and `#`, `\`, `'`, `"` after the first character, `,`,
  `]`, `}` and `(` are ordinary characters (no comment, no escape). A raw
  tag may not be empty, begin or end with a space, begin with a quote, or
  hold a tab or a line break.
- **A quoted name in parentheses**, for a tag a raw one cannot spell
  (one with `)`, the empty tag, edge spaces, control characters): one YAY
  string, either quote, with its escapes, closed by `)` at once:
  `3 ('a)b')`, `1 ('')`, `2 ("tab\there")`.
- **A fused `(`** (`'X-1'(com.example/sku)`) is accepted, and the
  formatter writes it spaced, so `yay --check -t yay` flags it, as it
  flags any other unformatted text (question 2).
- **Several tags**: at most one fused name, the innermost, then each
  further tag one space and parenthesized, innermost first:
  `'2026-11-01T15:00:00'datetime (utc)` is `utc(datetime(…))`, and
  `[1, 2] (a) (b)` is `b(a([1, 2]))`. A fused name after a `)` (`1 (t)s`) is
  refused.
- **Inside inline arrays and objects** the same: `[10s, 'ff'x, [1] (a, b)]`,
  `{c: <ff8800>rgb, e: 'v' (x}y)}`. The separators keep their rule
  (exactly `, `), and parentheses shield what is inside them.
- **Comments** follow the tags: `a: 'x' (t)  # note`. Two spaces before `(`
  are refused, as two spaces before anything but `#` are today.

**Block: a parenthesized tag introduces it.** Where a block is introduced,
the tag in parentheses stands alone, or before the block's introducer, and
the block is below:

- `key: (tag)` over the nested content, as `key:` is today: an object, an
  array, or a concatenated string of two or more quoted lines.
- `- (tag)` over the item's content at the dash's indent plus two.
- `` key: (tag) ` `` and `key: (tag) >` for block strings and bytes, the
  backtick alone at the end of a property's line as today; an item takes
  text after it on the same line (`` - (markdown) ` First line. ``,
  `- (sha256) > 89 50`).
- **At the root**, `(tag)` alone on the first content line, the block below
  at column 0, as a root block sits today (question 5):
  `(point)` over `x: 1` and `y: 2`.
- **Several tags**, outermost first, nearest the block innermost:
  `- (outer) (inner)` over a block is `outer(inner(…))`.
- A comment may follow: `color: (rgb)  # r, g, b`.
- An inline value never takes a prefix (`a: (rgb) [255, 136, 0]` is
  refused), and a `(tag)` with no block below (`a: (rgb)` over a sibling) is
  refused: a tag needs a payload, and a tag is never a value.

**Why it is unambiguous.** In YAY 2.0.0 nothing may follow a complete
inline value but a comment, no value begins with `(`, and no letter may
follow a number, a string, bytes, `]` or `}`. F gives only those refused
texts a meaning. A `(` at the start of a property's or item's value is
always a block's tag, a `(` after a value always a suffix, so no reader
looks ahead to tell a tag from a value: it looks ahead only, as for `key:`
today, to find the block. The misindented sibling stays an error. The
prototype shows it: it changes none of 2.0.0's 136 valid documents and
makes none of its 91 refused ones valid ([tagged-values-f.md](tagged-values-f.md)).

**Units.** The six of § 4.2 are ordinary fused names now, not a closed
grammar set: `10s` is `s(10)` as `10px` is `px(10)`. The closed set moves
to the conventional tags, where a schema or a lint can say that `px` is
unknown; the grammar needs no list (question 4).

**Normal form**, as the formatter writes it and `--check -t yay` requires:
a simple name fused to its value; any other tag after one space,
parenthesized, raw unless it must be quoted (it holds `)`, is empty,
begins with a quote, has an edge space or a character a raw tag cannot
hold), quoted then as the formatter quotes any string; the block form
exactly when the payload would be written as a block in that position, so
`[255, 136, 0]rgb` when it fits the line and `(rgb)` over its items when it
does not.

**The ports' cost.** One new character, `(`, at two places: after a value
(a suffix) and at the head of a property's or item's value (a block's tag),
and a fused-name rule after each value. The key test needs no change, since
a `(` begins no key. The root-object test ("the first line at indent 0
contains a colon outside quotes") learns to skip parenthesized text as it
skips quotes, so `(match:splitRecord)` and `'x' (a: b)` at the root are
not root objects.


### Others considered

- **EDN's `#tag value`.** Considered and rejected: `#` begins a comment in
  YAY, and `# date "…"` already means something.
- **A reserved object key,** `{"#tag": "rgb", payload: <ff8800>}`, smallcaps
  verbatim, with no new syntax. Considered and rejected as the written
  form: it is verbose, and an object can no longer have that key freely.
  It is the right *JSON* form (§ 3).
- **A tag glued to a string,** `date"2026-11-01"`, after JavaScript's
  tagged templates and Python's string prefixes. Considered and rejected: it
  reaches strings only, and dates are the one place it reads well.
- **Typed keys,** `timeout(s): 10`, after TOML-adjacent proposals. Considered
  and rejected: the tag belongs to the value, so an array of them cannot be
  written.

### Side by side

| Payload | A. call | B. suffix | C. prefix | D. C + units | E. suffix inline, prefix block | F. fused or parenthesized after, parenthesized before a block |
|---|---|---|---|---|---|---|
| integer | `s(10)` | `10s` | `!s 10` | `10s` | `10s` | `10s` |
| float | `ms(1.5)` | `1.5ms` | `!ms 1.5` | `1.5ms` | `1.5ms` | `1.5ms` |
| string | `date("2026-11-01")` | none | `!date "2026-11-01"` | same as C | `"2026-11-01" date` | `"2026-11-01"date` |
| bytes | `rgb(<ff8800>)` | none | `!rgb <ff8800>` | same as C | `<ff8800> rgb` | `<ff8800>rgb` |
| array | `oklch(0.7, 0.1, 200)` | none | `!oklch [0.7, 0.1, 200]` | same as C | `[0.7, 0.1, 200] oklch` | `[0.7, 0.1, 200]oklch` |
| object | `p({x: 1, y: 2})` | none | `!p {x: 1, y: 2}` | same as C | `{x: 1, y: 2} p` | `{x: 1, y: 2}p` |
| block string | `` markdown( `…` ) `` with a closing line | none | `` !markdown ` `` | same as C | `` markdown ` `` | `` (markdown) ` `` |
| block bytes | `image.png( > … )` with a closing line | none | `!image.png >` | same as C | `image.png >` | `(image/png) >` |
| block array | `rgb(` over the items, then `)` | none | `!rgb` over the items | same as C | `rgb` over the items | `(rgb)` over the items |
| tagged | `utc(datetime("…"))` | none | `!utc !datetime "…"` | same as C | `"…" datetime utc` | `"…"datetime (utc)` |
| any tag | `"my tag"("…")` | none | `!"my tag" "…"` | same as C | `"…" "my tag"` | `"…" (my tag)` |

## 3. Mappings

The mappings are the same for every written form: they carry the model,
not the syntax.

**YSON** writes a tagged value as smallcaps does,
`{"#tag": "rgb", "payload": "*ff8800"}`, with the tag and the payload
encoded as YSON encodes any string and value.
This keeps YSON a subset of smallcaps, as the README says it is.
It also needs a fix that is due regardless: YSON writes object keys
verbatim today, so the YAY object `{"#tag": "x", payload: 1}` becomes YSON
that reads back as an object while smallcaps reads it as tagged, and a key
`#x` makes YSON that smallcaps refuses (binyay 1.1.0, checked 2026-11-01).
Smallcaps encodes property names as strings, so YSON must write a key that
begins with `#`, `*` or `!` with the `!` escape it already gives strings.
That changes the output for such keys: a major version, by the project's
rule on breaking changes.

**JSON** cannot hold a tagged value. `yay -t json` writes the smallcaps
object, as it writes the other kinds JSON lacks only where it must, and
says so. A JSON import (`-f json`) reads `{"#tag": …, "payload": …}` as a
plain object. A **schema-guided** import restores tagged values, as the YAY
Schema design does for integers and bytes: at a site whose schema admits
exactly one tag (`tag: "s"`), a bare payload (`10`) becomes `!s 10`; at a
site that admits tagged values at all, the smallcaps object becomes the
tagged value; and conventional tags with a string payload accept the bare
string where the schema names the tag (`"2026-11-01"` at a `date` site).

**SHON** gains one reserved flag, `-T`, which takes the next token as the
tag and the token after as the payload: `yay [ --timeout -T s 10 --due -T
date 2026-11-01 ]`. A payload that is an array or object is a bracket
group: `-T oklch [ 0.7 0.1 200 ]`. The suffix sugar of B and D applies to
SHON's number tokens too, so `--timeout 10s` works where the units are
registered; `10s` is a string token today, so that is a change in meaning
for such tokens and belongs in the same major version.

**binyay's outputs.** Each target gains a tagged type, and the generated
code builds it:

| Target | Tagged as |
|---|---|
| `js` | `{tag, payload}` from a small `tagged(tag, payload)` helper, frozen; the generated module does not import `@endo/pass-style`, and a caller that wants Endo's passable wraps it with `makeTagged` |
| `go` | `yay.Tagged{Tag string; Payload any}` |
| `python` | a frozen dataclass `Tagged(tag: str, payload)` |
| `rust` | `Value::Tagged(String, Box<Value>)` |
| `c` | `YAY_TAGGED`, with `tag` and `payload` fields in the value union |
| `java` | `record Tagged(String tag, Object payload)` |
| `scheme` | a record type `tagged` with `tagged-tag` and `tagged-payload` |
| `md`, `yd` | front matter holds tagged values as YAY does |

Each port changes its parser, its writer (normal form), its value model,
its equality, its YSON reader and writer, its code generator, and its
corpus: the shared fixtures under `test/` gain a tagged case per payload
kind and per conventional tag, and `nay` cases for the refusals (`f()`, `!!`,
a suffix on a string, an unregistered unit).

**The dev data editor** (`lib/yay/model.js`) gains a ninth kind,
`tagged`, as a `YayTagged` class with `tag` and `payload`, read and written
by `read.js`, `write.js` and `json.js` as above. The editor shows the tag
as a chip before the payload's own editor, `c` on the chip changes the tag
and keeps the payload, and a conventional tag brings its own editor (a date
picker, the color editor) with the raw payload a keystroke away.

**YAY Schema** gains a type and two keywords:

| Keyword | Value | Meaning |
|---|---|---|
| `type: tagged` | | admits tagged values |
| `tag` | a string, or a string schema (`{enum: [s, ms, m]}`) | the tags admitted |
| `payload` | a schema | what the payload must be |

A conventional tag (§ 4) validates its own payload, so `{type: tagged,
tag: date}` is enough for a date. The duration union Kris named is
`{type: tagged, tag: {enum: [ns, us, ms, s, m, h]}, payload: {type:
number, minimum: 0}}`, and the editor offers its branches by tag as it does
for a discriminated object union.

## 4. Conventional tags

The written examples use C, with D's sugar for durations. In A,
`!date "2026-11-01"` is `date("2026-11-01")` and `!oklch [0.7, 0.1, 200]`
is `oklch(0.7, 0.1, 200)`; in F, the chosen form, they are
`"2026-11-01"date` and `[0.7, 0.1, 200]oklch`.

### 4.1 Dates and times

Payloads are strings in the standard syntax, as CBOR's tag 0 (RFC 3339)
and tag 1004 (RFC 8943, full-date) are, so they read as written and
validate with a grammar, not a calendar library.
RFC 3339's `partial-time` requires seconds (`15:00:00`, not `15:00`), and
RFC 9557's `date-time-ext` imports RFC 3339's `date-time`, so every time
payload below carries seconds; the shorter forms Temporal and ISO 8601
accept are not RFC 3339, and are refused.
Each corresponds to a Temporal type, which is how the JavaScript ports and
the editor hold it. Temporal is at Stage 4 and shipping (Firefox 139,
Chrome 144, Node 26), so the mapping leans on no proposal.

| Tag | Payload | Example | Temporal |
|---|---|---|---|
| `date` | RFC 3339 `full-date` | `!date "2026-11-01"` | `PlainDate` |
| `time` | RFC 3339 `partial-time`, no offset | `!time "15:00:00"` | `PlainTime` |
| `datetime` | `full-date "T" partial-time`, no offset | `!datetime "2026-11-01T15:00:00"` | `PlainDateTime` |
| `instant` | RFC 3339 `date-time`, with `Z` or an offset | `!instant "2026-11-01T15:00:00Z"` | `Instant` |
| `zoned` | RFC 9557 (IXDTF): `date-time` with a `[time-zone]` suffix | `!zoned "2026-11-01T08:00:00-07:00[America/Los_Angeles]"` | `ZonedDateTime` |
| `duration` | ISO 8601 duration, as Temporal reads it | `!duration "P1Y2M3DT4H"` | `Duration` |

Validation: the grammar of the cited production, then the calendar
(`2026-02-30` is refused), with seconds `00` to `60` for a leap second
only where RFC 3339 allows it. Temporal does not keep leap seconds: it
reads `:60` as `:59`, so a leap-second payload does not survive a trip
through Temporal (question 13). `instant` keeps the offset as written: two
instants with different offsets for the same moment are different values,
because the model compares payloads, and a schema or an application that
wants "the same moment" compares them as Temporal does.
The payload is kept as written, case and precision: the formatter does not
rewrite string payloads.

### 4.2 Durations as a discriminated union

Kris's `s(10)` and `m(20)`: each unit is its own tag whose payload is a
non-negative number (integer or float), and a duration-typed site admits
any of them. The tag says the unit; nothing is normalized, so `!s 60` and
`!m 1` are different values that mean the same span.

| Tag | Unit | Fused, in B, D, E and F |
|---|---|---|
| `ns` | nanosecond | `250ns` |
| `us` | microsecond | `250us` |
| `ms` | millisecond | `1.5ms` |
| `s` | second | `10s` |
| `m` | minute | `20m` |
| `h` | hour | `2h` |

`m` is minute, as in Go and SOML, which is safe only because the conventional
set is closed: there is no meter, and months are `!duration "P1M"`.
`d` (day) is left out, as Go and SOML leave it out, because a calendar day
is not 24 hours across a daylight-saving change; `!duration "P1D"` says a
calendar day, and `!h 24` says 24 hours.
Compound spellings (`1h30m`) are not in the first set: `!duration
"PT1H30M"` says them already.
CBOR's tag 1002 (RFC 9581) is always a map, structurally a tag 1001
extended time: its key 1 holds seconds, which `!s` matches, and its other
keys add decimal fractions of a second, a timescale and clock quality, not
other units. RFC 9581 has no ISO 8601 durations, and its tag 1003, a period,
is an array of start, end and an optional duration. Nothing in CBOR is a
tag per unit, so the mapping to CBOR is through key 1 alone.

### 4.3 Colors

Every color space CSS Color 4 defines, with alpha in each. Payloads are
numbers in CSS's own number forms (not percentages: YAY has no `%`), so a
value reads across to CSS by moving the numbers, and `null` stands for CSS's
`none`, a missing component. Alpha is optional, last, `0` to `1`, or `null`,
since CSS allows `none` for alpha too. No CBOR tag for colors is registered
(IANA, checked 2026-11-01), so these have no CBOR counterpart.

| Tag | Payload | Example |
|---|---|---|
| `rgb` | bytes, 3: `r g b`, 8 bits each (CSS `#rrggbb`) | `<ff8800>rgb` |
| `rgba` | bytes, 4: `r g b a`, 8 bits each (CSS `#rrggbbaa`) | `<ff8800ff>rgba` |
| `rgb` | array, 3 numbers: `r g b` from 0 to 255 (question 7) | `[255, 136, 0]rgb` |
| `rgba` | array, 4 numbers: `r g b` from 0 to 255, `a` from 0 to 1 | `[255, 136, 0, 0.5]rgba` |
| `hsl` | `[h, s, l]` or with alpha: degrees, 0 to 100, 0 to 100 | `!hsl [32, 100, 50]` |
| `hwb` | `[h, w, b]` or with alpha: degrees, 0 to 100, 0 to 100 | `!hwb [32, 0, 0]` |
| `lab` | `[L, a, b]` or with alpha: 0 to 100, about ±125, about ±125 | `!lab [70, 30, 70]` |
| `lch` | `[L, C, h]` or with alpha: 0 to 100, 0 to about 150, degrees | `!lch [70, 75, 66]` |
| `oklab` | `[L, a, b]` or with alpha: 0 to 1, about ±0.4, about ±0.4 | `!oklab [0.75, 0.08, 0.15]` |
| `oklch` | `[L, C, h]` or with alpha: 0 to 1, 0 to about 0.4, degrees | `!oklch [0.7, 0.1, 200]` |
| `color` | `[space, c1, c2, c3]` or with alpha; `space` one of `srgb`, `srgb-linear`, `display-p3`, `display-p3-linear`, `a98-rgb`, `prophoto-rgb`, `rec2020`, `xyz`, `xyz-d50`, `xyz-d65` | `!color ["display-p3", 1, 0.5, 0]` |

Validation: the array's length (3, or 4 with alpha), each component a
number or `null`, alpha in `0..1` or `null`, bytes of length 3 or 4, and for `color`
a space from the list. Out-of-gamut and out-of-range chroma are not refused,
as CSS does not refuse them. Hue is not wrapped.

**Bytes, and `rgba`** (Kris, 2026-11-03: "I accepted glued units as writ,
including gluing non-integer values, e.g., <ff8800ff>rgba"). The hex payload
is bytes, never a string: YAY's bytes already are lowercase hex in pairs,
refuse an odd digit and say "these are octets". `rgb` takes three bytes and
`rgba` four, alpha last as in `#rrggbbaa`, so the tag says how many
components to expect even though CSS Color 4 made `rgba()` an alias of
`rgb()`. CSS's three- and four-digit shorthands (`#fff`) are not carried:
they are a spelling, not a value.

CSS Color 5's functions (`color-mix()`, relative colors, `light-dark()`,
`contrast-color()`) compute a color from others; they are expressions, not
values, and are out of scope. Color 5's `device-cmyk()` is a value with a
fallback, not an expression, and is out of scope for this first set too.

## Recommendation in one paragraph

**Decided 2026-11-03: F**, on the 3.0.0 line. A simple name fused after any
inline value but a keyword (`10s`, `<ff8800ff>rgba`, `'…'date`), any other
tag after one space in parentheses, raw up to the first `)` or one quoted
string (`'X-1' (com.example/sku)`, `3 ('a)b')`), and a parenthesized tag
introducing a block (`color: (rgb)`, `- (match:splitRecord)`). It keeps E's
payload-first reading and glued units, needs no lookahead to tell a tag
from a value, and leaves today's misindentation errors in place. Strings in
standard syntax for dates and times, a tag per unit for durations, bytes
or number arrays for colors, the smallcaps object for YSON and JSON, and
Endo patterns as tagged values for YAY Schema. E's paragraph of 2026-11-02
and D's of 2026-11-01 are at `7df077b`.

## Test plan

- The prototype's cases in [`tagged-values-f/cases/`](tagged-values-f/cases/)
  (29 documents with their values and 31 refusals, one or more per rule in
  [tagged-values-f.md](tagged-values-f.md)) move into `test/yay` and
  `test/nay` with the reference port, with an expected file per port.
  2.0.0's corpus is unchanged: the bare-word refusals stay refused for the
  same reason.
- Formatter cases: `'x'(t)` formats to `'x' (t)`;
  `1 (s)` formats to `1s`; `1 ('t')` to `1 (t)`; a raw tag that must be
  quoted is quoted; `--check -t yay` names the line of each unformatted one.
- Each payload kind tagged, YAY → YSON → YAY, and YAY → JSON →
  schema-guided JSON import → YAY.
- A round trip through Endo: `decodeFromSmallcaps(yson)` is `makeTagged`
  with equal tag and payload, and `encodeToSmallcaps` of it reads back.
- YSON with keys `#tag`, `#x`, `*y`, `!z` reads back as objects in binyay
  and decodes in smallcaps; YSON integers are `"+42"`.
- Conventional tags under a schema: `"2026-02-30"date` refused, a color
  array of length 5 refused, `<ff8800>rgba` refused (three bytes).

## Rollout

On the 3.0.0 line, with the binyay stdin and native-bin work and
yaydown-final-newline (Kris, 2026-11-03).

1. Settle the open questions.
2. The spec and `GRAMMAR.md`, the reference port and the corpus; YSON's key
   escape and `"+42"` integers in the same major version.
3. The other ports' parsers and writers, each against the corpus; SHON;
   binyay's targets.
4. Conventional tags, as a separate section of the specification that ports
   may validate or may pass through as plain tagged values.
5. The 3.0.0 release notes' tagged-values section.
6. YAY Schema's patterns, then the dev editor's tagged kind and the color
   editor.

## Open questions

Restated 2026-11-03 for F, numbered fresh. Decided, for the record: the
written form is F (2026-11-03); a simple name fuses inline and any other is
parenthesized with one space before `(` in normal form; raw text runs to
the first `)`, and a tag holding `)` is a quoted string inside the
parentheses; units glue as E wrote them, non-integers included; colors'
hex payload is bytes and `rgba` exists (E's 1 and 7); YSON integers are
`"+42"` (E's 11, accepted as P4 of the schema-patterns design). Moot under
F: E's 3 (the bare charset: dotted, colon and slash names are now
parenthesized) and 4 (the canonical writer and lint over a block: the
misindent stays an error). E's 2 and 5 survive as 9 and 8 below.

1. **The fused simple name: ASCII letters, digits and `_`, not beginning
   with a digit?** Recommended: yes. It leaves out `.`, `-`, `+`, `:`, `/`,
   `,`, `]`, `}`, `#` and space, so decimals, signs, keys, separators and
   comments read as today; anything else goes in parentheses. Non-ASCII
   letters are the alternative, at the cost of a Unicode table in every
   port.
2. **The numeric guards:** after a number, no fused name beginning with
   `e` or `E` (`10em` refused; `10 (em)`), and after the integer `0`, none
   beginning with `x`, `o` or `b` (`0xff` refused, not `xff(0)`)?
   Recommended: yes; without the second, a hex literal typed by habit is
   silently a tagged zero.
3. **No fused name on a keyword:** `true (t)`, `nan (ms)`, never `nanms`?
   Recommended: yes; `nanms` and `truex` stay the refused words they are.
4. **Fused names on numbers are open:** `10px` is `px(10)` in the grammar,
   and the six units are the conventional set (§ 4.2), not a grammar list?
   Recommended: yes, as Kris's rule fuses any simple name on any inline
   value; a schema or lint flags an unknown unit. The alternative keeps E's
   closed six in the grammar and refuses `10px`.
5. **A fused `(`**, `'X-1'(com.example/sku)`: accepted, and written spaced
   by the formatter (so `--check -t yay` flags it)? Recommended: accept;
   Kris's own first example fused it, and `--check -t yay` already
   separates valid from formatted. Refusing it is the stricter choice.
6. **Raw text in parentheses:** spaces inside kept, `#` and `\` literal (no
   comment, no escape), and a raw tag may not be empty, begin or end with a
   space, begin with a quote, or hold a tab; those are written quoted
   (`('')`, `(' t ')`)? Recommended: yes, so every raw tag means what it
   shows.
7. **Several tags:** inline, at most one fused name (the innermost), then
   ` (b)` for each outer one (`'…'datetime (utc)`); before a block,
   outermost first (`- (outer) (inner)`)? Recommended: yes, the tag
   nearest the payload innermost in both directions, as in E.
8. **A tagged root block:** `(point)` on the first line, the block below
   at column 0? Recommended: column 0, where a root block sits today; the
   alternative is a deeper indent, as under `key: (tag)`.
9. **A tag over one quoted line** (`a: (t)` over `'x'`)? Recommended:
   refused, as `a:` over one quoted line is today; write `a: 'x'(t)`. Two
   or more quoted lines are a concatenation, and the tag is its prefix.
10. **Number arrays for `rgb` and `rgba`**, besides bytes? Kris's example
    tags three numbers `(rgb)`. Recommended: yes, `rgb` three numbers 0 to
    255 and `rgba` four with alpha 0 to 1; bytes stay the hex spelling.
11. **How are tags namespaced?** Recommended: simple names are the
    specification's; others name their owner reversed-domain, with `/`
    before the local name, as Kris's `(com.example/sku)` does; Endo's
    (`copySet`, `(match:…)`) pass through as Endo's. (E's 6.)
12. **Days in durations?** Recommended: no `d` in the conventional set; a
    calendar day is `'P1D'duration` and 24 hours is `24h`. (E's 8.)
13. **Does an instant keep its offset?** Recommended: yes, as written, so
    equality is by text and "the same moment" is the application's
    comparison. (E's 9.)
14. **YSON's key escape for `#`, `*` and `!` keys** in 3.0.0? Recommended:
    yes; 2.0.0 did not change YSON, and without it YSON is not the
    smallcaps subset the README says it is, tagged or not. (E's 10.)
15. **SHON: `-T tag value`, and fused names on SHON number tokens?**
    Recommended: both, in 3.0.0. Parentheses are shell syntax, so SHON keeps
    `-T` for every tag (`-T com.example/sku X-1`), and `--timeout 10s`,
    a string token in 2.0.0, becomes `s(10)`. (E's 12.)
16. **Leap seconds:** accept `:60` where RFC 3339 allows it, knowing that
    Temporal reads it as `:59`? Recommended: accept, and say the Temporal
    mapping clamps. (E's 13.)
17. **Times without seconds** (`'15:00'time`)? Recommended: refused, since
    RFC 3339 requires seconds. (E's 14.)
