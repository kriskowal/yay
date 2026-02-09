# YAY Parser for C

A parser for the [YAY](https://github.com/kriskowal/yay) data format, implemented in C (C11).

## Requirements

- C11 compatible compiler (GCC, Clang, MSVC)
- Standard C library

## Installation

Copy `yay.h` and `yay.c` to your project.

## Usage

```c
#include <stdio.h>
#include "yay.h"

int main(void) {
    // Parse a YAY document
    yay_result_t result = yay_parse("key: \"value\"", 0, NULL);
    if (result.error) {
        fprintf(stderr, "Error: %s at %d:%d\n",
                result.error->message,
                result.error->line,
                result.error->column);
        yay_result_free(&result);
        return 1;
    }

    // Use the parsed value
    char *str = yay_to_string(result.value);
    printf("%s\n", str);
    free(str);

    // Clean up
    yay_result_free(&result);
    return 0;
}
```

## API

### `yay_parse(source, length, filename)`

Parses YAY-encoded data and returns a result containing either a value or an error.

### `yay_free(value)`

Frees a YAY value and all its children.

### `yay_error_free(error)`

Frees a YAY error.

### `yay_result_free(result)`

Frees a parse result (both value and error if present).

## Working with Values

A `yay_result_t` contains either a `value` or an `error`.
After checking for errors, use the `type` field to inspect a value and the
`data` union to extract its contents.

### Checking Types

```c
yay_value_t *v = result.value;
switch (v->type) {
case YAY_NULL:    printf("null\n"); break;
case YAY_BOOL:    printf("bool: %s\n", v->data.boolean ? "true" : "false"); break;
case YAY_INT:     printf("int: %s%s\n", v->data.bigint.negative ? "-" : "", v->data.bigint.digits); break;
case YAY_FLOAT:   printf("float: %g\n", v->data.number); break;
case YAY_STRING:  printf("string: %s\n", v->data.string); break;
case YAY_BYTES:   printf("bytes: %zu bytes\n", v->data.bytes.length); break;
case YAY_ARRAY:   printf("array: %zu items\n", v->data.array.length); break;
case YAY_OBJECT:  printf("object: %zu pairs\n", v->data.object.length); break;
}
```

### Extracting Primitives

```c
// Boolean
bool flag = v->data.boolean;

// Integer (arbitrary precision, stored as a digit string)
const char *digits = v->data.bigint.digits;
bool negative = v->data.bigint.negative;

// Float
double num = v->data.number;

// String (null-terminated UTF-8)
const char *str = v->data.string;

// Bytes
const uint8_t *buf = v->data.bytes.data;
size_t len = v->data.bytes.length;
```

### Walking Arrays

```c
for (size_t i = 0; i < v->data.array.length; i++) {
    yay_value_t *item = v->data.array.items[i];
    printf("[%zu] type=%d\n", i, item->type);
}
```

### Walking Objects

```c
for (size_t i = 0; i < v->data.object.length; i++) {
    const char *key = v->data.object.pairs[i].key;
    yay_value_t *val = v->data.object.pairs[i].value;
    printf("%s: type=%d\n", key, val->type);
}
```

### Looking Up a Key

There is no built-in lookup function; scan the pairs:

```c
yay_value_t *lookup(yay_value_t *obj, const char *key) {
    for (size_t i = 0; i < obj->data.object.length; i++) {
        if (strcmp(obj->data.object.pairs[i].key, key) == 0)
            return obj->data.object.pairs[i].value;
    }
    return NULL;
}
```

## Code Generation

The [`yay` CLI](../CLI.md) can generate C expressions from YAY documents:

```bash
yay -t c data.yay
```

For example, given this YAY input:

```yay
parrot:
  status: "pining for the fjords"
  plumage: "beautiful"
```

`yay -t c` produces:

```c
YAY_OBJECT(
    "parrot", YAY_OBJECT(
        "plumage", yay_string("beautiful"),
        "status", yay_string("pining for the fjords")
    )
)
```

## Type Mapping

| YAY Type | C Type | Notes |
|----------|--------|-------|
| `null` | `YAY_NULL` | `yay_value_t` with type `YAY_NULL` |
| big integer | `yay_bigint_t` | String-based arbitrary precision |
| float64 | `double` | Including `INFINITY`, `-INFINITY`, `NAN` |
| boolean | `bool` | |
| string | `char *` | UTF-8 encoded |
| array | `yay_value_t **` | Array of value pointers |
| object | `yay_pair_t *` | Array of key-value pairs |
| bytes | `uint8_t *` | With length field |

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

[null-literal.c](https://github.com/kriskowal/yay/blob/main/test/c/null-literal.c)
```c
yay_null()
```

## Booleans

The literals `true` and `false` denote booleans.

A true boolean value.

[boolean-true.yay](https://github.com/kriskowal/yay/blob/main/test/yay/boolean-true.yay)
```yay
true
```

[boolean-true.c](https://github.com/kriskowal/yay/blob/main/test/c/boolean-true.c)
```c
yay_bool(true)
```

A false boolean value.

[boolean-false.yay](https://github.com/kriskowal/yay/blob/main/test/yay/boolean-false.yay)
```yay
false
```

[boolean-false.c](https://github.com/kriskowal/yay/blob/main/test/c/boolean-false.c)
```c
yay_bool(false)
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

[integer-big-basic.c](https://github.com/kriskowal/yay/blob/main/test/c/integer-big-basic.c)
```c
yay_int(42)
```

A negative integer.

[integer-big-negative.yay](https://github.com/kriskowal/yay/blob/main/test/yay/integer-big-negative.yay)
```yay
-42
```

[integer-big-negative.c](https://github.com/kriskowal/yay/blob/main/test/c/integer-big-negative.c)
```c
yay_int(-42)
```

Spaces group digits for readability without changing the value.

[integer-big.yay](https://github.com/kriskowal/yay/blob/main/test/yay/integer-big.yay)
```yay
867 5309
```

[integer-big.c](https://github.com/kriskowal/yay/blob/main/test/c/integer-big.c)
```c
yay_int(8675309)
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

[number-float.c](https://github.com/kriskowal/yay/blob/main/test/c/number-float.c)
```c
yay_float(6.283185307179586)
```

A float with a leading decimal point (no integer part).

[number-float-leading-dot.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-leading-dot.yay)
```yay
.5
```

[number-float-leading-dot.c](https://github.com/kriskowal/yay/blob/main/test/c/number-float-leading-dot.c)
```c
yay_float(0.5)
```

A float with a trailing decimal point (no fractional part).

[number-float-trailing-dot.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-trailing-dot.yay)
```yay
1.
```

[number-float-trailing-dot.c](https://github.com/kriskowal/yay/blob/main/test/c/number-float-trailing-dot.c)
```c
yay_float(1.0)
```

Negative zero is distinct from positive zero.

[number-float-negative-zero.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-negative-zero.yay)
```yay
-0.0
```

[number-float-negative-zero.c](https://github.com/kriskowal/yay/blob/main/test/c/number-float-negative-zero.c)
```c
yay_float(-0.0)
```

Positive infinity.

[number-float-infinity.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-infinity.yay)
```yay
infinity
```

[number-float-infinity.c](https://github.com/kriskowal/yay/blob/main/test/c/number-float-infinity.c)
```c
yay_float(INFINITY)
```

Negative infinity.

[number-float-negative-infinity.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-negative-infinity.yay)
```yay
-infinity
```

[number-float-negative-infinity.c](https://github.com/kriskowal/yay/blob/main/test/c/number-float-negative-infinity.c)
```c
yay_float(-INFINITY)
```

Not-a-number (canonical NaN).

[number-float-nan.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-nan.yay)
```yay
nan
```

[number-float-nan.c](https://github.com/kriskowal/yay/blob/main/test/c/number-float-nan.c)
```c
yay_float(NAN)
```

Spaces group digits for readability in floats.

[number-float-grouped.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-grouped.yay)
```yay
6.283 185 307 179 586
```

[number-float-grouped.c](https://github.com/kriskowal/yay/blob/main/test/c/number-float-grouped.c)
```c
yay_float(6.283185307179586)
```

Scientific notation using `e` or `E` for the exponent (Avogadro's number).

[number-float-avogadro.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-avogadro.yay)
```yay
6.022e23
```

[number-float-avogadro.c](https://github.com/kriskowal/yay/blob/main/test/c/number-float-avogadro.c)
```c
yay_float(602200000000000000000000.0)
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

[string-block-root-same-line.c](https://github.com/kriskowal/yay/blob/main/test/c/string-block-root-same-line.c)
```c
yay_string("I think you ought to know I'm feeling very depressed.\nThis will all end in tears.\n")
```

Backtick alone on its line produces a leading newline because content starts on the following line.

[string-block-root-next-line.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-root-next-line.yay)
```yay
`
  I've calculated your chance of survival,
  but I don't think you'll like it.
```

[string-block-root-next-line.c](https://github.com/kriskowal/yay/blob/main/test/c/string-block-root-next-line.c)
```c
yay_string("\nI've calculated your chance of survival,\nbut I don't think you'll like it.\n")
```

An empty line in the middle of a block string is preserved as a newline.

[string-block-empty-middle.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-empty-middle.yay)
```yay
`
  I'm getting better!

  No you're not.
```

[string-block-empty-middle.c](https://github.com/kriskowal/yay/blob/main/test/c/string-block-empty-middle.c)
```c
yay_string("\nI'm getting better!\n\nNo you're not.\n")
```

The `#` character inside a block string is literal content, not a comment.

[string-block-root-hash.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-root-hash.yay)
```yay
` # this is not a comment
  it is content
```

[string-block-root-hash.c](https://github.com/kriskowal/yay/blob/main/test/c/string-block-root-hash.c)
```c
yay_string("# this is not a comment\nit is content\n")
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

[string-block-nested-in-object-and-array.c](https://github.com/kriskowal/yay/blob/main/test/c/string-block-nested-in-object-and-array.c)
```c
YAY_OBJECT(
    "parrot", YAY_OBJECT(
        "condition", yay_string("No, no, it's just resting!\n"),
        "remarks", YAY_ARRAY(
            yay_string("Remarkable bird, the Norwegian Blue.\nBeautiful plumage, innit?\n"),
            yay_string("It's probably pining for the fjords.\nLovely plumage.\n")
        )
    )
)
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

[string-block-property.c](https://github.com/kriskowal/yay/blob/main/test/c/string-block-property.c)
```c
YAY_OBJECT(
    "message", yay_string("By Grabthar's hammer, we live to tell the tale.\n")
)
```

An empty line in the middle of a block string property is preserved.

[string-block-property-empty-middle.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-property-empty-middle.yay)
```yay
message: `
  It's not pining!

  It's passed on! This parrot is no more!
```

[string-block-property-empty-middle.c](https://github.com/kriskowal/yay/blob/main/test/c/string-block-property-empty-middle.c)
```c
YAY_OBJECT(
    "message", yay_string("It's not pining!\n\nIt's passed on! This parrot is no more!\n")
)
```

A block string property followed by another property: the block ends when a line at the same or lesser indent appears.
Trailing empty lines collapse to a single trailing newline.

[string-block-property-trailing-empty.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-property-trailing-empty.yay)
```yay
message: `
  By Grabthar's hammer... what a savings.


next: 1
```

[string-block-property-trailing-empty.c](https://github.com/kriskowal/yay/blob/main/test/c/string-block-property-trailing-empty.c)
```c
YAY_OBJECT(
    "message", yay_string("By Grabthar's hammer... what a savings.\n"),
    "next", yay_int(1)
)
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

[string-inline-doublequote-basic.c](https://github.com/kriskowal/yay/blob/main/test/c/string-inline-doublequote-basic.c)
```c
yay_string("This will all end in tears.")
```

A single-quoted string (literal, no escapes).

[string-inline-singlequote-basic.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-singlequote-basic.yay)
```yay
'Are you suggesting coconuts migrate?'
```

[string-inline-singlequote-basic.c](https://github.com/kriskowal/yay/blob/main/test/c/string-inline-singlequote-basic.c)
```c
yay_string("Are you suggesting coconuts migrate?")
```

A double-quoted string with escape sequences.

[string-inline-doublequote-escapes.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-doublequote-escapes.yay)
```yay
"\"\\\/\b\f\n\r\t\u{263A}"
```

[string-inline-doublequote-escapes.c](https://github.com/kriskowal/yay/blob/main/test/c/string-inline-doublequote-escapes.c)
```c
yay_string("\"\\/\b\f\n\r\t☺")
```

A double-quoted string with a Unicode emoji (literal UTF-8).

[string-inline-doublequote-unicode-emoji.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-doublequote-unicode-emoji.yay)
```yay
"😀"
```

[string-inline-doublequote-unicode-emoji.c](https://github.com/kriskowal/yay/blob/main/test/c/string-inline-doublequote-unicode-emoji.c)
```c
yay_string("😀")
```

A Unicode code point escape for a character outside the BMP (U+1F600), which requires a surrogate pair in UTF-16.
The `\u{...}` escape accepts 1 to 6 hexadecimal digits representing a Unicode code point (e.g. `\u{41}` for "A", `\u{1F600}` for "😀").
Surrogate code points (U+D800 through U+DFFF) are forbidden in `\u{...}` escapes.
Unlike JSON, the four-digit `\uXXXX` form is not supported; use `\u{XXXX}` instead.

[string-inline-doublequote-unicode-surrogate-pair.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-doublequote-unicode-surrogate-pair.yay)
```yay
"\u{1F600}"
```

[string-inline-doublequote-unicode-surrogate-pair.c](https://github.com/kriskowal/yay/blob/main/test/c/string-inline-doublequote-unicode-surrogate-pair.c)
```c
yay_string("😀")
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

[string-multiline-concat.c](https://github.com/kriskowal/yay/blob/main/test/c/string-multiline-concat.c)
```c
YAY_OBJECT("confession", yay_string("I'm not dead yet. I feel happy!"))
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

[array-multiline.c](https://github.com/kriskowal/yay/blob/main/test/c/array-multiline.c)
```c
YAY_ARRAY(yay_int(5), yay_int(3))
```

Nested arrays where each top-level item contains an inner array.

[array-multiline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-multiline-nested.yay)
```yay
- - "a"
  - "b"
- - 1
  - 2
```

[array-multiline-nested.c](https://github.com/kriskowal/yay/blob/main/test/c/array-multiline-nested.c)
```c
YAY_ARRAY(
    YAY_ARRAY(yay_string("a"), yay_string("b")),
    YAY_ARRAY(yay_int(1), yay_int(2))
)
```

An array as the value of an object property.

[array-multiline-named.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-multiline-named.yay)
```yay
complaints:
- "I didn't vote for you."
- "Help, help, I'm being repressed!"
```

[array-multiline-named.c](https://github.com/kriskowal/yay/blob/main/test/c/array-multiline-named.c)
```c
YAY_OBJECT(
    "complaints", YAY_ARRAY(
        yay_string("I didn't vote for you."),
        yay_string("Help, help, I'm being repressed!")
    )
)
```

## Inline Arrays

Inline arrays use JSON-style bracket syntax with strict spacing rules: no space after `[`, no space before `]`, exactly one space after each `,`.

A simple inline array with string values.

[array-inline-doublequote.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-doublequote.yay)
```yay
["And there was much rejoicing.", "yay."]
```

[array-inline-doublequote.c](https://github.com/kriskowal/yay/blob/main/test/c/array-inline-doublequote.c)
```c
YAY_ARRAY(
    yay_string("And there was much rejoicing."),
    yay_string("yay.")
)
```

An inline array containing big integers.

[array-inline-integers.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-integers.yay)
```yay
[42, 404, 418]
```

[array-inline-integers.c](https://github.com/kriskowal/yay/blob/main/test/c/array-inline-integers.c)
```c
YAY_ARRAY(yay_int(42), yay_int(404), yay_int(418))
```

An inline array containing byte array literals.

[array-inline-bytearray.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-bytearray.yay)
```yay
[<b0b5>, <cafe>]
```

[array-inline-bytearray.c](https://github.com/kriskowal/yay/blob/main/test/c/array-inline-bytearray.c)
```c
YAY_ARRAY(yay_bytes_from_hex("b0b5"), yay_bytes_from_hex("cafe"))
```

Inline arrays nested within an inline array.

[array-inline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-nested.yay)
```yay
[["I feel happy!", "yay."], ["And there was much rejoicing.", "yay."]]
```

[array-inline-nested.c](https://github.com/kriskowal/yay/blob/main/test/c/array-inline-nested.c)
```c
YAY_ARRAY(
    YAY_ARRAY(yay_string("I feel happy!"), yay_string("yay.")),
    YAY_ARRAY(
        yay_string("And there was much rejoicing."),
        yay_string("yay.")
    )
)
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

[object-multiline.c](https://github.com/kriskowal/yay/blob/main/test/c/object-multiline.c)
```c
YAY_OBJECT("answer", yay_int(42), "error", yay_int(404))
```

An object nested within another object, demonstrating indentation.

[object-multiline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-multiline-nested.yay)
```yay
parrot:
  status: "pining for the fjords"
  plumage: "beautiful"
```

[object-multiline-nested.c](https://github.com/kriskowal/yay/blob/main/test/c/object-multiline-nested.c)
```c
YAY_OBJECT(
    "parrot", YAY_OBJECT(
        "plumage", yay_string("beautiful"),
        "status", yay_string("pining for the fjords")
    )
)
```

Object keys containing spaces or special characters must be quoted.

[object-multiline-doublequote-key.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-multiline-doublequote-key.yay)
```yay
"key name": 1
```

[object-multiline-doublequote-key.c](https://github.com/kriskowal/yay/blob/main/test/c/object-multiline-doublequote-key.c)
```c
YAY_OBJECT("key name", yay_int(1))
```

An empty object as a property value.

[object-inline-empty.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-empty.yay)
```yay
empty: {}
```

[object-inline-empty.c](https://github.com/kriskowal/yay/blob/main/test/c/object-inline-empty.c)
```c
YAY_OBJECT("empty", yay_object())
```

## Inline Objects

Inline objects use JSON-style brace syntax with strict spacing rules: no space after `{`, no space before `}`, exactly one space after each `,`.

A simple inline object with integer values.

[object-inline-integers.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-integers.yay)
```yay
{answer: 42, error: 404}
```

[object-inline-integers.c](https://github.com/kriskowal/yay/blob/main/test/c/object-inline-integers.c)
```c
YAY_OBJECT("answer", yay_int(42), "error", yay_int(404))
```

An inline object with string and integer values.

[object-inline-mixed.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-mixed.yay)
```yay
{name: 'Marvin', mood: 'depressed'}
```

[object-inline-mixed.c](https://github.com/kriskowal/yay/blob/main/test/c/object-inline-mixed.c)
```c
YAY_OBJECT(
    "mood", yay_string("depressed"),
    "name", yay_string("Marvin")
)
```

An inline object containing both a nested object and an array.

[object-inline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-nested.yay)
```yay
{luggage: {combination: 12345}, air: ["canned", "Perri-Air"]}
```

[object-inline-nested.c](https://github.com/kriskowal/yay/blob/main/test/c/object-inline-nested.c)
```c
YAY_OBJECT(
    "air", YAY_ARRAY(yay_string("canned"), yay_string("Perri-Air")),
    "luggage", YAY_OBJECT("combination", yay_int(12345))
)
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

[bytearray-block-basic.c](https://github.com/kriskowal/yay/blob/main/test/c/bytearray-block-basic.c)
```c
yay_bytes_from_hex("b0b5c0ff")
```

A block byte array with a comment on the first line instead of hex.

[bytearray-block-comment-only.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-comment-only.yay)
```yay
> # header comment
  b0b5 c0ff
```

[bytearray-block-comment-only.c](https://github.com/kriskowal/yay/blob/main/test/c/bytearray-block-comment-only.c)
```c
yay_bytes_from_hex("b0b5c0ff")
```

Hex and comments on the same lines for inline documentation of byte sequences.

[bytearray-block-hex-and-comment.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-hex-and-comment.yay)
```yay
> b0b5 # first chunk
  c0ff # second chunk
```

[bytearray-block-hex-and-comment.c](https://github.com/kriskowal/yay/blob/main/test/c/bytearray-block-hex-and-comment.c)
```c
yay_bytes_from_hex("b0b5c0ff")
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

[bytearray-block-property.c](https://github.com/kriskowal/yay/blob/main/test/c/bytearray-block-property.c)
```c
YAY_OBJECT("data", yay_bytes_from_hex("b0b5c0ffeefacade"))
```

A block byte array property with a comment on the leader line.

[bytearray-block-property-comment.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-property-comment.yay)
```yay
data: > # raw bytes
  b0b5 c0ff
```

[bytearray-block-property-comment.c](https://github.com/kriskowal/yay/blob/main/test/c/bytearray-block-property-comment.c)
```c
YAY_OBJECT("data", yay_bytes_from_hex("b0b5c0ff"))
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

[bytearray-inline-empty.c](https://github.com/kriskowal/yay/blob/main/test/c/bytearray-inline-empty.c)
```c
yay_bytes_from_hex("")
```

An inline byte array with hex content.

[bytearray-inline-even.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-inline-even.yay)
```yay
<b0b5c0ffeefacade>
```

[bytearray-inline-even.c](https://github.com/kriskowal/yay/blob/main/test/c/bytearray-inline-even.c)
```c
yay_bytes_from_hex("b0b5c0ffeefacade")
```

An inline byte array as an object property value.

[bytearray-inline-named.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-inline-named.yay)
```yay
data: <b0b5c0ffeefacade>
```

[bytearray-inline-named.c](https://github.com/kriskowal/yay/blob/main/test/c/bytearray-inline-named.c)
```c
YAY_OBJECT("data", yay_bytes_from_hex("b0b5c0ffeefacade"))
```

## Error Handling

Errors include line and column numbers for debugging:

```c
yay_result_t result = yay_parse("invalid: [", 0, "config.yay");
if (result.error) {
    fprintf(stderr, "Error: %s at %d:%d\n",
            result.error->message,
            result.error->line,
            result.error->column);
    // "Error: Unexpected newline in inline array at 1:11"
}
yay_result_free(&result);
```

## Whitespace Rules

YAY has strict whitespace rules that the parser enforces:

- Two spaces for indentation (tabs are illegal)
- No trailing spaces on lines
- Exactly one space after `:` in key-value pairs
- Exactly one space after `,` in inline arrays/objects
- No spaces after `[` or `{`, or before `]` or `}`
- No space before `:` in keys

## Building

```bash
cd c
make
```

## Running Tests

```bash
cd c
make test
```

The test runner uses fixture files from `../test/`.
Files with `.yay` extension contain YAY input.
Files with `.c` extension contain expected C output.

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
