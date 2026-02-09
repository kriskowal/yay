# YAY Parser for Scheme

A parser for the [YAY](https://github.com/kriskowal/yay) data format, implemented in Guile Scheme.

## Requirements

- [GNU Guile](https://www.gnu.org/software/guile/) 3.0 or later

## Usage

```scheme
(load "yay-parser.scm")

;; Parse a YAY document
(define result (parse-yay "key: \"value\""))
;; => (("key" . "value"))

;; Convert a parsed value to Scheme string representation
(define output (yay->scheme-string result))
;; => "((\"key\" . \"value\"))"
```

## API

### `(parse-yay source)`

Parses a YAY document string and returns the corresponding Scheme value.

**Parameters:**

- `source` - A string containing the YAY document

**Returns:** A Scheme value representing the parsed document.

### `(yay->scheme-string value)`

Converts a parsed YAY value to its Scheme string representation.

## Type Mapping

| YAY Type | Scheme Type | Notes |
|----------|-------------|-------|
| `null` | `'null` | Symbol |
| big integer | exact integer | Arbitrary precision |
| float64 | inexact number | Including `+inf.0`, `-inf.0`, `+nan.0` |
| boolean | `#t` / `#f` | |
| string | string | |
| array | vector | `#(...)` |
| object | alist | `((key . value) ...)` |
| bytes | tagged list | `(bytevector b1 b2 ...)` |

# YAY Format

[at-a-glance.yay](https://github.com/kriskowal/yay/blob/main/test/yay/at-a-glance.yay)
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

## Null

The keyword `null` denotes a null value.

[null-literal.yay](https://github.com/kriskowal/yay/blob/main/test/yay/null-literal.yay)
```yay
null
```

[null-literal.scm](https://github.com/kriskowal/yay/blob/main/test/scm/null-literal.scm)
```scheme
'null
```

## Booleans

The literals `true` and `false` denote booleans.

A true boolean value.

[boolean-true.yay](https://github.com/kriskowal/yay/blob/main/test/yay/boolean-true.yay)
```yay
true
```

[boolean-true.scm](https://github.com/kriskowal/yay/blob/main/test/scm/boolean-true.scm)
```scheme
#t
```

A false boolean value.

[boolean-false.yay](https://github.com/kriskowal/yay/blob/main/test/yay/boolean-false.yay)
```yay
false
```

[boolean-false.scm](https://github.com/kriskowal/yay/blob/main/test/scm/boolean-false.scm)
```scheme
#f
```

## Big Integers

Unquoted decimal digit sequences are big integers (arbitrary precision).
A leading minus sign denotes a negative big integer; the minus must not be followed by a space.
Spaces may be used to group digits for readability; they do not change the value.

Scheme's exact integers are arbitrary precision, so YAY big integers map directly.

A basic positive integer.

[integer-big-basic.yay](https://github.com/kriskowal/yay/blob/main/test/yay/integer-big-basic.yay)
```yay
42
```

[integer-big-basic.scm](https://github.com/kriskowal/yay/blob/main/test/scm/integer-big-basic.scm)
```scheme
42
```

A negative integer.

[integer-big-negative.yay](https://github.com/kriskowal/yay/blob/main/test/yay/integer-big-negative.yay)
```yay
-42
```

[integer-big-negative.scm](https://github.com/kriskowal/yay/blob/main/test/scm/integer-big-negative.scm)
```scheme
-42
```

Spaces group digits for readability without changing the value.

[integer-big.yay](https://github.com/kriskowal/yay/blob/main/test/yay/integer-big.yay)
```yay
867 5309
```

[integer-big.scm](https://github.com/kriskowal/yay/blob/main/test/scm/integer-big.scm)
```scheme
8675309
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

[number-float.scm](https://github.com/kriskowal/yay/blob/main/test/scm/number-float.scm)
```scheme
6.283185307179586
```

A float with a leading decimal point (no integer part).

[number-float-leading-dot.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-leading-dot.yay)
```yay
.5
```

[number-float-leading-dot.scm](https://github.com/kriskowal/yay/blob/main/test/scm/number-float-leading-dot.scm)
```scheme
0.5
```

A float with a trailing decimal point (no fractional part).

[number-float-trailing-dot.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-trailing-dot.yay)
```yay
1.
```

[number-float-trailing-dot.scm](https://github.com/kriskowal/yay/blob/main/test/scm/number-float-trailing-dot.scm)
```scheme
1.0
```

Negative zero is distinct from positive zero.

[number-float-negative-zero.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-negative-zero.yay)
```yay
-0.0
```

[number-float-negative-zero.scm](https://github.com/kriskowal/yay/blob/main/test/scm/number-float-negative-zero.scm)
```scheme
-0.0
```

Positive infinity.

[number-float-infinity.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-infinity.yay)
```yay
infinity
```

[number-float-infinity.scm](https://github.com/kriskowal/yay/blob/main/test/scm/number-float-infinity.scm)
```scheme
+inf.0
```

Negative infinity.

[number-float-negative-infinity.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-negative-infinity.yay)
```yay
-infinity
```

[number-float-negative-infinity.scm](https://github.com/kriskowal/yay/blob/main/test/scm/number-float-negative-infinity.scm)
```scheme
-inf.0
```

Not-a-number (canonical NaN).

[number-float-nan.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-nan.yay)
```yay
nan
```

[number-float-nan.scm](https://github.com/kriskowal/yay/blob/main/test/scm/number-float-nan.scm)
```scheme
+nan.0
```

Spaces group digits for readability in floats.

[number-float-grouped.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-grouped.yay)
```yay
6.283 185 307 179 586
```

[number-float-grouped.scm](https://github.com/kriskowal/yay/blob/main/test/scm/number-float-grouped.scm)
```scheme
6.283185307179586
```

Scientific notation using `e` or `E` for the exponent (Avogadro's number).

[number-float-avogadro.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-avogadro.yay)
```yay
6.022e23
```

[number-float-avogadro.scm](https://github.com/kriskowal/yay/blob/main/test/scm/number-float-avogadro.scm)
```scheme
602200000000000000000000.0
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

[string-block-root-same-line.scm](https://github.com/kriskowal/yay/blob/main/test/scm/string-block-root-same-line.scm)
```scheme
"I think you ought to know I'm feeling very depressed.\nThis will all end in tears.\n"
```

Backtick alone on its line produces a leading newline because content starts on the following line.

[string-block-root-next-line.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-root-next-line.yay)
```yay
`
  I've calculated your chance of survival,
  but I don't think you'll like it.
```

[string-block-root-next-line.scm](https://github.com/kriskowal/yay/blob/main/test/scm/string-block-root-next-line.scm)
```scheme
"\nI've calculated your chance of survival,\nbut I don't think you'll like it.\n"
```

An empty line in the middle of a block string is preserved as a newline.

[string-block-empty-middle.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-empty-middle.yay)
```yay
`
  I'm getting better!

  No you're not.
```

[string-block-empty-middle.scm](https://github.com/kriskowal/yay/blob/main/test/scm/string-block-empty-middle.scm)
```scheme
"\nI'm getting better!\n\nNo you're not.\n"
```

The `#` character inside a block string is literal content, not a comment.

[string-block-root-hash.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-root-hash.yay)
```yay
` # this is not a comment
  it is content
```

[string-block-root-hash.scm](https://github.com/kriskowal/yay/blob/main/test/scm/string-block-root-hash.scm)
```scheme
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

[string-block-nested-in-object-and-array.scm](https://github.com/kriskowal/yay/blob/main/test/scm/string-block-nested-in-object-and-array.scm)
```scheme
(("parrot" . (("condition" . "No, no, it's just resting!\n") ("remarks" . #("Remarkable bird, the Norwegian Blue.\nBeautiful plumage, innit?\n" "It's probably pining for the fjords.\nLovely plumage.\n")))))
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

[string-block-property.scm](https://github.com/kriskowal/yay/blob/main/test/scm/string-block-property.scm)
```scheme
(("message" . "By Grabthar's hammer, we live to tell the tale.\n"))
```

An empty line in the middle of a block string property is preserved.

[string-block-property-empty-middle.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-property-empty-middle.yay)
```yay
message: `
  It's not pining!

  It's passed on! This parrot is no more!
```

[string-block-property-empty-middle.scm](https://github.com/kriskowal/yay/blob/main/test/scm/string-block-property-empty-middle.scm)
```scheme
(("message" . "It's not pining!\n\nIt's passed on! This parrot is no more!\n"))
```

A block string property followed by another property: the block ends when a line at the same or lesser indent appears.
Trailing empty lines collapse to a single trailing newline.

[string-block-property-trailing-empty.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-property-trailing-empty.yay)
```yay
message: `
  By Grabthar's hammer... what a savings.


next: 1
```

[string-block-property-trailing-empty.scm](https://github.com/kriskowal/yay/blob/main/test/scm/string-block-property-trailing-empty.scm)
```scheme
(("message" . "By Grabthar's hammer... what a savings.\n") ("next" . 1))
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

[string-inline-doublequote-basic.scm](https://github.com/kriskowal/yay/blob/main/test/scm/string-inline-doublequote-basic.scm)
```scheme
"This will all end in tears."
```

A single-quoted string (literal, no escapes).

[string-inline-singlequote-basic.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-singlequote-basic.yay)
```yay
'Are you suggesting coconuts migrate?'
```

[string-inline-singlequote-basic.scm](https://github.com/kriskowal/yay/blob/main/test/scm/string-inline-singlequote-basic.scm)
```scheme
"Are you suggesting coconuts migrate?"
```

A double-quoted string with escape sequences.

[string-inline-doublequote-escapes.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-doublequote-escapes.yay)
```yay
"\"\\\/\b\f\n\r\t\u{263A}"
```

[string-inline-doublequote-escapes.scm](https://github.com/kriskowal/yay/blob/main/test/scm/string-inline-doublequote-escapes.scm)
```scheme
"\"\\\/\b\f\n\r\t\u263A"
```

A double-quoted string with a Unicode emoji (literal UTF-8).

[string-inline-doublequote-unicode-emoji.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-doublequote-unicode-emoji.yay)
```yay
"😀"
```

[string-inline-doublequote-unicode-emoji.scm](https://github.com/kriskowal/yay/blob/main/test/scm/string-inline-doublequote-unicode-emoji.scm)
```scheme
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

[string-inline-doublequote-unicode-surrogate-pair.scm](https://github.com/kriskowal/yay/blob/main/test/scm/string-inline-doublequote-unicode-surrogate-pair.scm)
```scheme
"😀"
```

## Concatenated Strings (Quoted Lines)

Multiple quoted strings on consecutive lines are concatenated into a single
string.
This is useful for breaking long strings across lines without introducing
newlines in the result, or including visibly escaped characters like tab
that are otherwise forbidden in string blocks.

[string-multiline-concat.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-multiline-concat.yay)
```yay
confession:
  "I'm not dead yet. "
  "I feel happy!"
```

[string-multiline-concat.scm](https://github.com/kriskowal/yay/blob/main/test/scm/string-multiline-concat.scm)
```scheme
(("confession" . "I'm not dead yet. I feel happy!"))
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

[array-multiline.scm](https://github.com/kriskowal/yay/blob/main/test/scm/array-multiline.scm)
```scheme
#(5 3)
```

Nested arrays where each top-level item contains an inner array.

[array-multiline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-multiline-nested.yay)
```yay
- - "a"
  - "b"
- - 1
  - 2
```

[array-multiline-nested.scm](https://github.com/kriskowal/yay/blob/main/test/scm/array-multiline-nested.scm)
```scheme
#(#("a" "b") #(1 2))
```

An array as the value of an object property.

[array-multiline-named.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-multiline-named.yay)
```yay
complaints:
- "I didn't vote for you."
- "Help, help, I'm being repressed!"
```

[array-multiline-named.scm](https://github.com/kriskowal/yay/blob/main/test/scm/array-multiline-named.scm)
```scheme
(("complaints" . #("I didn't vote for you." "Help, help, I'm being repressed!")))
```

## Inline Arrays

Inline arrays use JSON-style bracket syntax with strict spacing rules: no space after `[`, no space before `]`, exactly one space after each `,`.

[array-inline-doublequote.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-doublequote.yay)
```yay
["And there was much rejoicing.", "yay."]
```

[array-inline-doublequote.scm](https://github.com/kriskowal/yay/blob/main/test/scm/array-inline-doublequote.scm)
```scheme
#("And there was much rejoicing." "yay.")
```

[array-inline-integers.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-integers.yay)
```yay
[42, 404, 418]
```

[array-inline-integers.scm](https://github.com/kriskowal/yay/blob/main/test/scm/array-inline-integers.scm)
```scheme
#(42 404 418)
```

[array-inline-bytearray.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-bytearray.yay)
```yay
[<b0b5>, <cafe>]
```

[array-inline-bytearray.scm](https://github.com/kriskowal/yay/blob/main/test/scm/array-inline-bytearray.scm)
```scheme
#((bytevector 176 181) (bytevector 202 254))
```

[array-inline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-nested.yay)
```yay
[["I feel happy!", "yay."], ["And there was much rejoicing.", "yay."]]
```

[array-inline-nested.scm](https://github.com/kriskowal/yay/blob/main/test/scm/array-inline-nested.scm)
```scheme
#(#("I feel happy!" "yay.") #("And there was much rejoicing." "yay."))
```

## Block Objects

Objects are key–value pairs.
A key is followed by `:` and then the value.
Object keys must be either alphanumeric or quoted.
Nested objects are indented by two spaces.

[object-multiline.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-multiline.yay)
```yay
answer: 42
error: 404
```

[object-multiline.scm](https://github.com/kriskowal/yay/blob/main/test/scm/object-multiline.scm)
```scheme
(("answer" . 42) ("error" . 404))
```

Nested object.

[object-multiline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-multiline-nested.yay)
```yay
parrot:
  status: "pining for the fjords"
  plumage: "beautiful"
```

[object-multiline-nested.scm](https://github.com/kriskowal/yay/blob/main/test/scm/object-multiline-nested.scm)
```scheme
(("parrot" . (("plumage" . "beautiful") ("status" . "pining for the fjords"))))
```

Object keys containing spaces or special characters must be quoted.

[object-multiline-doublequote-key.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-multiline-doublequote-key.yay)
```yay
"key name": 1
```

[object-multiline-doublequote-key.scm](https://github.com/kriskowal/yay/blob/main/test/scm/object-multiline-doublequote-key.scm)
```scheme
(("key name" . 1))
```

[object-inline-empty.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-empty.yay)
```yay
empty: {}
```

[object-inline-empty.scm](https://github.com/kriskowal/yay/blob/main/test/scm/object-inline-empty.scm)
```scheme
(("empty" . ()))
```

## Inline Objects

Inline objects use JSON-style brace syntax with strict spacing rules.

[object-inline-integers.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-integers.yay)
```yay
{answer: 42, error: 404}
```

[object-inline-integers.scm](https://github.com/kriskowal/yay/blob/main/test/scm/object-inline-integers.scm)
```scheme
(("answer" . 42) ("error" . 404))
```

[object-inline-mixed.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-mixed.yay)
```yay
{name: 'Marvin', mood: 'depressed'}
```

[object-inline-mixed.scm](https://github.com/kriskowal/yay/blob/main/test/scm/object-inline-mixed.scm)
```scheme
(("mood" . "depressed") ("name" . "Marvin"))
```

[object-inline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-nested.yay)
```yay
{luggage: {combination: 12345}, air: ["canned", "Perri-Air"]}
```

[object-inline-nested.scm](https://github.com/kriskowal/yay/blob/main/test/scm/object-inline-nested.scm)
```scheme
(("air" . #("canned" "Perri-Air")) ("luggage" . (("combination" . 12345))))
```

## Block Byte Arrays

Block byte arrays use the `>` introducer.
Each line may hold hex chunks and comments.

At root level, hex may appear on the same line after `> `.

[bytearray-block-basic.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-basic.yay)
```yay
> b0b5
  c0ff
```

[bytearray-block-basic.scm](https://github.com/kriskowal/yay/blob/main/test/scm/bytearray-block-basic.scm)
```scheme
(bytevector 176 181 192 255)
```

[bytearray-block-comment-only.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-comment-only.yay)
```yay
> # header comment
  b0b5 c0ff
```

[bytearray-block-comment-only.scm](https://github.com/kriskowal/yay/blob/main/test/scm/bytearray-block-comment-only.scm)
```scheme
(bytevector 176 181 192 255)
```

[bytearray-block-hex-and-comment.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-hex-and-comment.yay)
```yay
> b0b5 # first chunk
  c0ff # second chunk
```

[bytearray-block-hex-and-comment.scm](https://github.com/kriskowal/yay/blob/main/test/scm/bytearray-block-hex-and-comment.scm)
```scheme
(bytevector 176 181 192 255)
```

In property context, `>` must be followed only by a comment or newline.

[bytearray-block-property.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-property.yay)
```yay
data: >
  b0b5 c0ff
  eefa cade
```

[bytearray-block-property.scm](https://github.com/kriskowal/yay/blob/main/test/scm/bytearray-block-property.scm)
```scheme
(("data" . (bytevector 176 181 192 255 238 250 202 222)))
```

[bytearray-block-property-comment.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-property-comment.yay)
```yay
data: > # raw bytes
  b0b5 c0ff
```

[bytearray-block-property-comment.scm](https://github.com/kriskowal/yay/blob/main/test/scm/bytearray-block-property-comment.scm)
```scheme
(("data" . (bytevector 176 181 192 255)))
```

## Inline Byte Arrays

Binary data is written as hexadecimal inside angle brackets.
Hexadecimal must be lowercase.

[bytearray-inline-empty.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-inline-empty.yay)
```yay
<>
```

[bytearray-inline-empty.scm](https://github.com/kriskowal/yay/blob/main/test/scm/bytearray-inline-empty.scm)
```scheme
(bytevector)
```

[bytearray-inline-even.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-inline-even.yay)
```yay
<b0b5c0ffeefacade>
```

[bytearray-inline-even.scm](https://github.com/kriskowal/yay/blob/main/test/scm/bytearray-inline-even.scm)
```scheme
(bytevector 176 181 192 255 238 250 202 222)
```

[bytearray-inline-named.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-inline-named.yay)
```yay
data: <b0b5c0ffeefacade>
```

[bytearray-inline-named.scm](https://github.com/kriskowal/yay/blob/main/test/scm/bytearray-inline-named.scm)
```scheme
(("data" . (bytevector 176 181 192 255 238 250 202 222)))
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
cd scm
guile --no-auto-compile run-tests.scm
```

The test runner uses fixture files from `../test/`.
Files with `.yay` extension contain YAY input.
Files with `.scm` extension contain expected Scheme output.

## Limitations

The parser is designed for Guile Scheme and uses Guile-specific features like `(ice-9 receive)` for multiple return values.

Error messages are minimal and don't include line/column information.

The `yay->scheme-string` output is not pretty-printed.

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
