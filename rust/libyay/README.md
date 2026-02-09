# YAY Parser for Rust

A parser for the [YAY](https://github.com/kriskowal/yay) data format, implemented in Rust.

## Features

- **Big integers**: Arbitrary precision integers using `num-bigint`
- **IEEE 754 floats**: Including `infinity`, `-infinity`, and `nan`
- **Byte arrays**: Hexadecimal literals with `<cafe>` syntax
- **Block strings**: Multi-line strings without escape sequences
- **Clean diffs**: Changes to values don't cascade to adjacent lines

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
libyay = "1"
```

## Usage

```rust
use libyay::{parse, Value};

fn main() -> libyay::Result<()> {
    // Parse a simple value
    let value = parse("42")?;
    assert_eq!(value, Value::Integer(42.into()));

    // Parse an inline array
    let value = parse("[1, 2, 3]")?;
    let arr = value.as_array().unwrap();
    assert_eq!(arr.len(), 3);

    // Parse an inline object
    let value = parse("{name: 'Alice', age: 30}")?;
    let obj = value.as_object().unwrap();
    assert_eq!(obj.get("name"), Some(&Value::String("Alice".into())));

    // Parse a multi-line document
    let doc = r#"
user:
  name: "Alice"
  scores:
    - 100
    - 95
    - 87
"#;
    let value = parse(doc)?;
    println!("{:?}", value);

    Ok(())
}
```

## API

### `parse(source: &str) -> Result<Value>`

Parses a YAY document string and returns the corresponding value.

### `parse_with_filename(source: &str, filename: Option<&str>) -> Result<Value>`

Parses a YAY document with a filename for error messages.

## Type Mapping

| YAY Type | Rust Type | Notes |
|----------|-----------|-------|
| `null` | `Value::Null` | |
| big integer | `Value::Integer(BigInt)` | Arbitrary precision |
| float64 | `Value::Float(f64)` | Including `f64::INFINITY`, `f64::NEG_INFINITY`, `f64::NAN` |
| boolean | `Value::Bool(bool)` | |
| string | `Value::String(String)` | |
| array | `Value::Array(Vec<Value>)` | |
| object | `Value::Object(HashMap<String, Value>)` | |
| bytes | `Value::Bytes(Vec<u8>)` | |

# YAY Format

## Null

The keyword `null` denotes a null value.

[null-literal.yay](https://github.com/kriskowal/yay/blob/main/test/yay/null-literal.yay)
```yay
null
```

[null-literal.rs](https://github.com/kriskowal/yay/blob/main/test/rs/null-literal.rs)
```rust
Value::Null
```

## Booleans

The literals `true` and `false` denote booleans.

A true boolean value.

[boolean-true.yay](https://github.com/kriskowal/yay/blob/main/test/yay/boolean-true.yay)
```yay
true
```

[boolean-true.rs](https://github.com/kriskowal/yay/blob/main/test/rs/boolean-true.rs)
```rust
Value::Bool(true)
```

A false boolean value.

[boolean-false.yay](https://github.com/kriskowal/yay/blob/main/test/yay/boolean-false.yay)
```yay
false
```

[boolean-false.rs](https://github.com/kriskowal/yay/blob/main/test/rs/boolean-false.rs)
```rust
Value::Bool(false)
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

[integer-big-basic.rs](https://github.com/kriskowal/yay/blob/main/test/rs/integer-big-basic.rs)
```rust
Value::Integer(42.into())
```

A negative integer.

[integer-big-negative.yay](https://github.com/kriskowal/yay/blob/main/test/yay/integer-big-negative.yay)
```yay
-42
```

[integer-big-negative.rs](https://github.com/kriskowal/yay/blob/main/test/rs/integer-big-negative.rs)
```rust
Value::Integer(-42.into())
```

Spaces group digits for readability without changing the value.

[integer-big.yay](https://github.com/kriskowal/yay/blob/main/test/yay/integer-big.yay)
```yay
867 5309
```

[integer-big.rs](https://github.com/kriskowal/yay/blob/main/test/rs/integer-big.rs)
```rust
Value::Integer(8675309.into())
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

[number-float.rs](https://github.com/kriskowal/yay/blob/main/test/rs/number-float.rs)
```rust
Value::Float(6.283185307179586)
```

A float with a leading decimal point (no integer part).

[number-float-leading-dot.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-leading-dot.yay)
```yay
.5
```

[number-float-leading-dot.rs](https://github.com/kriskowal/yay/blob/main/test/rs/number-float-leading-dot.rs)
```rust
Value::Float(0.5)
```

A float with a trailing decimal point (no fractional part).

[number-float-trailing-dot.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-trailing-dot.yay)
```yay
1.
```

[number-float-trailing-dot.rs](https://github.com/kriskowal/yay/blob/main/test/rs/number-float-trailing-dot.rs)
```rust
Value::Float(1.0)
```

Negative zero is distinct from positive zero.

[number-float-negative-zero.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-negative-zero.yay)
```yay
-0.0
```

[number-float-negative-zero.rs](https://github.com/kriskowal/yay/blob/main/test/rs/number-float-negative-zero.rs)
```rust
Value::Float(-0.0)
```

Positive infinity.

[number-float-infinity.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-infinity.yay)
```yay
infinity
```

[number-float-infinity.rs](https://github.com/kriskowal/yay/blob/main/test/rs/number-float-infinity.rs)
```rust
Value::Float(f64::INFINITY)
```

Negative infinity.

[number-float-negative-infinity.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-negative-infinity.yay)
```yay
-infinity
```

[number-float-negative-infinity.rs](https://github.com/kriskowal/yay/blob/main/test/rs/number-float-negative-infinity.rs)
```rust
Value::Float(f64::NEG_INFINITY)
```

Not-a-number (canonical NaN).

[number-float-nan.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-nan.yay)
```yay
nan
```

[number-float-nan.rs](https://github.com/kriskowal/yay/blob/main/test/rs/number-float-nan.rs)
```rust
Value::Float(f64::NAN)
```

Spaces group digits for readability in floats.

[number-float-grouped.yay](https://github.com/kriskowal/yay/blob/main/test/yay/number-float-grouped.yay)
```yay
6.283 185 307 179 586
```

[number-float-grouped.rs](https://github.com/kriskowal/yay/blob/main/test/rs/number-float-grouped.rs)
```rust
Value::Float(6.283185307179586)
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

[string-block-root-same-line.rs](https://github.com/kriskowal/yay/blob/main/test/rs/string-block-root-same-line.rs)
```rust
Value::String("I think you ought to know I'm feeling very depressed.\nThis will all end in tears.\n".into())
```

Backtick alone on its line produces a leading newline because content starts on the following line.

[string-block-root-next-line.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-root-next-line.yay)
```yay
`
  I've calculated your chance of survival,
  but I don't think you'll like it.
```

[string-block-root-next-line.rs](https://github.com/kriskowal/yay/blob/main/test/rs/string-block-root-next-line.rs)
```rust
Value::String("\nI've calculated your chance of survival,\nbut I don't think you'll like it.\n".into())
```

An empty line in the middle of a block string is preserved as a newline.

[string-block-empty-middle.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-empty-middle.yay)
```yay
`
  I'm getting better!

  No you're not.
```

[string-block-empty-middle.rs](https://github.com/kriskowal/yay/blob/main/test/rs/string-block-empty-middle.rs)
```rust
Value::String("\nI'm getting better!\n\nNo you're not.\n".into())
```

The `#` character inside a block string is literal content, not a comment.

[string-block-root-hash.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-root-hash.yay)
```yay
` # this is not a comment
  it is content
```

[string-block-root-hash.rs](https://github.com/kriskowal/yay/blob/main/test/rs/string-block-root-hash.rs)
```rust
Value::String("# this is not a comment\nit is content\n".into())
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

[string-block-nested-in-object-and-array.rs](https://github.com/kriskowal/yay/blob/main/test/rs/string-block-nested-in-object-and-array.rs)
```rust
Value::Object(HashMap::from([
    ("parrot".into(), Value::Object(HashMap::from([
        ("condition".into(), Value::String("No, no, it's just resting!\n".into())),
        ("remarks".into(), Value::Array(vec![
            Value::String("Remarkable bird, the Norwegian Blue.\nBeautiful plumage, innit?\n".into()),
            Value::String("It's probably pining for the fjords.\nLovely plumage.\n".into()),
        ])),
    ]))),
]))
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

[string-block-property.rs](https://github.com/kriskowal/yay/blob/main/test/rs/string-block-property.rs)
```rust
Value::Object(HashMap::from([
    ("message".into(), Value::String("By Grabthar's hammer, we live to tell the tale.\n".into())),
]))
```

An empty line in the middle of a block string property is preserved.

[string-block-property-empty-middle.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-property-empty-middle.yay)
```yay
message: `
  It's not pining!

  It's passed on! This parrot is no more!
```

[string-block-property-empty-middle.rs](https://github.com/kriskowal/yay/blob/main/test/rs/string-block-property-empty-middle.rs)
```rust
Value::Object(HashMap::from([
    ("message".into(), Value::String("It's not pining!\n\nIt's passed on! This parrot is no more!\n".into())),
]))
```

A block string property followed by another property: the block ends when a line at the same or lesser indent appears.
Trailing empty lines collapse to a single trailing newline.

[string-block-property-trailing-empty.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-block-property-trailing-empty.yay)
```yay
message: `
  By Grabthar's hammer... what a savings.


next: 1
```

[string-block-property-trailing-empty.rs](https://github.com/kriskowal/yay/blob/main/test/rs/string-block-property-trailing-empty.rs)
```rust
Value::Object(HashMap::from([
    ("message".into(), Value::String("By Grabthar's hammer... what a savings.\n".into())),
    ("next".into(), Value::Integer(1.into())),
]))
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

[string-inline-doublequote-basic.rs](https://github.com/kriskowal/yay/blob/main/test/rs/string-inline-doublequote-basic.rs)
```rust
Value::String("This will all end in tears.".into())
```

A single-quoted string (literal, no escapes).

[string-inline-singlequote-basic.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-singlequote-basic.yay)
```yay
'Are you suggesting coconuts migrate?'
```

[string-inline-singlequote-basic.rs](https://github.com/kriskowal/yay/blob/main/test/rs/string-inline-singlequote-basic.rs)
```rust
Value::String("Are you suggesting coconuts migrate?".into())
```

A double-quoted string with escape sequences.

[string-inline-doublequote-escapes.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-doublequote-escapes.yay)
```yay
"\"\\\/\b\f\n\r\t\u{263A}"
```

[string-inline-doublequote-escapes.rs](https://github.com/kriskowal/yay/blob/main/test/rs/string-inline-doublequote-escapes.rs)
```rust
Value::String("\"\\/\x08\x0c\n\r\t☺".into())
```

A double-quoted string with a Unicode emoji (literal UTF-8).

[string-inline-doublequote-unicode-emoji.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-doublequote-unicode-emoji.yay)
```yay
"😀"
```

[string-inline-doublequote-unicode-emoji.rs](https://github.com/kriskowal/yay/blob/main/test/rs/string-inline-doublequote-unicode-emoji.rs)
```rust
Value::String("😀".into())
```

A Unicode code point escape for a character outside the BMP (U+1F600), which requires a surrogate pair in UTF-16.
The `\u{...}` escape accepts 1 to 6 hexadecimal digits representing a Unicode code point (e.g. `\u{41}` for "A", `\u{1F600}` for "😀").
Surrogate code points (U+D800 through U+DFFF) are forbidden in `\u{...}` escapes.
Unlike JSON, the four-digit `\uXXXX` form is not supported; use `\u{XXXX}` instead.

[string-inline-doublequote-unicode-surrogate-pair.yay](https://github.com/kriskowal/yay/blob/main/test/yay/string-inline-doublequote-unicode-surrogate-pair.yay)
```yay
"\u{1F600}"
```

[string-inline-doublequote-unicode-surrogate-pair.rs](https://github.com/kriskowal/yay/blob/main/test/rs/string-inline-doublequote-unicode-surrogate-pair.rs)
```rust
Value::String("😀".into())
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

[array-multiline.rs](https://github.com/kriskowal/yay/blob/main/test/rs/array-multiline.rs)
```rust
Value::Array(vec![
    Value::Integer(5.into()),
    Value::Integer(3.into()),
])
```

Nested arrays where each top-level item contains an inner array.

[array-multiline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-multiline-nested.yay)
```yay
- - "a"
  - "b"
- - 1
  - 2
```

[array-multiline-nested.rs](https://github.com/kriskowal/yay/blob/main/test/rs/array-multiline-nested.rs)
```rust
Value::Array(vec![
    Value::Array(vec![Value::String("a".into()), Value::String("b".into())]),
    Value::Array(vec![Value::Integer(1.into()), Value::Integer(2.into())]),
])
```

An array as the value of an object property.

[array-multiline-named.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-multiline-named.yay)
```yay
complaints:
- "I didn't vote for you."
- "Help, help, I'm being repressed!"
```

[array-multiline-named.rs](https://github.com/kriskowal/yay/blob/main/test/rs/array-multiline-named.rs)
```rust
Value::Object(HashMap::from([
    ("complaints".into(), Value::Array(vec![
        Value::String("I didn't vote for you.".into()),
        Value::String("Help, help, I'm being repressed!".into()),
    ])),
]))
```

## Inline Arrays

Inline arrays use JSON-style bracket syntax with strict spacing rules: no space after `[`, no space before `]`, exactly one space after each `,`.

[array-inline-doublequote.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-doublequote.yay)
```yay
["And there was much rejoicing.", "yay."]
```

[array-inline-doublequote.rs](https://github.com/kriskowal/yay/blob/main/test/rs/array-inline-doublequote.rs)
```rust
Value::Array(vec![
    Value::String("And there was much rejoicing.".into()),
    Value::String("yay.".into()),
])
```

[array-inline-integers.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-integers.yay)
```yay
[42, 404, 418]
```

[array-inline-integers.rs](https://github.com/kriskowal/yay/blob/main/test/rs/array-inline-integers.rs)
```rust
Value::Array(vec![
    Value::Integer(42.into()),
    Value::Integer(404.into()),
    Value::Integer(418.into()),
])
```

[array-inline-bytearray.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-bytearray.yay)
```yay
[<b0b5>, <cafe>]
```

[array-inline-bytearray.rs](https://github.com/kriskowal/yay/blob/main/test/rs/array-inline-bytearray.rs)
```rust
Value::Array(vec![
    Value::Bytes(vec![0xb0, 0xb5]),
    Value::Bytes(vec![0xca, 0xfe]),
])
```

[array-inline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/array-inline-nested.yay)
```yay
[["I feel happy!", "yay."], ["And there was much rejoicing.", "yay."]]
```

[array-inline-nested.rs](https://github.com/kriskowal/yay/blob/main/test/rs/array-inline-nested.rs)
```rust
Value::Array(vec![
    Value::Array(vec![
        Value::String("I feel happy!".into()),
        Value::String("yay.".into()),
    ]),
    Value::Array(vec![
        Value::String("And there was much rejoicing.".into()),
        Value::String("yay.".into()),
    ]),
])
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

[object-multiline.rs](https://github.com/kriskowal/yay/blob/main/test/rs/object-multiline.rs)
```rust
Value::Object(HashMap::from([
    ("answer".into(), Value::Integer(42.into())),
    ("error".into(), Value::Integer(404.into())),
]))
```

Nested object.

[object-multiline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-multiline-nested.yay)
```yay
parrot:
  status: "pining for the fjords"
  plumage: "beautiful"
```

[object-multiline-nested.rs](https://github.com/kriskowal/yay/blob/main/test/rs/object-multiline-nested.rs)
```rust
Value::Object(HashMap::from([
    ("parrot".into(), Value::Object(HashMap::from([
        ("plumage".into(), Value::String("beautiful".into())),
        ("status".into(), Value::String("pining for the fjords".into())),
    ]))),
]))
```

Object keys containing spaces or special characters must be quoted.

[object-multiline-doublequote-key.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-multiline-doublequote-key.yay)
```yay
"key name": 1
```

[object-multiline-doublequote-key.rs](https://github.com/kriskowal/yay/blob/main/test/rs/object-multiline-doublequote-key.rs)
```rust
Value::Object(HashMap::from([(
    "key name".into(),
    Value::Integer(1.into()),
)]))
```

[object-inline-empty.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-empty.yay)
```yay
empty: {}
```

[object-inline-empty.rs](https://github.com/kriskowal/yay/blob/main/test/rs/object-inline-empty.rs)
```rust
Value::Object(HashMap::from([
    ("empty".into(), Value::Object(HashMap::new())),
]))
```

## Inline Objects

Inline objects use JSON-style brace syntax with strict spacing rules.

[object-inline-integers.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-integers.yay)
```yay
{answer: 42, error: 404}
```

[object-inline-integers.rs](https://github.com/kriskowal/yay/blob/main/test/rs/object-inline-integers.rs)
```rust
Value::Object(HashMap::from([
    ("answer".into(), Value::Integer(42.into())),
    ("error".into(), Value::Integer(404.into())),
]))
```

[object-inline-mixed.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-mixed.yay)
```yay
{name: 'Marvin', mood: 'depressed'}
```

[object-inline-mixed.rs](https://github.com/kriskowal/yay/blob/main/test/rs/object-inline-mixed.rs)
```rust
Value::Object(HashMap::from([
    ("mood".into(), Value::String("depressed".into())),
    ("name".into(), Value::String("Marvin".into())),
]))
```

[object-inline-nested.yay](https://github.com/kriskowal/yay/blob/main/test/yay/object-inline-nested.yay)
```yay
{luggage: {combination: 12345}, air: ["canned", "Perri-Air"]}
```

[object-inline-nested.rs](https://github.com/kriskowal/yay/blob/main/test/rs/object-inline-nested.rs)
```rust
Value::Object(HashMap::from([
    ("air".into(), Value::Array(vec![
        Value::String("canned".into()),
        Value::String("Perri-Air".into()),
    ])),
    ("luggage".into(), Value::Object(HashMap::from([
        ("combination".into(), Value::Integer(12345.into())),
    ]))),
]))
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

[bytearray-block-basic.rs](https://github.com/kriskowal/yay/blob/main/test/rs/bytearray-block-basic.rs)
```rust
Value::Bytes(vec![0xb0, 0xb5, 0xc0, 0xff])
```

[bytearray-block-comment-only.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-comment-only.yay)
```yay
> # header comment
  b0b5 c0ff
```

[bytearray-block-comment-only.rs](https://github.com/kriskowal/yay/blob/main/test/rs/bytearray-block-comment-only.rs)
```rust
Value::Bytes(vec![0xb0, 0xb5, 0xc0, 0xff])
```

[bytearray-block-hex-and-comment.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-hex-and-comment.yay)
```yay
> b0b5 # first chunk
  c0ff # second chunk
```

[bytearray-block-hex-and-comment.rs](https://github.com/kriskowal/yay/blob/main/test/rs/bytearray-block-hex-and-comment.rs)
```rust
Value::Bytes(vec![0xb0, 0xb5, 0xc0, 0xff])
```

In property context, `>` must be followed only by a comment or newline.

[bytearray-block-property.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-property.yay)
```yay
data: >
  b0b5 c0ff
  eefa cade
```

[bytearray-block-property.rs](https://github.com/kriskowal/yay/blob/main/test/rs/bytearray-block-property.rs)
```rust
Value::Object(HashMap::from([(
    "data".into(),
    Value::Bytes(vec![0xb0, 0xb5, 0xc0, 0xff, 0xee, 0xfa, 0xca, 0xde]),
)]))
```

[bytearray-block-property-comment.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-block-property-comment.yay)
```yay
data: > # raw bytes
  b0b5 c0ff
```

[bytearray-block-property-comment.rs](https://github.com/kriskowal/yay/blob/main/test/rs/bytearray-block-property-comment.rs)
```rust
Value::Object(HashMap::from([(
    "data".into(),
    Value::Bytes(vec![0xb0, 0xb5, 0xc0, 0xff]),
)]))
```

## Inline Byte Arrays

Binary data is written as hexadecimal inside angle brackets.
Hexadecimal must be lowercase.

[bytearray-inline-empty.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-inline-empty.yay)
```yay
<>
```

[bytearray-inline-empty.rs](https://github.com/kriskowal/yay/blob/main/test/rs/bytearray-inline-empty.rs)
```rust
Value::Bytes(vec![])
```

[bytearray-inline-even.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-inline-even.yay)
```yay
<b0b5c0ffeefacade>
```

[bytearray-inline-even.rs](https://github.com/kriskowal/yay/blob/main/test/rs/bytearray-inline-even.rs)
```rust
Value::Bytes(vec![0xb0, 0xb5, 0xc0, 0xff, 0xee, 0xfa, 0xca, 0xde])
```

[bytearray-inline-named.yay](https://github.com/kriskowal/yay/blob/main/test/yay/bytearray-inline-named.yay)
```yay
data: <b0b5c0ffeefacade>
```

[bytearray-inline-named.rs](https://github.com/kriskowal/yay/blob/main/test/rs/bytearray-inline-named.rs)
```rust
Value::Object(HashMap::from([
    ("data".into(), Value::Bytes(vec![0xb0, 0xb5, 0xc0, 0xff, 0xee, 0xfa, 0xca, 0xde])),
]))
```

## Error Handling

Parse errors include location information:

```rust
use libyay::parse_with_filename;

let result = parse_with_filename("{\n  invalid", Some("config.yay"));
if let Err(e) = result {
    println!("{}", e);
    // "Unexpected newline in inline object at 1:1 of <config.yay>"
}
```

## Whitespace Rules

YAY has strict whitespace rules:

- Two spaces for indentation (tabs are illegal)
- No trailing spaces on lines
- Exactly one space after `:` in key-value pairs
- Exactly one space after `,` in inline arrays/objects
- No spaces after `[` or `{`, or before `]` or `}`
- No space before `:` in keys

## Running Tests

```bash
cd rust
cargo test
```

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
