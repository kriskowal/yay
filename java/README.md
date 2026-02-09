# YAY Parser for Java

A parser for the [YAY](https://github.com/kriskowal/yay) data format, implemented in Java (17+).

## Requirements

- Java 17+

## Installation

Add the dependency to your `pom.xml`:

```xml
<dependency>
    <groupId>com.kriskowal</groupId>
    <artifactId>yay</artifactId>
    <version>1.0.0</version>
</dependency>
```

Or copy `Yay.java` to your project.

## Usage

```java
import com.kriskowal.yay.Yay;

public class Example {
    public static void main(String[] args) {
        // Parse a YAY document
        Object result = Yay.parse("key: \"value\"");
        System.out.println(result);
        // {key=value}

        // Parse with filename for error messages
        Object data = Yay.parse(source, "config.yay");
    }
}
```

## API

### `Yay.parse(String source)`

Parses YAY-encoded data and returns the result.

### `Yay.parse(String source, String filename)`

Parses YAY-encoded data with a filename for error messages.

## Type Mapping

| YAY Type | Java Type | Notes |
|----------|-----------|-------|
| `null` | `null` | |
| big integer | `BigInteger` | Arbitrary precision |
| float64 | `Double` | Including `Double.POSITIVE_INFINITY`, `Double.NEGATIVE_INFINITY`, `Double.NaN` |
| boolean | `Boolean` | |
| string | `String` | |
| array | `List<Object>` | |
| object | `Map<String, Object>` | `LinkedHashMap` preserves insertion order |
| bytes | `byte[]` | |

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

[null-literal.java](https://github.com/kriskowal/yay/blob/main/test/java/null-literal.java)
```java
null
```

## Booleans

The literals `true` and `false` denote booleans.

A true boolean value.

[boolean-true.yay](https://github.com/kriskowal/yay/blob/main/test/yay/boolean-true.yay)
```yay
true
```

[boolean-true.java](https://github.com/kriskowal/yay/blob/main/test/java/boolean-true.java)
```java
true
```

A false boolean value.

[boolean-false.yay](https://github.com/kriskowal/yay/blob/main/test/yay/boolean-false.yay)
```yay
false
```

[boolean-false.java](https://github.com/kriskowal/yay/blob/main/test/java/boolean-false.java)
```java
false
```

## Big Integers

Unquoted decimal digit sequences are big integers (arbitrary precision).
A leading minus sign denotes a negative big integer; the minus must not be followed by a space.
Spaces may be used to group digits for readability; they do not change the value.
Java's `BigInteger` provides arbitrary precision, so YAY big integers map directly.

A basic positive integer.

[integer-big-basic.yay](https://github.com/kriskowal/yay/blob/main/test/yay/integer-big-basic.yay)
```yay
42
```

[integer-big-basic.java](https://github.com/kriskowal/yay/blob/main/test/java/integer-big-basic.java)
```java
BigInteger.valueOf(42)
```

A negative integer.

[integer-big-negative.yay](https://github.com/kriskowal/yay/blob/main/test/yay/integer-big-negative.yay)
```yay
-42
```

[integer-big-negative.java](https://github.com/kriskowal/yay/blob/main/test/java/integer-big-negative.java)
```java
BigInteger.valueOf(-42)
```

Spaces group digits for readability without changing the value.

[integer-big.yay](https://github.com/kriskowal/yay/blob/main/test/yay/integer-big.yay)
```yay
867 5309
```

[integer-big.java](https://github.com/kriskowal/yay/blob/main/test/java/integer-big.java)
```java
BigInteger.valueOf(8675309)
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

[number-float.java](https://github.com/kriskowal/yay/blob/main/test/java/number-float.java)
```java
6.283185307179586
```

A float with a leading decimal point (no integer part).

[number-float-leading-dot.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-leading-dot.yay)
```yay
.5
```

[number-float-leading-dot.java](https://github.com/kriskowal/yay/blob/main/test/java/number-float-leading-dot.java)
```java
0.5
```

A float with a trailing decimal point (no fractional part).

[number-float-trailing-dot.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-trailing-dot.yay)
```yay
1.
```

[number-float-trailing-dot.java](https://github.com/kriskowal/yay/blob/main/test/java/number-float-trailing-dot.java)
```java
1.0
```

Negative zero is distinct from positive zero.

[number-float-negative-zero.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-negative-zero.yay)
```yay
-0.0
```

[number-float-negative-zero.java](https://github.com/kriskowal/yay/blob/main/test/java/number-float-negative-zero.java)
```java
-0.0
```

Positive infinity.

[number-float-infinity.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-infinity.yay)
```yay
infinity
```

[number-float-infinity.java](https://github.com/kriskowal/yay/blob/main/test/java/number-float-infinity.java)
```java
Double.POSITIVE_INFINITY
```

Negative infinity.

[number-float-negative-infinity.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-negative-infinity.yay)
```yay
-infinity
```

[number-float-negative-infinity.java](https://github.com/kriskowal/yay/blob/main/test/java/number-float-negative-infinity.java)
```java
Double.NEGATIVE_INFINITY
```

Not-a-number (canonical NaN).

[number-float-nan.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-nan.yay)
```yay
nan
```

[number-float-nan.java](https://github.com/kriskowal/yay/blob/main/test/java/number-float-nan.java)
```java
Double.NaN
```

Spaces group digits for readability in floats.

[number-float-grouped.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-grouped.yay)
```yay
6.283 185 307 179 586
```

[number-float-grouped.java](https://github.com/kriskowal/yay/blob/main/test/java/number-float-grouped.java)
```java
6.283185307179586
```

Scientific notation using `e` or `E` for the exponent (Avogadro's number).

[number-float-avogadro.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-avogadro.yay)
```yay
6.022e23
```

[number-float-avogadro.java](https://github.com/kriskowal/yay/blob/main/test/java/number-float-avogadro.java)
```java
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

[string-block-root-same-line.java](https://github.com/kriskowal/yay/blob/main/test/java/string-block-root-same-line.java)
```java
"I think you ought to know I'm feeling very depressed.\nThis will all end in tears.\n"
```

Backtick alone on its line produces a leading newline because content starts on the following line.

[string-block-root-next-line.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-root-next-line.yay)
```yay
`
  I've calculated your chance of survival,
  but I don't think you'll like it.
```

[string-block-root-next-line.java](https://github.com/kriskowal/yay/blob/main/test/java/string-block-root-next-line.java)
```java
"\nI've calculated your chance of survival,\nbut I don't think you'll like it.\n"
```

An empty line in the middle of a block string is preserved as a newline.

[string-block-empty-middle.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-empty-middle.yay)
```yay
`
  I'm getting better!

  No you're not.
```

[string-block-empty-middle.java](https://github.com/kriskowal/yay/blob/main/test/java/string-block-empty-middle.java)
```java
"\nI'm getting better!\n\nNo you're not.\n"
```

The `#` character inside a block string is literal content, not a comment.

[string-block-root-hash.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-root-hash.yay)
```yay
` # this is not a comment
  it is content
```

[string-block-root-hash.java](https://github.com/kriskowal/yay/blob/main/test/java/string-block-root-hash.java)
```java
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

[string-block-nested-in-object-and-array.java](https://github.com/kriskowal/yay/blob/main/test/java/string-block-nested-in-object-and-array.java)
```java
Map.of("parrot", Map.of("condition", "No, no, it's just resting!\n", "remarks", List.of("Remarkable bird, the Norwegian Blue.\nBeautiful plumage, innit?\n", "It's probably pining for the fjords.\nLovely plumage.\n")))
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

[string-block-property.java](https://github.com/kriskowal/yay/blob/main/test/java/string-block-property.java)
```java
Map.of("message", "By Grabthar's hammer, we live to tell the tale.\n")
```

An empty line in the middle of a block string property is preserved.

[string-block-property-empty-middle.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-property-empty-middle.yay)
```yay
message: `
  It's not pining!

  It's passed on! This parrot is no more!
```

[string-block-property-empty-middle.java](https://github.com/kriskowal/yay/blob/main/test/java/string-block-property-empty-middle.java)
```java
Map.of("message", "It's not pining!\n\nIt's passed on! This parrot is no more!\n")
```

A block string property followed by another property: the block ends when a line at the same or lesser indent appears.
Trailing empty lines collapse to a single trailing newline.

[string-block-property-trailing-empty.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-property-trailing-empty.yay)
```yay
message: `
  By Grabthar's hammer... what a savings.


next: 1
```

[string-block-property-trailing-empty.java](https://github.com/kriskowal/yay/blob/main/test/java/string-block-property-trailing-empty.java)
```java
Map.of("message", "By Grabthar's hammer... what a savings.\n", "next", BigInteger.valueOf(1))
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

[string-inline-doublequote-basic.java](https://github.com/kriskowal/yay/blob/main/test/java/string-inline-doublequote-basic.java)
```java
"This will all end in tears."
```

A single-quoted string (literal, no escapes).

[string-inline-singlequote-basic.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-singlequote-basic.yay)
```yay
'Are you suggesting coconuts migrate?'
```

[string-inline-singlequote-basic.java](https://github.com/kriskowal/yay/blob/main/test/java/string-inline-singlequote-basic.java)
```java
"Are you suggesting coconuts migrate?"
```

A double-quoted string with escape sequences.

[string-inline-doublequote-escapes.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-doublequote-escapes.yay)
```yay
"\"\\\/\b\f\n\r\t\u{263A}"
```

[string-inline-doublequote-escapes.java](https://github.com/kriskowal/yay/blob/main/test/java/string-inline-doublequote-escapes.java)
```java
"\"\\/\n\r\t☺"
```

A double-quoted string with a Unicode emoji (literal UTF-8).

[string-inline-doublequote-unicode-emoji.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-doublequote-unicode-emoji.yay)
```yay
"😀"
```

[string-inline-doublequote-unicode-emoji.java](https://github.com/kriskowal/yay/blob/main/test/java/string-inline-doublequote-unicode-emoji.java)
```java
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

[string-inline-doublequote-unicode-surrogate-pair.java](https://github.com/kriskowal/yay/blob/main/test/java/string-inline-doublequote-unicode-surrogate-pair.java)
```java
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

[string-multiline-concat.java](https://github.com/kriskowal/yay/blob/main/test/java/string-multiline-concat.java)
```java
Map.of("confession", "I'm not dead yet. I feel happy!")
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

[array-multiline.java](https://github.com/kriskowal/yay/blob/main/test/java/array-multiline.java)
```java
List.of(BigInteger.valueOf(5), BigInteger.valueOf(3))
```

Nested arrays where each top-level item contains an inner array.

[array-multiline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-multiline-nested.yay)
```yay
- - "a"
  - "b"
- - 1
  - 2
```

[array-multiline-nested.java](https://github.com/kriskowal/yay/blob/main/test/java/array-multiline-nested.java)
```java
List.of(
    List.of("a", "b"),
    List.of(BigInteger.valueOf(1), BigInteger.valueOf(2))
)
```

An array as the value of an object property.

[array-multiline-named.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-multiline-named.yay)
```yay
complaints:
- "I didn't vote for you."
- "Help, help, I'm being repressed!"
```

[array-multiline-named.java](https://github.com/kriskowal/yay/blob/main/test/java/array-multiline-named.java)
```java
Map.of("complaints", List.of("I didn't vote for you.", "Help, help, I'm being repressed!"))
```

## Inline Arrays

Inline arrays use JSON-style bracket syntax with strict spacing rules: no space after `[`, no space before `]`, exactly one space after each `,`.

A simple inline array with string values.

[array-inline-doublequote.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-doublequote.yay)
```yay
["And there was much rejoicing.", "yay."]
```

[array-inline-doublequote.java](https://github.com/kriskowal/yay/blob/main/test/java/array-inline-doublequote.java)
```java
List.of("And there was much rejoicing.", "yay.")
```

An inline array containing big integers.

[array-inline-integers.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-integers.yay)
```yay
[42, 404, 418]
```

[array-inline-integers.java](https://github.com/kriskowal/yay/blob/main/test/java/array-inline-integers.java)
```java
List.of(BigInteger.valueOf(42), BigInteger.valueOf(404), BigInteger.valueOf(418))
```

An inline array containing byte array literals.

[array-inline-bytearray.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-bytearray.yay)
```yay
[<b0b5>, <cafe>]
```

[array-inline-bytearray.java](https://github.com/kriskowal/yay/blob/main/test/java/array-inline-bytearray.java)
```java
List.of(
    new byte[] {(byte) 0xb0, (byte) 0xb5},
    new byte[] {(byte) 0xca, (byte) 0xfe}
)
```

Inline arrays nested within an inline array.

[array-inline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-nested.yay)
```yay
[["I feel happy!", "yay."], ["And there was much rejoicing.", "yay."]]
```

[array-inline-nested.java](https://github.com/kriskowal/yay/blob/main/test/java/array-inline-nested.java)
```java
List.of(List.of("I feel happy!", "yay."), List.of("And there was much rejoicing.", "yay."))
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

[object-multiline.java](https://github.com/kriskowal/yay/blob/main/test/java/object-multiline.java)
```java
Map.of("answer", BigInteger.valueOf(42), "error", BigInteger.valueOf(404))
```

An object nested within another object, demonstrating indentation.

[object-multiline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-multiline-nested.yay)
```yay
parrot:
  status: "pining for the fjords"
  plumage: "beautiful"
```

[object-multiline-nested.java](https://github.com/kriskowal/yay/blob/main/test/java/object-multiline-nested.java)
```java
Map.of("parrot", Map.of("plumage", "beautiful", "status", "pining for the fjords"))
```

Object keys containing spaces or special characters must be quoted.

[object-multiline-doublequote-key.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-multiline-doublequote-key.yay)
```yay
"key name": 1
```

[object-multiline-doublequote-key.java](https://github.com/kriskowal/yay/blob/main/test/java/object-multiline-doublequote-key.java)
```java
Map.of("key name", BigInteger.valueOf(1))
```

An empty object as a property value.

[object-inline-empty.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-empty.yay)
```yay
empty: {}
```

[object-inline-empty.java](https://github.com/kriskowal/yay/blob/main/test/java/object-inline-empty.java)
```java
Map.of("empty", Map.of())
```

## Inline Objects

Inline objects use JSON-style brace syntax with strict spacing rules: no space after `{`, no space before `}`, exactly one space after each `,`.

A simple inline object with integer values.

[object-inline-integers.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-integers.yay)
```yay
{answer: 42, error: 404}
```

[object-inline-integers.java](https://github.com/kriskowal/yay/blob/main/test/java/object-inline-integers.java)
```java
Map.of("answer", BigInteger.valueOf(42), "error", BigInteger.valueOf(404))
```

An inline object with string and integer values.

[object-inline-mixed.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-mixed.yay)
```yay
{name: 'Marvin', mood: 'depressed'}
```

[object-inline-mixed.java](https://github.com/kriskowal/yay/blob/main/test/java/object-inline-mixed.java)
```java
Map.of("mood", "depressed", "name", "Marvin")
```

An inline object containing both a nested object and an array.

[object-inline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-nested.yay)
```yay
{luggage: {combination: 12345}, air: ["canned", "Perri-Air"]}
```

[object-inline-nested.java](https://github.com/kriskowal/yay/blob/main/test/java/object-inline-nested.java)
```java
Map.of("air", List.of("canned", "Perri-Air"), "luggage", Map.of("combination", BigInteger.valueOf(12345)))
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

[bytearray-block-basic.java](https://github.com/kriskowal/yay/blob/main/test/java/bytearray-block-basic.java)
```java
new byte[] {(byte) 0xb0, (byte) 0xb5, (byte) 0xc0, (byte) 0xff}
```

A block byte array with a comment on the first line instead of hex.

[bytearray-block-comment-only.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-comment-only.yay)
```yay
> # header comment
  b0b5 c0ff
```

[bytearray-block-comment-only.java](https://github.com/kriskowal/yay/blob/main/test/java/bytearray-block-comment-only.java)
```java
new byte[] {(byte) 0xb0, (byte) 0xb5, (byte) 0xc0, (byte) 0xff}
```

Hex and comments on the same lines for inline documentation of byte sequences.

[bytearray-block-hex-and-comment.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-hex-and-comment.yay)
```yay
> b0b5 # first chunk
  c0ff # second chunk
```

[bytearray-block-hex-and-comment.java](https://github.com/kriskowal/yay/blob/main/test/java/bytearray-block-hex-and-comment.java)
```java
new byte[] {(byte) 0xb0, (byte) 0xb5, (byte) 0xc0, (byte) 0xff}
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

[bytearray-block-property.java](https://github.com/kriskowal/yay/blob/main/test/java/bytearray-block-property.java)
```java
Map.of(
    "data", new byte[] {
        (byte) 0xb0, (byte) 0xb5, (byte) 0xc0, (byte) 0xff, (byte) 0xee, (byte) 0xfa, (byte) 0xca, (byte) 0xde
    }
)
```

A block byte array property with a comment on the leader line.

[bytearray-block-property-comment.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-property-comment.yay)
```yay
data: > # raw bytes
  b0b5 c0ff
```

[bytearray-block-property-comment.java](https://github.com/kriskowal/yay/blob/main/test/java/bytearray-block-property-comment.java)
```java
Map.of(
    "data", new byte[] {(byte) 0xb0, (byte) 0xb5, (byte) 0xc0, (byte) 0xff}
)
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

[bytearray-inline-empty.java](https://github.com/kriskowal/yay/blob/main/test/java/bytearray-inline-empty.java)
```java
new byte[0]
```

An inline byte array with hex content.

[bytearray-inline-even.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-inline-even.yay)
```yay
<b0b5c0ffeefacade>
```

[bytearray-inline-even.java](https://github.com/kriskowal/yay/blob/main/test/java/bytearray-inline-even.java)
```java
new byte[] {
    (byte) 0xb0, (byte) 0xb5, (byte) 0xc0, (byte) 0xff, (byte) 0xee, (byte) 0xfa, (byte) 0xca, (byte) 0xde
}
```

An inline byte array as an object property value.

[bytearray-inline-named.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-inline-named.yay)
```yay
data: <b0b5c0ffeefacade>
```

[bytearray-inline-named.java](https://github.com/kriskowal/yay/blob/main/test/java/bytearray-inline-named.java)
```java
Map.of("data", new byte[] {(byte) 0xb0, (byte) 0xb5, (byte) 0xc0, (byte) 0xff, (byte) 0xee, (byte) 0xfa, (byte) 0xca, (byte) 0xde})
```

## Error Handling

Errors include line and column numbers for debugging:

```java
try {
    Object result = Yay.parse("invalid: [", "config.yay");
} catch (YayParseException e) {
    System.err.println(e.getMessage());
    // "Unexpected newline in inline array at 1:11 of <config.yay>"
    System.err.println("Line: " + e.getLine());
    System.err.println("Column: " + e.getColumn());
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
cd java
mvn test
```

The test runner uses fixture files from `../test/`.
Files with `.yay` extension contain YAY input.
Files with `.java` extension contain expected Java output.

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
