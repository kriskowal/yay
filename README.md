# YAY

YAY is Yet Another YAML.
The preferred extension for YAY is `.yay`.

**[Command Line Tool Documentation](CLI.md)** - Usage, options, and formatting behavior.

YAY is intended for data.
The data can be annotated with comments.
YAY is slightly more expressive than JSON, just to catch up with modern
affordances like _big integers_ and _bytes_, and clear boundaries between what
should be expressed as integers versus floating-point precision.
Like YAML and unlike JSON, YAY produces cleaner diffs because changes to a
value do not cascade changes into the next or prior line.
YAY has fewer surprising misfeatures than YAML.

The root document may be any of the value types (null, big integer, float,
boolean, string, array, object, or bytes).
The root may use the multi-line form for values that support it: string,
object, array and byte array.

[at-a-glance.yay](test/yay/at-a-glance.yay)
```yay
roses-are-red: true      # There is no "yes" or "on".
violets-are-blue: false  # Violets are violet.
arrays:
  - "may"
  - "have"
  - "many"
  - "values"
and-objects-too:
  integers-are-distinct: 42
  from-their-floating-friends: 6.283 185 307 179 586  # digit grouping
inline:
  string: "is concise"
  array: [infinity, -infinity, nan]
  object: {bigint: 1, float64: 2.0}
  bytes: <f33d face>
block:
  string: `
    This is a string.
    There are many like it.
  array:
    - "But"
    - "this"
    - "one's"
  object:
    mine: null
  bytes: >
    b0 b5  c0 ff  # Bob's Coffee
    fe fa  ca de  # Facade.
concatenated:
  "I'm not dead yet. "
  "I feel happy!"
unicode-code-point: "\u{1F600}"  # UTF-16 surrogates are inexpressible
"name with spaces": 'works too'
```

# Examples

## Null

The keyword `null` denotes a null value.

[null-literal.yay](test/yay/null-literal.yay)
```yay
null
```

[null-literal.js](test/js/null-literal.js)
```js
null
```

## Booleans

The literals `true` and `false` denote booleans.

A true boolean value.

[boolean-true.yay](test/yay/boolean-true.yay)
```yay
true
```

[boolean-true.js](test/js/boolean-true.js)
```js
true
```

A false boolean value.

[boolean-false.yay](test/yay/boolean-false.yay)
```yay
false
```

[boolean-false.js](test/js/boolean-false.js)
```js
false
```

## Big Integers

Unquoted decimal digit sequences are _big integers_ (arbitrary precision).
A leading minus sign denotes a negative big integer; the minus must not be
followed by a space (e.g. `-10`, not `- 10`).
Spaces may be used to group digits for readability; they do not change the
value.

A basic positive integer.

[integer-big-basic.yay](test/yay/integer-big-basic.yay)
```yay
42
```

[integer-big-basic.js](test/js/integer-big-basic.js)
```js
42n
```

A negative integer.

[integer-big-negative.yay](test/yay/integer-big-negative.yay)
```yay
-42
```

[integer-big-negative.js](test/js/integer-big-negative.js)
```js
-42n
```

Spaces group digits for readability without changing the value.

[integer-big.yay](test/yay/integer-big.yay)
```yay
867 5309
```

[integer-big.js](test/js/integer-big.js)
```js
8675309n
```

## Floating-Point (Float64)

A decimal point must be present to distinguish a float from a big integer.
Otherwise, the same rules as JSON apply for whether digits must appear before
or after the decimal point (e.g. `.5` and `1.` are valid).
Decimal literals with a decimal point, or the keywords `infinity`, `-infinity`,
and `nan`, denote 64-bit floats.
A leading minus must not be followed by a space (e.g. `-0.0`, not `- 0.0`).
Spaces may group digits.
A floating-point number captures exactly and only the nearest expressible IEEE
754 binary64 value.
The keyword `nan` denotes the same canonical NaN as JavaScript `NaN` when
captured in a `Float64Array`.

A basic floating-point number.

[number-float.yay](test/yay/number-float.yay)
```yay
6.283185307179586
```

[number-float.js](test/js/number-float.js)
```js
6.283185307179586
```

A float with a leading decimal point (no integer part).

[number-float-leading-dot.yay](test/yay/number-float-leading-dot.yay)
```yay
.5
```

[number-float-leading-dot.js](test/js/number-float-leading-dot.js)
```js
0.5
```

A float with a trailing decimal point (no fractional part).

[number-float-trailing-dot.yay](test/yay/number-float-trailing-dot.yay)
```yay
1.
```

[number-float-trailing-dot.js](test/js/number-float-trailing-dot.js)
```js
1
```

Negative zero is distinct from positive zero.

[number-float-negative-zero.yay](test/yay/number-float-negative-zero.yay)
```yay
-0.0
```

[number-float-negative-zero.js](test/js/number-float-negative-zero.js)
```js
-0
```

Positive infinity.

[number-float-infinity.yay](test/yay/number-float-infinity.yay)
```yay
infinity
```

[number-float-infinity.js](test/js/number-float-infinity.js)
```js
Infinity
```

Negative infinity.

[number-float-negative-infinity.yay](test/yay/number-float-negative-infinity.yay)
```yay
-infinity
```

[number-float-negative-infinity.js](test/js/number-float-negative-infinity.js)
```js
-Infinity
```

Not-a-number (canonical NaN).

[number-float-nan.yay](test/yay/number-float-nan.yay)
```yay
nan
```

[number-float-nan.js](test/js/number-float-nan.js)
```js
NaN
```

Spaces group digits for readability in floats.

[number-float-grouped.yay](test/yay/number-float-grouped.yay)
```yay
6.283 185 307 179 586
```

[number-float-grouped.js](test/js/number-float-grouped.js)
```js
6.283185307179586
```

Scientific notation using `e` or `E` for the exponent (Avogadro's number).

[number-float-avogadro.yay](test/yay/number-float-avogadro.yay)
```yay
6.022e23
```

[number-float-avogadro.js](test/js/number-float-avogadro.js)
```js
602200000000000000000000
```

## Block Strings

Block strings use the backtick (`` ` ``) introducer.
The body continues until the next line that is indented the same or less.
The first two spaces of each content line are stripped; any further indentation
is preserved.
Empty lines (including lines with insufficient indentation) are interpreted as
newlines.
Trailing empty lines collapse to a single trailing newline.
Block strings do not support escape sequences—a backslash is just a backslash.
Comments are also not recognized inside block strings; `#` is literal content.

At root level or as an array item, content may appear on the same line after ``
` `` (backtick + space + content).
When the backtick is alone on a line, the result has an implicit leading
newline.

In property context, the backtick must be followed only by a newline (no
content on the same line).
There is no implicit leading newline in property context.

### At Root Level

Content on the same line as the backtick starts the string without a leading
newline.

[string-block-root-same-line.yay](test/yay/string-block-root-same-line.yay)
```yay
` I think you ought to know I'm feeling very depressed.
  This will all end in tears.
```

[string-block-root-same-line.js](test/js/string-block-root-same-line.js)
```js
"I think you ought to know I'm feeling very depressed.\nThis will all end in tears.\n"
```

Backtick alone on its line produces a leading newline because content starts on
the following line.

[string-block-root-next-line.yay](test/yay/string-block-root-next-line.yay)
```yay
`
  I've calculated your chance of survival,
  but I don't think you'll like it.
```

[string-block-root-next-line.js](test/js/string-block-root-next-line.js)
```js
"\nI've calculated your chance of survival,\nbut I don't think you'll like it.\n"
```

An empty line in the middle of a block string is preserved as a newline.

[string-block-empty-middle.yay](test/yay/string-block-empty-middle.yay)
```yay
`
  I'm getting better!

  No you're not.
```

[string-block-empty-middle.js](test/js/string-block-empty-middle.js)
```js
"\nI'm getting better!\n\nNo you're not.\n"
```

The `#` character inside a block string is literal content, not a comment.

[string-block-root-hash.yay](test/yay/string-block-root-hash.yay)
```yay
` # this is not a comment
  it is content
```

[string-block-root-hash.js](test/js/string-block-root-hash.js)
```js
"# this is not a comment\nit is content\n"
```

A block string may be deeply nested and the indentation prefix will be absent
in the value.
The string will end with a single newline regardless of any subsequent newlines
in the YAY text.

[string-block-nested-in-object-and-array.yay](test/yay/string-block-nested-in-object-and-array.yay)
```yay
parrot:
  condition: `
    No, no, it's just resting!

  remarks:
  - ` Remarkable bird, the Norwegian Blue.
      Beautiful plumage, innit?

  - ` It's probably pining for the fjords.
      Lovely plumage.
```

[string-block-nested-in-object-and-array.js](test/js/string-block-nested-in-object-and-array.js)
```js
({
  "parrot": {
    "condition": "No, no, it's just resting!\n",
    "remarks": [
      "Remarkable bird, the Norwegian Blue.\nBeautiful plumage, innit?\n",
      "It's probably pining for the fjords.\nLovely plumage.\n",
    ],
  },
})
```

### As Object Property

In property context, the backtick must be alone on the line (no content after
it).
There is no implicit leading newline.
The first content line becomes the start of the string.

[string-block-property.yay](test/yay/string-block-property.yay)
```yay
message: `
  By Grabthar's hammer, we live to tell the tale.
```

[string-block-property.js](test/js/string-block-property.js)
```js
({
  "message": "By Grabthar's hammer, we live to tell the tale.\n",
})
```

An empty line in the middle of a block string property is preserved.

[string-block-property-empty-middle.yay](test/yay/string-block-property-empty-middle.yay)
```yay
message: `
  It's not pining!

  It's passed on! This parrot is no more!
```

[string-block-property-empty-middle.js](test/js/string-block-property-empty-middle.js)
```js
({
  "message": "It's not pining!\n\nIt's passed on! This parrot is no more!\n",
})
```

A block string property followed by another property: the block ends when a
line at the same or lesser indent appears.
Trailing empty lines collapse to a single trailing newline.

[string-block-property-trailing-empty.yay](test/yay/string-block-property-trailing-empty.yay)
```yay
message: `
  By Grabthar's hammer... what a savings.


next: 1
```

[string-block-property-trailing-empty.js](test/js/string-block-property-trailing-empty.js)
```js
({
  "message": "By Grabthar's hammer... what a savings.\n",
  "next": 1n,
})
```

## Inline Strings

Strings may be quoted with double or single quotes.
Double-quoted strings support escape sequences: `\"`, `\\`, `\/`, `\b`, `\f`,
`\n`, `\r`, `\t`, and `\u{XXXXXX}` for Unicode code points.
Single-quoted strings are literal (no escape sequences).

A double-quoted string.

[string-inline-doublequote-basic.yay](test/yay/string-inline-doublequote-basic.yay)
```yay
"This will all end in tears."
```

[string-inline-doublequote-basic.js](test/js/string-inline-doublequote-basic.js)
```js
"This will all end in tears."
```

A single-quoted string (literal, no escapes).

[string-inline-singlequote-basic.yay](test/yay/string-inline-singlequote-basic.yay)
```yay
'Are you suggesting coconuts migrate?'
```

[string-inline-singlequote-basic.js](test/js/string-inline-singlequote-basic.js)
```js
"Are you suggesting coconuts migrate?"
```

A double-quoted string with escape sequences.

[string-inline-doublequote-escapes.yay](test/yay/string-inline-doublequote-escapes.yay)
```yay
"\"\\\/\b\f\n\r\t\u{263A}"
```

[string-inline-doublequote-escapes.js](test/js/string-inline-doublequote-escapes.js)
```js
'"\\/\b\f\n\r\t☺'
```

A double-quoted string with a Unicode emoji (literal UTF-8).

[string-inline-doublequote-unicode-emoji.yay](test/yay/string-inline-doublequote-unicode-emoji.yay)
```yay
"😀"
```

[string-inline-doublequote-unicode-emoji.js](test/js/string-inline-doublequote-unicode-emoji.js)
```js
"😀"
```

A Unicode code point escape for a character outside the BMP (U+1F600), which
requires a surrogate pair in UTF-16.
The `\u{...}` escape accepts 1 to 6 hexadecimal digits representing a Unicode
code point (e.g. `\u{41}` for "A", `\u{1F600}` for "😀").
Surrogate code points (U+D800 through U+DFFF) are forbidden in `\u{...}`
escapes.
Unlike JSON, the four-digit `\uXXXX` form is not supported; use `\u{XXXX}`
instead.

[string-inline-doublequote-unicode-surrogate-pair.yay](test/yay/string-inline-doublequote-unicode-surrogate-pair.yay)
```yay
"\u{1F600}"
```

[string-inline-doublequote-unicode-surrogate-pair.js](test/js/string-inline-doublequote-unicode-surrogate-pair.js)
```js
"😀"
```

## Concatenated Strings (Quoted Lines)

Multiple quoted strings on consecutive lines are concatenated into a single
string.
This is useful for breaking long strings across lines without introducing
newlines in the result, or including visibly escaped characters like tab
that are otherwise forbidden in string blocks.

[string-multiline-concat.yay](test/yay/string-multiline-concat.yay)
```yay
confession:
  "I'm not dead yet. "
  "I feel happy!"
```

[string-multiline-concat.js](test/js/string-multiline-concat.js)
```js
({ "confession": "I'm not dead yet. I feel happy!" })
```

## Block Arrays

Arrays are written as a sequence of items, each introduced by `- ` (dash and
space).
The two-character `- ` prefix is the list marker; the value follows
immediately.
Items may be nested: a bullet line whose content starts with `- ` begins an
inner list; following lines at the same or greater indent whose content also
starts with `- ` are siblings in that inner list, until a line at less indent
or another top-level bullet.
An array may be given a name as a key followed by `:`.

A basic block array with three integer items.

[array-multiline.yay](test/yay/array-multiline.yay)
```yay
- 5
- 3
```

[array-multiline.js](test/js/array-multiline.js)
```js
[5n, 3n]
```

Nested arrays where each top-level item contains an inner array.

[array-multiline-nested.yay](test/yay/array-multiline-nested.yay)
```yay
- - "a"
  - "b"
- - 1
  - 2
```

[array-multiline-nested.js](test/js/array-multiline-nested.js)
```js
[["a", "b"], [1n, 2n]]
```

An array as the value of an object property.

[array-multiline-named.yay](test/yay/array-multiline-named.yay)
```yay
complaints:
- "I didn't vote for you."
- "Help, help, I'm being repressed!"
```

[array-multiline-named.js](test/js/array-multiline-named.js)
```js
({
  "complaints": [
    "I didn't vote for you.",
    "Help, help, I'm being repressed!",
  ],
})
```

## Inline Arrays

Inline arrays use JSON-style bracket syntax with strict spacing rules: no space
after `[`, no space before `]`, exactly one space after each `,`.

A simple inline array with string values.

[array-inline-doublequote.yay](test/yay/array-inline-doublequote.yay)
```yay
["And there was much rejoicing.", "yay."]
```

[array-inline-doublequote.js](test/js/array-inline-doublequote.js)
```js
["And there was much rejoicing.", "yay."]
```

An inline array containing big integers.

[array-inline-integers.yay](test/yay/array-inline-integers.yay)
```yay
[42, 404, 418]
```

[array-inline-integers.js](test/js/array-inline-integers.js)
```js
[42n, 404n, 418n]
```

An inline array containing byte array literals.

[array-inline-bytearray.yay](test/yay/array-inline-bytearray.yay)
```yay
[<b0b5>, <cafe>]
```

[array-inline-bytearray.js](test/js/array-inline-bytearray.js)
```js
[
  Uint8Array.from([0xb0, 0xb5]),
  Uint8Array.from([0xca, 0xfe]),
]
```

Inline arrays nested within an inline array.

[array-inline-nested.yay](test/yay/array-inline-nested.yay)
```yay
[["I feel happy!", "yay."], ["And there was much rejoicing.", "yay."]]
```

[array-inline-nested.js](test/js/array-inline-nested.js)
```js
[
  ["I feel happy!", "yay."],
  ["And there was much rejoicing.", "yay."],
]
```

## Block Objects

Objects are key–value pairs.
A key is followed by `:` and then the value.
Object keys must be either alphanumeric (letters and digits) or quoted (double
or single quotes, same rules as string values).
Nested objects are indented by two spaces.
Empty objects are written as `key: {}`.

A simple object with two key-value pairs at the root level.

[object-multiline.yay](test/yay/object-multiline.yay)
```yay
answer: 42
error: 404
```

[object-multiline.js](test/js/object-multiline.js)
```js
({ "answer": 42n, "error": 404n })
```

An object nested within another object, demonstrating indentation.

[object-multiline-nested.yay](test/yay/object-multiline-nested.yay)
```yay
parrot:
  status: "pining for the fjords"
  plumage: "beautiful"
```

[object-multiline-nested.js](test/js/object-multiline-nested.js)
```js
({
  "parrot": { "plumage": "beautiful", "status": "pining for the fjords" },
})
```

Object keys containing spaces or special characters must be quoted.

[object-multiline-doublequote-key.yay](test/yay/object-multiline-doublequote-key.yay)
```yay
"key name": 1
```

[object-multiline-doublequote-key.js](test/js/object-multiline-doublequote-key.js)
```js
({ "key name": 1n })
```

An empty object as a property value.

[object-inline-empty.yay](test/yay/object-inline-empty.yay)
```yay
empty: {}
```

[object-inline-empty.js](test/js/object-inline-empty.js)
```js
({ "empty": {} })
```

## Inline Objects

Inline objects use JSON-style brace syntax with strict spacing rules: no space
after `{`, no space before `}`, exactly one space after each `,`.

A simple inline object with integer values.

[object-inline-integers.yay](test/yay/object-inline-integers.yay)
```yay
{answer: 42, error: 404}
```

[object-inline-integers.js](test/js/object-inline-integers.js)
```js
({ "answer": 42n, "error": 404n })
```

An inline object with string values.

[object-inline-mixed.yay](test/yay/object-inline-mixed.yay)
```yay
{name: 'Marvin', mood: 'depressed'}
```

[object-inline-mixed.js](test/js/object-inline-mixed.js)
```js
({ "mood": "depressed", "name": "Marvin" })
```

An inline object containing both a nested object and an array.

[object-inline-nested.yay](test/yay/object-inline-nested.yay)
```yay
{luggage: {combination: 12345}, air: ["canned", "Perri-Air"]}
```

[object-inline-nested.js](test/js/object-inline-nested.js)
```js
({
  "air": ["canned", "Perri-Air"],
  "luggage": { "combination": 12345n },
})
```

## Block Byte Arrays

Block byte arrays use the `>` introducer (as opposed to `<...>` for inline).
Each line may hold hex chunks and comments (comments start with `#`).
Spaces within hex content are ignored.

At root level or as an array item, hex may appear on the same line after `> `.
The `> ` leader must have hex digits or a comment on the line—`>` alone is
invalid.

In property context, `>` must be followed only by a comment or newline (no hex
on the same line).
This preserves column alignment of hex content across lines.

### At Root Level

A block byte array at root level with hex on the same line as the `>`
introducer.

[bytearray-block-basic.yay](test/yay/bytearray-block-basic.yay)
```yay
> b0b5
  c0ff
```

[bytearray-block-basic.js](test/js/bytearray-block-basic.js)
```js
Uint8Array.from([0xb0, 0xb5, 0xc0, 0xff])
```

A block byte array with a comment on the first line instead of hex.

[bytearray-block-comment-only.yay](test/yay/bytearray-block-comment-only.yay)
```yay
> # header comment
  b0b5 c0ff
```

[bytearray-block-comment-only.js](test/js/bytearray-block-comment-only.js)
```js
Uint8Array.from([0xb0, 0xb5, 0xc0, 0xff])
```

Hex and comments on the same lines for inline documentation of byte sequences.

[bytearray-block-hex-and-comment.yay](test/yay/bytearray-block-hex-and-comment.yay)
```yay
> b0b5 # first chunk
  c0ff # second chunk
```

[bytearray-block-hex-and-comment.js](test/js/bytearray-block-hex-and-comment.js)
```js
Uint8Array.from([0xb0, 0xb5, 0xc0, 0xff])
```

### As Object Property

In property context, the `>` must be alone on the line (optionally followed by
a comment).
Hex content starts on the following lines, preserving column alignment.

[bytearray-block-property.yay](test/yay/bytearray-block-property.yay)
```yay
data: >
  b0b5 c0ff
  eefa cade
```

[bytearray-block-property.js](test/js/bytearray-block-property.js)
```js
({
  "data": Uint8Array.from([0xb0, 0xb5, 0xc0, 0xff, 0xee, 0xfa, 0xca, 0xde]),
})
```

A block byte array property with a comment on the leader line.

[bytearray-block-property-comment.yay](test/yay/bytearray-block-property-comment.yay)
```yay
data: > # raw bytes
  b0b5 c0ff
```

[bytearray-block-property-comment.js](test/js/bytearray-block-property-comment.js)
```js
({ "data": Uint8Array.from([0xb0, 0xb5, 0xc0, 0xff]) })
```

## Inline Byte Arrays

Binary data is written as hexadecimal inside angle brackets.
Hexadecimal must be lowercase.
An odd number of hex digits is forbidden.
`<>` denotes an empty byte array.
Spaces inside the brackets are allowed for readability.

An empty byte array.

[bytearray-inline-empty.yay](test/yay/bytearray-inline-empty.yay)
```yay
<>
```

[bytearray-inline-empty.js](test/js/bytearray-inline-empty.js)
```js
new Uint8Array(0)
```

An inline byte array with hex content.

[bytearray-inline-even.yay](test/yay/bytearray-inline-even.yay)
```yay
<b0b5c0ffeefacade>
```

[bytearray-inline-even.js](test/js/bytearray-inline-even.js)
```js
Uint8Array.from([0xb0, 0xb5, 0xc0, 0xff, 0xee, 0xfa, 0xca, 0xde])
```

An inline byte array as an object property value.

[bytearray-inline-named.yay](test/yay/bytearray-inline-named.yay)
```yay
data: <b0b5c0ffeefacade>
```

[bytearray-inline-named.js](test/js/bytearray-inline-named.js)
```js
({
  "data": Uint8Array.from([0xb0, 0xb5, 0xc0, 0xff, 0xee, 0xfa, 0xca, 0xde]),
})
```

# Encoding And Text

## UTF-8

YAY documents are encoded in UTF-8.
BOM is illegal.
Surrogates are illegal.

## Significant Whitespace

Two leading spaces denote an indent.
Presence of a tab in place of spaces is illegal.
Trailing space on a line is illegal.
Any space after an expected indent are passed through and may be taken
literally in string context, but are otherwise illegal.

## Comments

Comments start with `#` and extend to the end of the line.
Comments may be present anywhere except inside block strings.
A document containing only comments (no value) is invalid.

## YSON (JSON with YAY Extensions)

YSON is a strict superset of JSON that preserves all YAY value types through a
simple prefix convention.
YSON is itself a subset of the [Endo SmallCaps](https://endojs.org) encoding.
Standard JSON cannot represent big integers (only IEEE 754 doubles), byte
arrays, or the special float values `infinity`, `-infinity`, and `nan`.
YSON fills the gap by encoding these types as prefixed strings:

| YAY type   | YSON encoding             | Example              |
|------------|---------------------------|----------------------|
| integer    | `"#"` + digits            | `"#42"`, `"#-7"`     |
| float ∞    | `"#Infinity"`             | `"#Infinity"`        |
| float −∞   | `"#-Infinity"`            | `"#-Infinity"`       |
| float NaN  | `"#NaN"`                  | `"#NaN"`             |
| bytes      | `"*"` + hex               | `"*cafe"`, `"*"`     |

Strings that naturally start with `#` or `*` are escaped with a `!` prefix:
`"!#not an integer"`, `"!*not bytes"`.

YSON is useful when you need to round-trip YAY data through JSON-only tooling
(APIs, databases, message queues) without losing type information.
A YSON document is always valid JSON, so any JSON parser can read it; a
YSON-aware reader will recover the original YAY types.

Convert to YSON:
```sh
yay -t yson config.yay
```

Convert from YSON back to YAY:
```sh
yay -f yson config.yson
```

Unlike plain JSON, YSON preserves `infinity`, `-infinity`, and `nan` by
encoding them as `"#Infinity"`, `"#-Infinity"`, and `"#NaN"`.
If your data does not use big integers, byte arrays, or special float values,
plain JSON (`-t json`) is identical to YSON.

## SHON (Shell Object Notation)

SHON lets you construct structured data directly from command-line arguments,
without writing a file or piping stdin.
See **[SHON.md](SHON.md)** for the full specification.

SHON is activated by `[`, `-x`, `-b`, or `-s` in a positional argument slot:

```sh
# Object
yay [ --name hello --count 42 ]

# Array
yay [ 1 2 3 ]

# Nested
yay [ --servers [ localhost:8080 localhost:8081 ] --options [ --verbose -t ] ]

# Convert to JSON
yay -t json [ --x 1.0 --y 2.0 ]

# Root byte array from hex
yay -t yson -x cafe

# Read a file as bytes or string
yay -b image.png -o image.yay
yay -s message.txt
```

Inside brackets, all YAY value types are available: `-n` (null), `-t` (true),
`-f` (false), `-I` (infinity), `-i` (-infinity), `-N` (NaN), `-x` (hex
bytes), `-b` (file→bytes), `-s` (file→string), `--` (string escape), and
`--key` (object key).
Bare words are strings, bare numbers are integers or floats.
Root scalars are not expressible as SHON; use a file or stdin for those.

## Libraries

- **[libyay](https://www.npmjs.com/package/libyay)** — JavaScript/TypeScript library for parsing and encoding YAY. `npm install libyay`
- **[libyay](https://crates.io/crates/libyay)** (Rust) — Rust crate for parsing and encoding YAY.

## Editor Support

- **[Vim](vim/)** — Syntax highlighting, filetype detection, and indent settings for `.yay` and `.meh` files.
- **[VS Code](vscode/)** — TextMate grammar with syntax highlighting, bracket matching, and indent-based folding.

## References

Examples in this document pay homage to:

- The Hitchhiker's Guide to the Galaxy (Douglas Adams)
- Monty Python and the Holy Grail
- Monty Python's Flying Circus ("Dead Parrot" sketch)
- Galaxy Quest
- Spaceballs
- Tommy Tutone ("867-5309/Jenny")
- The Tau Manifesto

## License

Apache 2.0

Copyright (C) 2026 Kris Kowal
