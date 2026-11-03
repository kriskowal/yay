# Tagged values

Status: decided, 2026-11-03, for the 3.0.0 line, against binyay 2.0.0
(`733849b`). Proposed 2026-11-01; Kris chose option E on 2026-11-02, walked
its bare words back for option F on 2026-11-03, and the same day stepped
back once more for the simpler grammar here, option G: "Tags should simply
always be parenthesized, never fused, in both block prefix and inline
suffix forms." Kris then answered every question G left. Nothing is open;
the build waits for Kris's go-ahead. The grammar patch and its evidence are
in [tagged-values-g.md](tagged-values-g.md). Option F's text and prototype
are at `d459f0d` on `design-tagged-values-parens`, E's at `7df077b` on
`design-tagged-values`.

## Problem

YAY has eight kinds of value: null, boolean, integer, float, string, bytes,
array and object. A document that holds a discriminated union has to spell
the discriminant as an object key or a string beside the value, and a
document that round-trips Endo's passable data has no way to hold a tagged
value at all.

OCapN and Endo already have the missing kind. Endo's
`makeTagged(tag, payload)` makes a value whose `passStyleOf` is `"tagged"`,
and `@endo/marshal`'s smallcaps encoding writes it as
`{"#tag": tag, "payload": payload}`. OCapN's data model has the same thing:
a tag and one payload. CBOR has had it from the start (RFC 8949 § 3.4:
major type 6, a number and one data item). YSON is already a subset of
smallcaps, so YAY is one kind away from carrying all of Endo's passable
data that is not a capability, a promise or an error.

## Scope

In scope: the data model, the written form in YAY and MEH, YSON, JSON,
SHON, binyay's outputs, and YAY Schema's keyword.

Not in scope: **any tag convention.** YAY codifies no tag names, no
namespaces, no units, colors, dates, times or durations (Kris,
2026-11-03: "YAY provides no universal conventions"). A tag means what the
document's author and its schema say it means at that position; the same
tag may mean different things in different places. Conventions an editor
chooses to understand (durations, times, colors) belong to that editor:
for dev.kriskowal.com, to its own designs. Capabilities, promises and
errors stay outside YAY.

## 1. The model

A **tagged value** is a pair of a **tag** and one **payload**.

- **The tag is a string,** any string, the empty string included. Endo
  requires a string (`makeTagged` refuses anything else) and OCapN's tag is
  a string as well, so any narrower model would fail to round-trip with
  them.
- **The payload is any YAY value,** a tagged value included, so tags nest.
- **No namespaces.** There is no global registry of tags and no reserved
  name. Kris, 2026-11-03: "The same tag may be used for different values
  and have different meanings in those positions. Types, for example, may
  be implied by a schema and tags might be reserved for discriminated
  unions in band."
- **Equality and hashing.** Two tagged values are equal when their tags are
  equal strings and their payloads are equal YAY values. A tagged value
  never equals its payload, nor any value of another kind: `10 (s)` is not
  `10`, and `60 (s)` is not `1 (m)`. Hash the tag and the payload together
  with a kind marker no other kind uses.
- **Ordering**, where a port orders values (Endo's `rankOrder` does), puts
  tagged after the other kinds, then by tag, then by payload.
- **Round trip.** YAY tagged ↔ Endo `makeTagged(tag, payload)` ↔ smallcaps
  `{"#tag": tag, "payload": …}` ↔ OCapN tagged, losslessly, for every tag
  and every payload YAY can hold. Endo payloads YAY cannot hold
  (`undefined`, remotables, promises, errors) are refused on the way in, as
  they are today.

The data model table in `GRAMMAR.md` grows a ninth row:

| Type | Description |
|------|-------------|
| Tagged | A tag (a string) and one payload (any value) |

## 2. The written form: always in parentheses (option G)

```yay
count: 10 (s)
color: <ff8800ff> (rgba)
sku: 'X-1' (com.example/sku)
odd: 3 (a\)b)
when: '2026-11-03T05:00:00Z' (datetime) (utc)

color: (rgb)
  - 255
  - 136
  - 0

- (outer) (inner)
  name: 'x'
```

(The tag names are examples. YAY gives none of them a meaning.)

**A tag is always in parentheses.** After an inline value, one space, then
`(tag)`. Before a block, `(tag)` where the block is introduced, and the
block below. A tag is never fused to a name or a value, and never bare.

**Inside the parentheses, a tag is written as a double-quoted YAY string
is, with `(` and `)` as its delimiters** (Kris, 2026-11-03: "Treat tags
much like strings, except with parens instead of quotes"):

- The same escapes as a double-quoted string, with `\)` in the place of
  `\"`: `\)`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t` and `\u{…}`. So
  `3 (a\)b)` is the tag `a)b`, and `(back\\slash)` is `back\slash`.
- Every other character but a line break and a tab stands for itself, as in
  a string: spaces (inner and at the edges), `(`, `"`, `'`, `#`, `,`, `]`,
  `}` and `:`. `(it's)` is `it's`, `('a')` is `'a'` with its quotes, and
  `(a  # b)` holds no comment.
- An unknown escape is refused, as in a string: `\(` and `\"` are not
  escapes in a tag (`(` and `"` need none), as `\'` is not in a
  double-quoted string.
- `()` is the empty tag, as `""` is the empty string, and `( a )` is the
  tag ` a `, edge spaces kept. A tab is refused, as in a string; write
  `\t`.

**Inline: the tag follows its value.** Any inline value takes tags:
numbers, keywords (`true (t)`, `nan (x)`), strings, bytes, inline arrays
and objects, and values inside inline arrays and objects
(`[10 (s), 'ff' (x)]`). A comment follows the tags, two spaces then `#`:
`a: 'x' (t)  # note`. Separators keep their rule (exactly `, `), and the
parentheses shield what is inside them.

**Before a block: the tag introduces it.**

- `key: (tag)` over the nested content, as `key:` is today: an object, an
  array, or a concatenated string of two or more quoted lines.
- `- (tag)` over the item's content at the dash's indent plus two.
- `` key: (tag) ` `` and `key: (tag) >` for block strings and bytes, the
  backtick alone at the end of a property's line as today; an item takes
  text after it on the same line (`` - (markdown) ` First line. ``).
- **At the root**, `(tag)` alone on the first content line, the block
  below at column 0, where a root block sits today.
- A comment may follow: `color: (rgb)  # r, g, b`.
- An inline value never takes a prefix (`a: (rgb) [255, 136, 0]` is
  refused, write `a: [255, 136, 0] (rgb)`), and a `(tag)` with no block
  below is refused: a tag needs a payload. A block tag over a single quoted
  line is refused, as `a:` over one quoted line is today: write
  `a: 'x' (t)`.

**Several tags: the tag nearest the payload is innermost, both ways.**
Inline, `'…' (datetime) (utc)` is `utc(datetime('…'))`. Before a block,
`- (outer) (inner)` is `outer(inner(…))`. One space between tags.

**Spacing: lenient on MEH, strict on YAY** (Kris, 2026-11-03: "Lenient on
"meh" input, strict on "yay" input, strict on "yay" output. One space
between value and tag for inline suffix notation"). YAY input and YAY
output have exactly one space between a value and its tag and between two
tags; MEH input accepts any run of spaces, or none: `'X-1'(sku)`,
`1(a)(b)`, `1   (t)`. So `yay -f meh -t yay` normalizes the spacing, and
`yay --check -t yay` flags it.

**Normal form**, as the writer emits: one space before each tag; a tag's
characters as written, with `)` and `\` escaped and a control character as
the double-quoted writer escapes it; the block form exactly when the
payload would be written as a block in that position.

**Why it is unambiguous.** In YAY 2.0.0 nothing may follow a complete
inline value but a comment, and no value begins with `(`. G gives only
those refused texts a meaning. A `(` at the head of a property's or item's
value is always a block's tag, a `(` after a value always a suffix, so no
reader looks ahead to tell a tag from a value; it looks ahead only, as for
`key:` today, to find the block. The key test needs no change, since a `(`
begins no key. The root-object test ("the first line at indent 0 contains a
colon outside quotes") skips parenthesized text as it skips quotes, so
`(match:splitRecord)` and `'x' (a: b)` at the root are not root objects.
A fused name has no form to collide with, so F's guards for exponents
(`10em`), hex literals (`0xff`) and keywords (`nanms`) are gone: all three
stay refused, as in 2.0.0. The prototype changes none of 2.0.0's 136 valid
documents, read as YAY or as MEH, and makes none of its 91 refused ones
valid ([tagged-values-g.md](tagged-values-g.md)).

## 3. Alternatives

Considered and rejected, one line each; their full text is at `7df077b`
(A to E) and `d459f0d` (F).

- **A. Call form,** `s(10)`, CBOR's diagnostic style. Reason: a block
  payload needs a closing line, and it reads tag first.
- **B. Suffix labels,** `10s`, for units only. Reason: it tags numbers and
  nothing else.
- **C. Prefix tags,** YAML's `!rgb <ff8800>`. Reason: a sigil, and the tag
  before the payload where Kris writes the payload first.
- **D. C with unit suffixes.** Reason: two spellings and a closed unit
  registry in the grammar.
- **E. Bare words after an inline value and before a block.** Reason: a
  bare word cannot be told from a value without the next line, so a
  misindented sibling became a tagged block (Kris: "too many hazards").
- **F. Fused simple names inline, parentheses for the rest and before
  blocks.** Reason: fused names needed guards for exponents, hex literals
  and keywords, and two spellings of one tag.
- **EDN's `#tag value`.** Reason: `#` begins a comment.
- **The reserved object key as the written form,** smallcaps verbatim.
  Reason: verbose; it is the YSON and JSON form (§ 4).
- **Typed keys,** `timeout(s): 10`. Reason: the tag belongs to the value,
  so an array of them could not be written.

## 4. Mappings

**YSON** writes a tagged value as smallcaps does, `{"#tag": "rgb",
"payload": "*ff8800"}`, with the tag and the payload encoded as YSON
encodes any string and value: `10 (s)` is `{"#tag":"s","payload":"+10"}`
once integers are `"+42"`. The tag is a string and gets the string escape
(a tag beginning with `!`, `#`, `*`, `+`, `-` and the other smallcaps
prefixes is written `"!…"`). This keeps YSON a subset of smallcaps. It
needs two changes due in 3.0.0 regardless (Kris, 2026-11-03):

- **The key escape.** YSON writes object keys verbatim today, so the YAY
  object `{"#tag": "x", payload: 1}` becomes YSON that smallcaps reads as
  tagged, and a key `#x` makes YSON that smallcaps refuses. YSON writes a
  key that begins with `#`, `*` or `!` with the `!` escape it already gives
  strings, and reads it back.
- **Integers as `"+42"`** (P4 of the schema-patterns design), so a YSON
  integer is smallcaps' bigint.

Both change output, so they are a major version, by the project's rule.

**JSON** cannot hold a tagged value. `yay -t json` writes the smallcaps
object. `-f json` reads `{"#tag": …, "payload": …}` as a plain object; a
schema-guided import restores a tagged value at a position whose schema
admits tagged values.

**SHON** gains one reserved flag, `-T`, which takes the next token as the
tag and the token after as the payload: `yay [ --timeout -T s 10 ]`. A
payload that is an array or object is a bracket group:
`-T rgb [ 255 136 0 ]`. Nothing is fused onto SHON tokens: `10s` stays the
string it is in 2.0.0 (Kris, 2026-11-03: "`-T tag value` only").

**binyay's outputs.** Each target gains a tagged type:

| Target | Tagged as |
|---|---|
| `js` | `{tag, payload}` from a small frozen `tagged(tag, payload)` helper; a caller that wants Endo's passable wraps it with `makeTagged` |
| `go` | `yay.Tagged{Tag string; Payload any}` |
| `python` | a frozen dataclass `Tagged(tag: str, payload)` |
| `rust` | `Value::Tagged(String, Box<Value>)` |
| `c` | `YAY_TAGGED`, with `tag` and `payload` fields in the value union |
| `java` | `record Tagged(String tag, Object payload)` |
| `scheme` | a record type `tagged` with `tagged-tag` and `tagged-payload` |
| `md`, `yd` | front matter holds tagged values as YAY does |

Each port changes its parser (MEH lenient, YAY strict), its writer, its
value model, its equality, its YSON reader and writer, its code generator,
and the shared corpus under `test/`.

**YAY Schema** says what a position holds, so a schema, not a tag, gives a
value its type. Where a document carries a discriminated union in band, a
schema admits tagged values and names the tags (`tag`, a string or a
string schema) and each branch's `payload`; Endo patterns carry this as
tagged values, as the schema-patterns design has it. YAY Schema defines no
tag of its own beyond Endo's matchers.

## Test plan

- The prototype's cases in [`tagged-values-g/cases/`](tagged-values-g/cases/)
  move into `test/yay`, `test/meh` and
  `test/nay` with the reference port, each with its expected value. 2.0.0's
  corpus is unchanged.
- Writer cases: `-f meh -t yay` of `'x'(t)` is `'x' (t)`; of `1  (a)(b)`,
  `1 (a) (b)`; a tag with `)` or `\` is escaped; `--check -t yay` names the
  line of each unformatted one.
- Each payload kind tagged, YAY → YSON → YAY, and YAY → JSON →
  schema-guided JSON import → YAY.
- A round trip through Endo: `decodeFromSmallcaps(yson)` is `makeTagged`
  with an equal tag and payload, and `encodeToSmallcaps` of it reads back,
  for tags `""`, `"a)b"` and `"!x"`.
- YSON with keys `#tag`, `#x`, `*y`, `!z` reads back as objects in binyay
  and decodes in smallcaps; YSON integers are `"+42"`.
- SHON: `-T s 10`, `-T rgb [ 255 136 0 ]`, `-T` nested, and `10s` still a
  string.

## Rollout

On the 3.0.0 line, with the binyay stdin and native-bin work and
yaydown-final-newline (Kris, 2026-11-03), after Kris's go-ahead.

1. The spec and `GRAMMAR.md` ([tagged-values-g.md](tagged-values-g.md)
   § The patch), the reference port and the corpus; YSON's key escape,
   `"+42"` integers and tagged values in the same major version.
2. The other ports' parsers and writers, each against the corpus; SHON's
   `-T`; binyay's targets.
3. The 3.0.0 release notes' tagged-values section.
4. YAY Schema's tagged keyword; the dev editor's tagged kind and its own
   conventional tags, in dev.kriskowal.com's designs, not YAY's.

## Open questions

None. Decided 2026-11-03, for the record: always parenthesized (G), one
space in normal form, MEH lenient and YAY strict; tags written as strings
with parentheses for quotes, `\)` and `\\` escaped; the nearest tag
innermost both ways; a tagged root block at column 0; a block tag over one
quoted line refused; no conventions and no namespaces in YAY; YSON's key
escape and `"+42"` in 3.0.0; SHON `-T` only. Two consequences the designer
drew from "much like strings", which Kris may overturn without cost to the
rest: `()` is the empty tag and edge spaces are kept, and `\(` and `\"` are
refused as unknown escapes.
