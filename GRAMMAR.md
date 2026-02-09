# YAY Grammar

This document defines the formal grammar for YAY (Yet Another YAML).
The grammar is a surface grammar: productions describe the text as it
appears in a document, with indentation expressed as a parameter rather
than as an internal token stream.

## Notation

| Symbol | Meaning |
|--------|---------|
| `=` | Definition |
| `\|` | Alternation |
| `( )` | Grouping |
| `[ ]` | Optional (zero or one) |
| `{ }` | Repetition (zero or more) |
| `" "` | Literal string |
| `' '` | Literal string (alternate) |
| `..` | Character range (inclusive) |
| `- ` | Exception (set difference) |
| `/* */` | Prose annotation |
| `⟨n⟩` | Current indent level (number of leading spaces) |
| `INDENT(n)` | Exactly `n` space characters |

Many productions are parameterized by indent level `n`.
When a production refers to "deeper indent," it means any line whose leading
spaces exceed `n`.

## Source Encoding

A YAY document is a sequence of Unicode scalar values encoded as UTF-8.
The byte sequence `EF BB BF` (UTF-8 BOM) must not appear at the start.

### Allowed Characters

Only line feed (U+000A) and printable characters are permitted in the source
text.

```
char       = LF | printable ;

LF         = U+000A ;

SP         = U+0020 ;

printable  = U+0020..U+007E
           | U+00A0..U+D7FF
           | U+E000..U+FFFD - U+FDD0..U+FDEF
           | U+10000..U+10FFFF - plane-nonchars ;

plane-nonchars = /* U+xFFFE and U+xFFFF for each plane */ ;
```

Forbidden (non-exhaustive):

- U+0009 (tab) — forbidden everywhere, including inside block strings
- U+000D (carriage return)
- All C0 and C1 control characters except LF
- Surrogates (U+D800..U+DFFF)
- Non-characters (U+FDD0..U+FDEF, U+xFFFE, U+xFFFF)

## Lines

A document is a sequence of lines terminated by LF.
Every line has the form:

```
line⟨n⟩ = INDENT(n) line-body LF ;

line-body = blank
          | comment
          | content ;

blank     = /* zero characters after the indent */ ;
```

Trailing spaces after content are forbidden.
Tabs are forbidden anywhere on a line.

## Document

A document contains exactly one root value, optionally preceded by top-level
comments.
A document containing only comments (no value) is invalid.

```
document = { comment-line } root-value { blank-line } ;

comment-line = "#" { char - LF } LF ;
               /* must be at indent 0 */

blank-line   = LF ;

root-value   = root-object⟨0⟩
             | value⟨0⟩ ;
```

The parser tries `root-object` first: if the first non-blank, non-comment
line at indent 0 contains a colon (outside quotes) and does not start with
`{`, the document is a root object.
Otherwise it is parsed as a single value.
No content may follow the root value (after skipping trailing blank lines).

## Values

```
value⟨n⟩ = null
         | boolean
         | integer
         | float
         | inline-string
         | block-string⟨n⟩
         | concatenated-string⟨n⟩
         | inline-bytes
         | block-bytes⟨n⟩
         | inline-array
         | inline-object
         | block-array⟨n⟩ ;
```

Note: block objects do not appear here because they are only valid as
the value of a property or as the root document.

## Null

```
null = "null" ;
```

## Booleans

```
boolean = "true" | "false" ;
```

## Integers

Integers are arbitrary-precision.

```
integer = [ "-" ] digit { digit | SP-between-digits } ;

digit   = "0".."9" ;

SP-between-digits = /* a SP that is both preceded and followed by a digit */ ;
```

The minus must be immediately followed by a digit (no intervening space).
Digit-grouping spaces are valid only between two digits.

## Floats

Floats are IEEE 754 binary64.
A decimal point or exponent distinguishes a float from an integer.

```
float         = decimal-float | special-float ;

decimal-float = [ "-" ] mantissa [ exponent ] ;

mantissa      = int-digits "." [ frac-digits ]
              | [ int-digits ] "." frac-digits
              | int-digits /* when followed by exponent */ ;

exponent      = "e" [ "+" | "-" ] int-digits ;

int-digits    = digit { digit | SP-between-digits } ;

frac-digits   = digit { digit | SP-between-digits } ;

special-float = "nan" | "infinity" | "-infinity" ;
```

The exponent marker must be lowercase `e`.
Uppercase `E` is rejected.
`-infinity` is a single keyword.

`"."` alone and `"-."` alone are not valid floats.

## Strings

### Double-Quoted Strings

```
double-quoted = '"' { dq-char } '"' ;

dq-char       = dq-literal | dq-escape ;

dq-literal    = char - LF - '"' - '\' ;

dq-escape     = '\"' | '\\' | '\/' | '\b' | '\f'
              | '\n' | '\r' | '\t'
              | unicode-escape ;

unicode-escape = '\u{' hex-digits-1-to-6 '}' ;

hex-digits-1-to-6 = hex-digit { hex-digit }  /* 1 to 6 digits */ ;

hex-digit     = "0".."9" | "a".."f" | "A".."F" ;
```

The `\u{...}` escape accepts 1 to 6 hex digits representing a Unicode scalar
value.
Surrogates (U+D800..U+DFFF) and code points above U+10FFFF are forbidden.
The four-digit `\uXXXX` form (without braces) is not supported.

### Single-Quoted Strings

Single-quoted strings are literal.
Only `\'` and `\\` are recognized as escapes; all other backslash sequences
are literal content.

```
single-quoted = "'" { sq-char } "'" ;

sq-char       = char - LF - "'" - '\'
              | "\'" | "\\" ;
```

### Block Strings

Block strings use the backtick (`` ` ``) as an introducer.
The block string body consists of subsequent lines at deeper indent.

**At root level or as an array item:**

```
block-string-open⟨n⟩ = "`" [ SP text-to-eol ] LF
                        block-string-body⟨n⟩ ;
```

When content follows `` ` `` on the same line, it becomes the first line of
the string (no leading newline).
When the backtick is alone, the result begins with a leading newline.

**As a property value:**

```
block-string-prop⟨n⟩ = "`" LF
                        block-string-body⟨n⟩ ;
```

The backtick must be alone on the line.
There is no implicit leading newline.

**Body:**

```
block-string-body⟨n⟩ = { block-string-line⟨n⟩ } ;

block-string-line⟨n⟩ = INDENT(m) text-to-eol LF   /* where m > n */
                      | blank-line ;
```

Semantics:

- The minimum indentation of non-blank body lines is stripped; additional
  indentation is preserved.
- Empty lines within the block are preserved as newlines.
- Trailing empty lines collapse to a single trailing newline.
- No escape sequences are recognized; backslash and `#` are literal content.
- Tab characters are forbidden.

### Concatenated Strings

Multiple quoted strings on consecutive lines at the same deeper indent are
concatenated into a single string.
At least two lines are required.

```
concatenated-string⟨n⟩ = INDENT(m) quoted-string LF
                          INDENT(m) quoted-string LF
                          { INDENT(m) quoted-string LF }   /* where m > n */ ;

quoted-string = double-quoted | single-quoted ;
```

Each line must contain exactly one complete quoted string.

## Byte Arrays

### Inline Byte Arrays

```
inline-bytes = "<" [ hex-body ] ">" ;

hex-body     = hex-pair { [ SP ] hex-pair } ;

hex-pair     = hex-lc hex-lc ;

hex-lc       = "0".."9" | "a".."f" ;
```

- `<>` is an empty byte array.
- Uppercase hex digits are forbidden.
- An odd number of hex digits is forbidden.
- No space after `<` or before `>`.
- Must be closed on the same line.

### Block Byte Arrays

Block byte arrays use the `>` introducer.

**At root level or as an array item:**

```
block-bytes-open⟨n⟩ = ">" SP ( hex-line | comment ) LF
                       { block-hex-line⟨n⟩ } ;
```

Hex or a comment must appear on the same line as `>`.
A bare `>` alone at root level is invalid.

**As a property value:**

```
block-bytes-prop⟨n⟩ = ">" [ SP comment ] LF
                       block-hex-line⟨n⟩
                       { block-hex-line⟨n⟩ } ;
```

In property context, `>` must be followed only by an optional comment and
a newline.
Hex content starts on the following indented lines.

**Hex lines:**

```
block-hex-line⟨n⟩ = INDENT(m) ( hex-line [ inline-comment ]
                               | comment ) LF       /* where m > n */ ;

hex-line       = hex-body ;

inline-comment = SP SP comment ;

comment        = "#" { char - LF } ;
```

Spaces within hex content are for readability and are ignored.

## Arrays

### Inline Arrays

```
inline-array = "[" [ inline-items ] "]" ;

inline-items = inline-value { "," SP inline-value } ;
```

Spacing rules:

- No space after `[`
- No space before `]`
- Exactly one space after each `,`
- No space before `,`
- Must be closed on the same line

### Block Arrays

Block arrays are sequences of list items at the same indent level.

```
block-array⟨n⟩ = block-array-item⟨n⟩ { block-array-item⟨n⟩ } ;

block-array-item⟨n⟩ = INDENT(n) "- " item-value⟨n⟩ LF ;
```

Each item begins with `"- "` (dash + space), which is the two-character
list-item leader.

**Item values:**

```
item-value⟨n⟩ = inline-value
              | block-string-open⟨n+2⟩
              | block-bytes-open⟨n+2⟩
              | nested-bullet⟨n+2⟩
              | key-value-then-object⟨n+2⟩
              | /* empty: nested content on subsequent lines at indent > n */ ;
```

When the value is empty after `"- "`, the item's value comes from subsequent
lines at deeper indent: a nested block array, block object, block string,
or block bytes.

**Nested bullets** may appear inline:

```
nested-bullet⟨n⟩ = "- " item-value⟨n+2⟩ ;
```

This allows `- - - "value"` for arbitrarily nested single-element arrays.

**Key-value on a list item line** creates an object:

```
key-value-then-object⟨n⟩ = key ":" SP value⟨n⟩ ;
```

Additional properties at deeper indent are merged into the same object.

## Objects

### Inline Objects

```
inline-object    = "{" [ inline-entries ] "}" ;

inline-entries   = inline-entry { "," SP inline-entry } ;

inline-entry     = inline-key ":" SP inline-value ;
```

Spacing rules:

- No space after `{`
- No space before `}`
- Exactly one space after each `,`
- No space before `,`
- Exactly one space after `:`
- Must be closed on the same line

### Block Objects

Block objects are sequences of properties at the same indent level.

**As the root document:**

```
root-object⟨0⟩ = root-property⟨0⟩ { root-property⟨0⟩ } ;

root-property⟨0⟩ = key ":" property-value⟨0⟩ LF ;
```

**As nested content (inside a property or array item):**

```
nested-object⟨n⟩ = nested-property⟨n⟩ { nested-property⟨n⟩ } ;

nested-property⟨n⟩ = INDENT(n) key ":" property-value⟨n⟩ LF ;
```

**Property values:**

```
property-value⟨n⟩ = SP inline-value [ inline-comment ]
                   | SP block-string-prop⟨n⟩
                   | SP block-bytes-prop⟨n⟩
                   | SP "{}" [ inline-comment ]
                   | [ inline-comment ] LF nested-content⟨n⟩ ;
```

When the value is absent after the colon, the property's value is determined
by nested content at deeper indent on subsequent lines:

```
nested-content⟨n⟩ = block-array⟨m⟩            /* where m > n */
                   | nested-object⟨m⟩          /* where m > n */
                   | block-string-open⟨m⟩      /* where m > n */
                   | concatenated-string⟨n⟩ ;
```

A property with no value (`key:` at end of document or followed by a
property at the same or lesser indent) is invalid.

## Keys

```
key        = bare-key | double-quoted | single-quoted ;

bare-key   = key-char { key-char } ;

key-char   = "a".."z" | "A".."Z" | "0".."9" | "_" | "-" ;

inline-key = bare-key | double-quoted | single-quoted ;
```

Bare keys support letters, digits, underscores, and hyphens.
Keys containing spaces or other characters must be quoted.
No space is permitted before the colon in a property.

## Inline Values

Inline values are the subset of values permitted inside `[...]` and `{...}`:

```
inline-value = null
             | boolean
             | inline-integer
             | inline-float
             | special-float
             | double-quoted
             | single-quoted
             | inline-bytes
             | inline-array
             | inline-object ;

inline-integer = [ "-" ] digit { digit } ;
                 /* no digit-grouping spaces */

inline-float   = [ "-" ] inline-mantissa [ inline-exponent ] ;

inline-mantissa = digit { digit } "." { digit }
                | { digit } "." digit { digit }
                | digit { digit } /* when followed by inline-exponent */ ;

inline-exponent = "e" [ "+" | "-" ] digit { digit } ;
```

Digit-grouping spaces are not allowed in inline numbers because the space
would be ambiguous with the comma separator.

## Comments

```
comment = "#" { char - LF } ;
```

Comments may appear:

- At indent 0 before the root value (top-level comments)
- After a value on a property or array-item line
- After hex content in block byte lines
- After `>` on a block bytes leader line (in property context)

Comments are never recognized inside block strings.

## Whitespace Summary

| Rule | |
|------|-|
| Indentation | Spaces only, two per level |
| Tabs | Forbidden everywhere |
| Trailing spaces | Forbidden on every line |
| After `[`, `{`, `<` | No space |
| Before `]`, `}`, `>` | No space |
| Before `,` | No space |
| After `,` | Exactly one space |
| After `:` (with value) | Exactly one space |
| After `:` (no value) | Nothing before LF |
| After `-` in list marker | Always `"- "` (dash + space) |
| Digit grouping | Space between digits only |
| BOM (U+FEFF) | Forbidden |

## Data Model

YAY defines eight value types:

| Type | Description |
|------|-------------|
| Null | The `null` keyword |
| Boolean | `true` or `false` |
| Integer | Arbitrary-precision decimal integer |
| Float | IEEE 754 binary64 (including NaN, ±Infinity, -0.0) |
| String | Unicode text (UTF-8) |
| Bytes | Arbitrary byte sequence |
| Array | Ordered sequence of values |
| Object | Unordered map from string keys to values |

Object keys are always strings.
Object key order is not significant; implementations may enumerate keys in
any order.
