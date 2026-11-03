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
             | value⟨0⟩
             | prefix [ inline-comment ] LF root-object⟨0⟩
             | prefix [ inline-comment ] LF value⟨0⟩ ;
```

The parser tries `root-object` first: if the first non-blank, non-comment
line at indent 0 contains a colon (outside quotes and outside a tag's
parentheses) and does not start with `{`, the document is a root object.
A tag alone on the first content line, `(tag)`, tags the root block below it,
which begins at column 0 on the next line.
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
         | inline-reference
         | inline-array
         | inline-object
         | block-array⟨n⟩ ;
```

Each single-line alternative (the scalars, `inline-string`, `inline-bytes`,
`inline-reference`, `inline-array` and `inline-object`) may be followed by a `suffix` of tags,
before any inline comment (see [Tagged Values](#tagged-values)).

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

**Formatted form:**

In the formatter's form, a block string is followed by exactly one blank line,
unless it is the last thing in the document, which then ends at the string's
last line.
The rule holds at every depth and whatever follows the string: a sibling
property, a key at a lesser indent, or the next list item.

```yay
notes:
  summary: `
    First paragraph.

    Second paragraph.

  author: "Ada Lovelace"
items:
- ` One.

- title: "two"
  body: `
    Two.

end: `
  The last thing in the document.
```

The blank line never changes a value.
Trailing empty lines are not content (they collapse to the single trailing
newline), so a reader cannot take the separating blank line for part of the
string, and reads a document the same with or without it.
Readers accept both forms. `yay -t yay` writes the formatted one, and
`yay --check -t yay` refuses a document that is not in it.

It follows that a block string's value always ends in exactly one newline,
after a line that is not empty.
A string whose last line is empty (`"a\n\n"`), or that does not end in a
newline (`"a\nb"`), cannot be written as a block string; the formatter writes
it as a double-quoted string with `\n` escapes.

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

## References

A reference is a URI or IRI in angle brackets: a name for something
elsewhere, held as written and never dereferenced by a reader.

```
inline-reference = "<" scheme ":" { uri-char } ">" ;

scheme           = "a".."z" { "a".."z" | digit | "+" | "-" | "." } ;

uri-char         = unreserved | gen-delim | sub-delim | pct-encoded
                 | ucschar ;

unreserved       = "A".."Z" | "a".."z" | digit | "-" | "." | "_" | "~" ;

gen-delim        = ":" | "/" | "?" | "#" | "[" | "]" | "@" ;

sub-delim        = "!" | "$" | "&" | "'" | "(" | ")"
                 | "*" | "+" | "," | ";" | "=" ;

pct-encoded      = "%" hex hex ;   /* either case */

ucschar          = /* U+00A0 and above, not a control, space or
                      separator, nor U+FFFE or U+FFFF (RFC 3987) */ ;
```

- After `<`, a letter, then letters, digits, `+`, `-` or `.`, then `:`
  make a reference; `<>` and anything else is an inline byte array. So
  `<ff8800>` is bytes, `<urn:x>` a reference, and `<./a.yay>` and `<#f>`
  are refused, since a reference needs a scheme.
- The scheme is lowercase: `<HTTPS://x>` is refused (MEH lowercases it).
- Nothing is escaped. A URI cannot hold a space, `<`, `>`, `"`, `{`, `}`,
  `|`, `\`, `^` or a backtick, so `>` always closes a reference and the
  text between the brackets is the URI, verbatim. Non-ASCII characters
  are written as they are; percent-encodings are kept as written.
- The brackets shield their text: `#` does not begin a comment inside
  them, and `,`, `]`, `}`, `(`, `)` and `'` are not syntax there.
- No space after `<`, before `>` or inside; closed on the same line.
- No block form, and never a key.

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
              | prefix SP block-string-open⟨n+2⟩
              | prefix SP block-bytes-open⟨n+2⟩
              | prefix [ inline-comment ] LF item-content⟨n⟩
              | /* empty: nested content on subsequent lines at indent > n */ ;

item-content⟨n⟩ = /* nested content on subsequent lines at indent > n,
                     as an empty item takes */ ;
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
                   | [ inline-comment ] LF nested-content⟨n⟩
                   | SP prefix SP block-string-prop⟨n⟩
                   | SP prefix SP block-bytes-prop⟨n⟩
                   | SP prefix [ inline-comment ] LF nested-content⟨n⟩ ;
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
inline-value = inline-atom suffix ;

inline-atom  = null
             | boolean
             | inline-integer
             | inline-float
             | special-float
             | double-quoted
             | single-quoted
             | inline-bytes
             | inline-reference
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

## Tagged Values

A tagged value is a tag, which is a string, and one payload, which is any
value. A tag is always written in parentheses: after an inline value, one
space and then the tag; before a block, the tag where the block is
introduced, with the block below.

```
tag         = "(" { tag-char } ")" ;
tag-char    = tag-literal | tag-escape ;
tag-literal = char - LF - TAB - ")" - "\" ;
tag-escape  = "\)" | "\\" | "\/" | "\b" | "\f"
            | "\n" | "\r" | "\t"
            | unicode-escape ;

suffix      = { tag-space tag } ;            /* innermost first */
prefix      = tag { tag-space tag } ;        /* outermost first */
tag-space   = SP ;                           /* YAY; MEH: { SP } */
```

Inside the parentheses a tag reads as a double-quoted string does, with `(`
and `)` for its quotes and `\)` in the place of `\"`. `()` is the empty
tag, and spaces are literal anywhere, at the edges included. A tab, a line
break and an unknown escape are refused; `\(` and `\"` are unknown escapes,
since `(` and `"` need none. `unicode-escape` refuses surrogates and code
points above U+10FFFF as it does in a string.

**Inline.** Every inline value takes a suffix, inside inline arrays and
objects too:

```yay
count: 10 (s)
color: <ff8800ff> (rgba)
sku: 'X-1' (com.example/sku)
odd: 3 (a\)b)
pair: [1 (x), 'y' (z)]
```

After a suffix, the next character is the end of the line, `,`, `]`, `}`,
or the two spaces of an inline comment, so `1 (t)s` and `3 (a)b)` are
refused.

**Before a block.** `key: (tag)`, `- (tag)` and, at the root, `(tag)` alone
on the first content line introduce a tagged block: an object, an array, a
concatenated string of two or more quoted lines, a block string or block
bytes. A comment may follow the tags.

```yay
color: (rgb)
  - 255
  - 136
  - 0
doc: (markdown) `
  Some *text*.
- (outer) (inner)
  name: 'x'
```

An inline value never takes a prefix (`a: (rgb) [255, 136, 0]` is
refused), a prefix with no block below is refused, and a prefix over a
single quoted line is refused, as `key:` over one quoted line is.

**Several tags.** The tag nearest the payload is innermost, both ways:
`'x' (a) (b)` is `b` around `a` around `'x'`, and `- (outer) (inner)` is
`outer` around `inner` around the block.

**Spacing.** In YAY there is exactly one space between a value and its tag
and between two tags. MEH accepts any run of spaces, or none, in those
places: `'X-1'(sku)`, `1(a)(b)`, `1   (t)`.

**Normal form.** The writer puts one space before each tag, writes a tag's
characters as they are but `)` as `\)`, `\` as `\\` and the control
characters as the double-quoted writer escapes them, and uses the block
form exactly when it would write the payload as a block in that position.

YAY gives no tag a meaning. Tag names in these examples are examples.

## Comments

```
comment = "#" { char - LF } ;
```

Comments may appear:

- At indent 0 before the root value (top-level comments)
- After a value on a property or array-item line
- After hex content in block byte lines
- After `>` on a block bytes leader line (in property context)

Comments are never recognized inside block strings, nor inside a
reference's angle brackets.

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
| Before a tag | Exactly one space (MEH: any, or none) |
| Digit grouping | Space between digits only |
| BOM (U+FEFF) | Forbidden |

## Data Model

YAY defines ten value types:

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
| Tagged | A tag (a string) and one payload (any value) |
| Reference | A URI or IRI, held as written; designates, never dereferenced |

A reference is never equal to a string, nor to bytes: two references are
equal when their text is, code point for code point, with no URI
normalization. Where a port orders values, references come after tagged
values, by text.

Object keys are always strings.
Object key order is not significant; implementations may enumerate keys in
any order.
