# libyay

A parser for the [YAY](https://github.com/kriskowal/yay) data format, implemented in JavaScript (ES2024+).

## Requirements

- Node.js 22+ (for `Uint8Array.fromHex`)
- Or any JavaScript runtime with ES2024 support

## Installation

```bash
npm install libyay
```

## Usage

```js
import { parseYay } from 'libyay';

// Parse a YAY document
const result = parseYay('key: "value"');
// => { key: "value" }

// Parse with filename for error messages
const data = parseYay(source, 'config.yay');
```

## API

### `parseYay(source, filename?)`

Parses a YAY document string and returns the corresponding JavaScript value.

**Parameters:**

- `source` - A string containing the YAY document (UTF-8)
- `filename` - Optional filename for error messages

**Returns:** A JavaScript value representing the parsed document.

**Throws:** `Error` with line:column location if parsing fails.

## Type Mapping

| YAY Type | JavaScript Type |
|----------|-----------------|
| `null` | `null` |
| big integer | `bigint` |
| float64 | `number` |
| boolean | `boolean` |
| string | `string` |
| array | `Array` |
| object | `Object` |
| bytes | `Uint8Array` |

# YAY Format

## Null

The keyword `null` denotes a null value.

[null-literal.yay](https://github.com/kriskowal/yay/blob/main/test/yay/null-literal.yay)
```yay
null
```

[null-literal.js](https://github.com/kriskowal/yay/blob/main/test/js/null-literal.js)
```js
null
```

## Booleans

The literals `true` and `false` denote booleans.

A true boolean value.

[boolean-true.yay](https://github.com/kriskowal/yay/blob/main/test/yay/boolean-true.yay)
```yay
true
```

[boolean-true.js](https://github.com/kriskowal/yay/blob/main/test/js/boolean-true.js)
```js
true
```

A false boolean value.

[boolean-false.yay](https://github.com/kriskowal/yay/blob/main/test/yay/boolean-false.yay)
```yay
false
```

[boolean-false.js](https://github.com/kriskowal/yay/blob/main/test/js/boolean-false.js)
```js
false
```

## Big Integers

Unquoted decimal digit sequences are big integers (arbitrary precision).
A leading minus sign denotes a negative big integer; the minus must not be followed by a space.
Spaces may be used to group digits for readability; they do not change the value.

A basic positive integer.

[integer-big-basic.yay](https://github.com/kriskowal/yay/blob/main/test/yay/integer-big-basic.yay)
```yay
42
```

[integer-big-basic.js](https://github.com/kriskowal/yay/blob/main/test/js/integer-big-basic.js)
```js
42n
```

A negative integer.

[integer-big-negative.yay](https://github.com/kriskowal/yay/blob/main/test/yay/integer-big-negative.yay)
```yay
-42
```

[integer-big-negative.js](https://github.com/kriskowal/yay/blob/main/test/js/integer-big-negative.js)
```js
-42n
```

Spaces group digits for readability without changing the value.

[integer-big.yay](https://github.com/kriskowal/yay/blob/main/test/yay/integer-big.yay)
```yay
867 5309
```

[integer-big.js](https://github.com/kriskowal/yay/blob/main/test/js/integer-big.js)
```js
8675309n
```

## Floating-Point (Float64)

A decimal point must be present to distinguish a float from a big integer.
Decimal literals with a decimal point, or the keywords `infinity`, `-infinity`, and `nan`, denote 64-bit floats.
A leading minus must not be followed by a space.
Spaces may group digits.

A basic floating-point number.

[number-float.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float.yay)
```yay
6.283185307179586
```

[number-float.js](https://github.com/kriskowal/yay/blob/main/test/js/number-float.js)
```js
6.283185307179586
```

A float with a leading decimal point (no integer part).

[number-float-leading-dot.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-leading-dot.yay)
```yay
.5
```

[number-float-leading-dot.js](https://github.com/kriskowal/yay/blob/main/test/js/number-float-leading-dot.js)
```js
0.5
```

A float with a trailing decimal point (no fractional part).

[number-float-trailing-dot.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-trailing-dot.yay)
```yay
1.
```

[number-float-trailing-dot.js](https://github.com/kriskowal/yay/blob/main/test/js/number-float-trailing-dot.js)
```js
1
```

Negative zero is distinct from positive zero.

[number-float-negative-zero.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-negative-zero.yay)
```yay
-0.0
```

[number-float-negative-zero.js](https://github.com/kriskowal/yay/blob/main/test/js/number-float-negative-zero.js)
```js
-0
```

Positive infinity.

[number-float-infinity.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-infinity.yay)
```yay
infinity
```

[number-float-infinity.js](https://github.com/kriskowal/yay/blob/main/test/js/number-float-infinity.js)
```js
Infinity
```

Negative infinity.

[number-float-negative-infinity.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-negative-infinity.yay)
```yay
-infinity
```

[number-float-negative-infinity.js](https://github.com/kriskowal/yay/blob/main/test/js/number-float-negative-infinity.js)
```js
-Infinity
```

Not-a-number (canonical NaN).

[number-float-nan.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-nan.yay)
```yay
nan
```

[number-float-nan.js](https://github.com/kriskowal/yay/blob/main/test/js/number-float-nan.js)
```js
NaN
```

Spaces group digits for readability in floats.

[number-float-grouped.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-grouped.yay)
```yay
6.283 185 307 179 586
```

[number-float-grouped.js](https://github.com/kriskowal/yay/blob/main/test/js/number-float-grouped.js)
```js
6.283185307179586
```

## Block Strings

Block strings use the backtick (`` ` ``) introducer.
The body continues until the next line that is indented the same or less.
The first two spaces of each content line are stripped; any further indentation is preserved.
Empty lines are interpreted as newlines.
Trailing empty lines collapse to a single trailing newline.
Block strings do not support escape sequences—a backslash is just a backslash.
Comments are also not recognized inside block strings; `#` is literal content.

### At Root Level

At root level or as an array item, content may appear on the same line after `` ` `` (backtick + space + content).
When the backtick is alone on a line, the result has an implicit leading newline.

Content on the same line as the backtick starts the string without a leading newline.

[string-block-root-same-line.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-root-same-line.yay)
```yay
` I think you ought to know I'm feeling very depressed.
  This will all end in tears.
```

[string-block-root-same-line.js](https://github.com/kriskowal/yay/blob/main/test/js/string-block-root-same-line.js)
```js
"I think you ought to know I'm feeling very depressed.\nThis will all end in tears.\n"
```

Backtick alone on its line produces a leading newline because content starts on the following line.

[string-block-root-next-line.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-root-next-line.yay)
```yay
`
  I've calculated your chance of survival,
  but I don't think you'll like it.
```

[string-block-root-next-line.js](https://github.com/kriskowal/yay/blob/main/test/js/string-block-root-next-line.js)
```js
"\nI've calculated your chance of survival,\nbut I don't think you'll like it.\n"
```

An empty line in the middle of a block string is preserved as a newline.

[string-block-empty-middle.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-empty-middle.yay)
```yay
`
  I'm getting better!

  No you're not.
```

[string-block-empty-middle.js](https://github.com/kriskowal/yay/blob/main/test/js/string-block-empty-middle.js)
```js
"\nI'm getting better!\n\nNo you're not.\n"
```

The `#` character inside a block string is literal content, not a comment.

[string-block-root-hash.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-root-hash.yay)
```yay
` # this is not a comment
  it is content
```

[string-block-root-hash.js](https://github.com/kriskowal/yay/blob/main/test/js/string-block-root-hash.js)
```js
"# this is not a comment\nit is content\n"
```

A block string may be deeply nested and the indentation prefix will be absent
in the value.
The string will end with a single newline regardless of any subsequent newlines
in the YAY text.

[string-block-nested-in-object-and-array.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-nested-in-object-and-array.yay)
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

[string-block-nested-in-object-and-array.js](https://github.com/kriskowal/yay/blob/main/test/js/string-block-nested-in-object-and-array.js)
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

In property context, the backtick must be alone on the line (no content after it).
There is no implicit leading newline.
The first content line becomes the start of the string.

[string-block-property.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-property.yay)
```yay
message: `
  By Grabthar's hammer, we live to tell the tale.
```

[string-block-property.js](https://github.com/kriskowal/yay/blob/main/test/js/string-block-property.js)
```js
({
  "message": "By Grabthar's hammer, we live to tell the tale.\n",
})
```

An empty line in the middle of a block string property is preserved.

[string-block-property-empty-middle.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-property-empty-middle.yay)
```yay
message: `
  It's not pining!

  It's passed on! This parrot is no more!
```

[string-block-property-empty-middle.js](https://github.com/kriskowal/yay/blob/main/test/js/string-block-property-empty-middle.js)
```js
({
  "message": "It's not pining!\n\nIt's passed on! This parrot is no more!\n",
})
```

A block string property followed by another property: the block ends when a line at the same or lesser indent appears.
Trailing empty lines collapse to a single trailing newline.

[string-block-property-trailing-empty.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-property-trailing-empty.yay)
```yay
message: `
  By Grabthar's hammer... what a savings.


next: 1
```

[string-block-property-trailing-empty.js](https://github.com/kriskowal/yay/blob/main/test/js/string-block-property-trailing-empty.js)
```js
({
  "message": "By Grabthar's hammer... what a savings.\n",
  "next": 1n,
})
```

## Inline Strings

Strings may be quoted with double or single quotes.
Double-quoted strings support escape sequences: `\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t`, and `\u{XXXXXX}` for Unicode code points.
Single-quoted strings are literal (no escape sequences).

A double-quoted string.

[string-inline-doublequote-basic.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-doublequote-basic.yay)
```yay
"This will all end in tears."
```

[string-inline-doublequote-basic.js](https://github.com/kriskowal/yay/blob/main/test/js/string-inline-doublequote-basic.js)
```js
"This will all end in tears."
```

A single-quoted string (literal, no escapes).

[string-inline-singlequote-basic.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-singlequote-basic.yay)
```yay
'Are you suggesting coconuts migrate?'
```

[string-inline-singlequote-basic.js](https://github.com/kriskowal/yay/blob/main/test/js/string-inline-singlequote-basic.js)
```js
"Are you suggesting coconuts migrate?"
```

A double-quoted string with escape sequences.

[string-inline-doublequote-escapes.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-doublequote-escapes.yay)
```yay
"\"\\\/\b\f\n\r\t\u{263A}"
```

[string-inline-doublequote-escapes.js](https://github.com/kriskowal/yay/blob/main/test/js/string-inline-doublequote-escapes.js)
```js
'"\\/\b\f\n\r\t☺'
```

A double-quoted string with a Unicode emoji (literal UTF-8).

[string-inline-doublequote-unicode-emoji.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-doublequote-unicode-emoji.yay)
```yay
"😀"
```

[string-inline-doublequote-unicode-emoji.js](https://github.com/kriskowal/yay/blob/main/test/js/string-inline-doublequote-unicode-emoji.js)
```js
"😀"
```

A Unicode code point escape for a character outside the BMP (U+1F600), which requires a surrogate pair in UTF-16.
The `\u{...}` escape accepts 1 to 6 hexadecimal digits representing a Unicode code point (e.g. `\u{41}` for "A", `\u{1F600}` for "😀").
Surrogate code points (U+D800 through U+DFFF) are forbidden in `\u{...}` escapes.
Unlike JSON, the four-digit `\uXXXX` form is not supported; use `\u{XXXX}` instead.

[string-inline-doublequote-unicode-surrogate-pair.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-doublequote-unicode-surrogate-pair.yay)
```yay
"\u{1F600}"
```

[string-inline-doublequote-unicode-surrogate-pair.js](https://github.com/kriskowal/yay/blob/main/test/js/string-inline-doublequote-unicode-surrogate-pair.js)
```js
"😀"
```

## Block Arrays

Arrays are written as a sequence of items, each introduced by `- ` (dash and space).
The two-character `- ` prefix is the list marker; the value follows immediately.
Items may be nested: a bullet line whose content starts with `- ` begins an inner list.
An array may be given a name as a key followed by `:`.

A basic block array with three integer items.

[array-multiline.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-multiline.yay)
```yay
- 5
- 3
```

[array-multiline.js](https://github.com/kriskowal/yay/blob/main/test/js/array-multiline.js)
```js
[5n, 3n]
```

Nested arrays where each top-level item contains an inner array.

[array-multiline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-multiline-nested.yay)
```yay
- - "a"
  - "b"
- - 1
  - 2
```

[array-multiline-nested.js](https://github.com/kriskowal/yay/blob/main/test/js/array-multiline-nested.js)
```js
[["a", "b"], [1n, 2n]]
```

An array as the value of an object property.

[array-multiline-named.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-multiline-named.yay)
```yay
complaints:
- "I didn't vote for you."
- "Help, help, I'm being repressed!"
```

[array-multiline-named.js](https://github.com/kriskowal/yay/blob/main/test/js/array-multiline-named.js)
```js
({
  "complaints": [
    "I didn't vote for you.",
    "Help, help, I'm being repressed!",
  ],
})
```

## Inline Arrays

Inline arrays use JSON-style bracket syntax with strict spacing rules: no space after `[`, no space before `]`, exactly one space after each `,`.

A simple inline array with string values.

[array-inline-doublequote.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-doublequote.yay)
```yay
["And there was much rejoicing.", "yay."]
```

[array-inline-doublequote.js](https://github.com/kriskowal/yay/blob/main/test/js/array-inline-doublequote.js)
```js
["And there was much rejoicing.", "yay."]
```

An inline array containing big integers.

[array-inline-integers.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-integers.yay)
```yay
[42, 404, 418]
```

[array-inline-integers.js](https://github.com/kriskowal/yay/blob/main/test/js/array-inline-integers.js)
```js
[42n, 404n, 418n]
```

An inline array containing byte array literals.

[array-inline-bytearray.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-bytearray.yay)
```yay
[<b0b5>, <cafe>]
```

[array-inline-bytearray.js](https://github.com/kriskowal/yay/blob/main/test/js/array-inline-bytearray.js)
```js
[
  Uint8Array.from([0xb0, 0xb5]),
  Uint8Array.from([0xca, 0xfe]),
]
```

Inline arrays nested within an inline array.

[array-inline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-nested.yay)
```yay
[["I feel happy!", "yay."], ["And there was much rejoicing.", "yay."]]
```

[array-inline-nested.js](https://github.com/kriskowal/yay/blob/main/test/js/array-inline-nested.js)
```js
[
  ["I feel happy!", "yay."],
  ["And there was much rejoicing.", "yay."],
]
```

## Block Objects

Objects are key–value pairs.
A key is followed by `:` and then the value.
Object keys must be either alphanumeric (letters and digits) or quoted (double or single quotes, same rules as string values).
Nested objects are indented by two spaces.
Empty objects are written as `key: {}`.

A simple object with two key-value pairs at the root level.

[object-multiline.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-multiline.yay)
```yay
answer: 42
error: 404
```

[object-multiline.js](https://github.com/kriskowal/yay/blob/main/test/js/object-multiline.js)
```js
({ "answer": 42n, "error": 404n })
```

An object nested within another object, demonstrating indentation.

[object-multiline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-multiline-nested.yay)
```yay
parrot:
  status: "pining for the fjords"
  plumage: "beautiful"
```

[object-multiline-nested.js](https://github.com/kriskowal/yay/blob/main/test/js/object-multiline-nested.js)
```js
({
  "parrot": { "plumage": "beautiful", "status": "pining for the fjords" },
})
```

Object keys containing spaces or special characters must be quoted.

[object-multiline-doublequote-key.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-multiline-doublequote-key.yay)
```yay
"key name": 1
```

[object-multiline-doublequote-key.js](https://github.com/kriskowal/yay/blob/main/test/js/object-multiline-doublequote-key.js)
```js
({ "key name": 1n })
```

An empty object as a property value.

[object-inline-empty.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-empty.yay)
```yay
empty: {}
```

[object-inline-empty.js](https://github.com/kriskowal/yay/blob/main/test/js/object-inline-empty.js)
```js
({ "empty": {} })
```

## Inline Objects

Inline objects use JSON-style brace syntax with strict spacing rules: no space after `{`, no space before `}`, exactly one space after each `,`.

A simple inline object with integer values.

[object-inline-integers.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-integers.yay)
```yay
{answer: 42, error: 404}
```

[object-inline-integers.js](https://github.com/kriskowal/yay/blob/main/test/js/object-inline-integers.js)
```js
({ "answer": 42n, "error": 404n })
```

An inline object with string and integer values.

[object-inline-mixed.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-mixed.yay)
```yay
{name: 'Marvin', mood: 'depressed'}
```

[object-inline-mixed.js](https://github.com/kriskowal/yay/blob/main/test/js/object-inline-mixed.js)
```js
({ "mood": "depressed", "name": "Marvin" })
```

An inline object containing both a nested object and an array.

[object-inline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-nested.yay)
```yay
{luggage: {combination: 12345}, air: ["canned", "Perri-Air"]}
```

[object-inline-nested.js](https://github.com/kriskowal/yay/blob/main/test/js/object-inline-nested.js)
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

### At Root Level

At root level or as an array item, hex may appear on the same line after `> `.
The `> ` leader must have hex digits or a comment on the line—`>` alone is invalid.

A block byte array at root level with hex on the same line as the `>` introducer.

[bytearray-block-basic.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-basic.yay)
```yay
> b0b5
  c0ff
```

[bytearray-block-basic.js](https://github.com/kriskowal/yay/blob/main/test/js/bytearray-block-basic.js)
```js
Uint8Array.from([0xb0, 0xb5, 0xc0, 0xff])
```

A block byte array with a comment on the first line instead of hex.

[bytearray-block-comment-only.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-comment-only.yay)
```yay
> # header comment
  b0b5 c0ff
```

[bytearray-block-comment-only.js](https://github.com/kriskowal/yay/blob/main/test/js/bytearray-block-comment-only.js)
```js
Uint8Array.from([0xb0, 0xb5, 0xc0, 0xff])
```

Hex and comments on the same lines for inline documentation of byte sequences.

[bytearray-block-hex-and-comment.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-hex-and-comment.yay)
```yay
> b0b5 # first chunk
  c0ff # second chunk
```

[bytearray-block-hex-and-comment.js](https://github.com/kriskowal/yay/blob/main/test/js/bytearray-block-hex-and-comment.js)
```js
Uint8Array.from([0xb0, 0xb5, 0xc0, 0xff])
```

### As Object Property

In property context, the `>` must be alone on the line (optionally followed by a comment).
Hex content starts on the following lines, preserving column alignment.

[bytearray-block-property.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-property.yay)
```yay
data: >
  b0b5 c0ff
  eefa cade
```

[bytearray-block-property.js](https://github.com/kriskowal/yay/blob/main/test/js/bytearray-block-property.js)
```js
({
  "data": Uint8Array.from([0xb0, 0xb5, 0xc0, 0xff, 0xee, 0xfa, 0xca, 0xde]),
})
```

A block byte array property with a comment on the leader line.

[bytearray-block-property-comment.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-property-comment.yay)
```yay
data: > # raw bytes
  b0b5 c0ff
```

[bytearray-block-property-comment.js](https://github.com/kriskowal/yay/blob/main/test/js/bytearray-block-property-comment.js)
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

[bytearray-inline-empty.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-inline-empty.yay)
```yay
<>
```

[bytearray-inline-empty.js](https://github.com/kriskowal/yay/blob/main/test/js/bytearray-inline-empty.js)
```js
new Uint8Array(0)
```

An inline byte array with hex content.

[bytearray-inline-even.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-inline-even.yay)
```yay
<b0b5c0ffeefacade>
```

[bytearray-inline-even.js](https://github.com/kriskowal/yay/blob/main/test/js/bytearray-inline-even.js)
```js
Uint8Array.from([0xb0, 0xb5, 0xc0, 0xff, 0xee, 0xfa, 0xca, 0xde])
```

An inline byte array as an object property value.

[bytearray-inline-named.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-inline-named.yay)
```yay
data: <b0b5c0ffeefacade>
```

[bytearray-inline-named.js](https://github.com/kriskowal/yay/blob/main/test/js/bytearray-inline-named.js)
```js
({
  "data": Uint8Array.from([0xb0, 0xb5, 0xc0, 0xff, 0xee, 0xfa, 0xca, 0xde]),
})
```

## Error Handling

Errors include line and column numbers for debugging:

```js
try {
  parseYay('invalid: [', 'config.yay');
} catch (e) {
  console.error(e.message);
  // "Unexpected newline in inline array at 1:11 of <config.yay>"
}
```

## Whitespace Rules

YAY has strict whitespace rules that the parser enforces:

- Two spaces for indentation (tabs are illegal)
- No trailing spaces on lines
- Exactly one space after `:` in key-value pairs
- Exactly one space after `,` in inline arrays/objects
- No spaces after `[` or `{`, or before `]` or `}`
- No space before `:` in keys

## Running Tests

```bash
cd js
node --test yay.test.js
```

The test runner uses fixture files from `../test/`.
Files with `.yay` extension contain YAY input.
Files with `.js` extension contain expected JavaScript output.

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
