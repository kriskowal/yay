# Tagged values, option F: grammar patch and evidence

Status: proposal, 2026-11-03, for the form Kris confirmed, against binyay
2.0.0 (`733849b`). The companion of [tagged-values.md](tagged-values.md)
§ F: the patch F makes to [`GRAMMAR.md`](../../GRAMMAR.md), the check that
it changes no existing document, and the cases for every rule. It replaces
option E's companion, which is at `7df077b` on `design-tagged-values`.

## The patch

New productions:

```
simple-name = name-start { name-start | digit } ;
name-start  = "a".."z" | "A".."Z" | "_" ;

raw-tag     = raw-start [ { raw-char } raw-end ] ;
raw-start   = /* any char but SP, TAB, LF, "'", '"', ")" */ ;
raw-end     = /* any char but SP, TAB, LF, ")" */ ;
raw-char    = raw-end | SP ;
paren-tag   = "(" ( raw-tag | double-quoted | single-quoted ) ")" ;

suffix      = [ fused ] { [ SP ] paren-tag } ;  /* innermost first; SP in normal form */
fused       = simple-name ;                     /* directly after the value */
prefix      = paren-tag { SP paren-tag } ;      /* outermost first */
```

**Inline suffix.** Every inline value takes a suffix:

```
inline-value = inline-atom suffix ;
inline-atom  = /* the alternatives of inline-value in GRAMMAR.md */ ;
```

and every single-line value in block position does too: the scalar,
inline-string, inline-bytes, inline-array and inline-object alternatives of
`value⟨n⟩` and `property-value`'s `SP inline-value` are each followed by
`suffix`, before any `inline-comment`.

**Fused-name constraints**, which the grammar states as prose:

- `fused` is not allowed after `null`, `true`, `false`, `nan`, `infinity`
  or `-infinity` (those words with letters after them stay refused words).
- After `integer`, `decimal-float`, `inline-integer` or `inline-float`, the
  number is read longest first (`1e5m` is `m(100000.0)`), and `fused` may
  not begin with `e` or `E`; after the integer `0` or `-0`, it may not
  begin with `x`, `X`, `o`, `O`, `b` or `B`.
- After `suffix`, the next character is the end of the line, `,`, `]`,
  `}`, or the two spaces of an `inline-comment`. So `1 (t)s`, `'x'a b`,
  `10s-1` and `'x'com.example` are refused.

**Block prefix.**

```
property-value⟨n⟩ =/ SP prefix SP block-string-prop⟨n⟩
                   | SP prefix SP block-bytes-prop⟨n⟩
                   | SP prefix [ inline-comment ] LF nested-content⟨n⟩ ;
item-value⟨n⟩     =/ prefix SP block-string-open⟨n+2⟩
                   | prefix SP block-bytes-open⟨n+2⟩
                   | prefix [ inline-comment ] LF item-content⟨n⟩ ;
root-value        =/ prefix [ inline-comment ] LF root-object⟨0⟩
                   | prefix [ inline-comment ] LF value⟨0⟩ ;
```

`item-content⟨n⟩` is what an empty item takes today, the nested content on
the following lines deeper than the dash. A `prefix` with no block below
is refused: a tag is never a value. A prefix tag then an inline value on
the same line (`a: (rgb) [255, 136, 0]`) matches nothing and is refused.

**The root.** The root-object test skips parenthesized text as it skips
quoted text when it looks for a colon, so `(match:splitRecord)` and
`'x' (a: b)` on the first line are not root objects.

**Data model.** The table gains `| Tagged | A tag (a string) and one payload
(any value) |`.

**Normal form**, for the formatter and `--check -t yay`: a tag that is a
`simple-name` is `fused` (unless the value is a keyword, or a number and
the name begins with `e`, `E`, or after `0` with `x`, `o`, `b`); any other
is `SP paren-tag`, `raw-tag` when it is one, else quoted as the formatter
quotes a string. A fused `(` is valid and formats spaced.

## The check against the corpus

A throwaway prototype, [`tagged-values-f/desugar.py`](tagged-values-f/desugar.py),
reads F and rewrites each tagged value as the reserved-object spelling
`{"#tag": tag, payload: value}`, which the unmodified Python port then
parses. It is a line-level rewrite for this check, not a port.
[`tagged-values-f/check.py`](tagged-values-f/check.py) runs it (`python3
-I doc/design/tagged-values-f/check.py`, 2026-11-03, on 2.0.0's corpus):

| Set | Files | Result |
|---|---|---|
| `test/yay/*.yay` | 136 | every one rewrites to itself: no valid document changes meaning |
| `test/nay/*.nay` | 91 | none becomes valid |
| `tagged-values-f/cases/*.yay` | 29 | each parses to its `.json` (tags as `{"#tag", "payload"}`, bytes as `*hex`) |
| `tagged-values-f/cases/*.nay` | 31 | each refused |

No document in 2.0.0's corpus holds a `(`, valid or refused, so the
parenthesized forms touch nothing; the fused names touch only text that is
refused today (a letter after a value).

## The cases

Accepted, each with the value in its `.json`:

| Case | Shows |
|---|---|
| kris-example, kris-example-blocks | the example Kris confirmed, in normal form |
| fused-on-every-kind | a fused name on single- and double-quoted strings, bytes, arrays, objects, floats, negatives, exponents and grouped digits |
| fused-name-charset | `_`, capitals and digits in a simple name |
| fused-any-name-not-only-units | `10px`, `3xyz`: names on numbers are open (question 4) |
| paren-on-keywords | `true (t)`, `null (null)`, `nan (ms)`, `-infinity (x)` |
| paren-raw-text | dots, colons, slashes, inner spaces, `#`, two spaces and `#`, `\`, `'` and `(` raw |
| paren-quoted | `('a)b')`, `("tab\there")`, `('')`, `(' edge ')`, a tag beginning with a quote |
| paren-fused-accepted | `'X-1'(com.example/sku)`, `3('a)b')` (question 5) |
| nested-inline-innermost-first | `'…'datetime (utc)`, `[1, 2] (inner) (outer)` |
| inside-inline-containers | fused and parenthesized tags in arrays and objects, `,` and `}` inside parentheses |
| inline-then-comment | a comment after each kind of suffix |
| inline-root, inline-root-colon-in-tag | a tagged inline root, one whose tag holds `: ` |
| block-item-array, block-property-object | `- (rgb)` and `at: (point)` over their blocks |
| block-nested-outer-first | `- (outer) (inner)` |
| block-then-comment | `color: (rgb)  # …` |
| block-under-item-key | `- color: (rgb)` over items, then a sibling key |
| block-quoted-tag | `a: ('x)y')` over a block |
| block-string-property, block-string-item-same-line | `` (markdown) ` `` alone and with text after |
| block-bytes-property, block-bytes-item-same-line | `(sha256) >` alone and with hex after |
| block-concatenated-string | `msg: (note)` over two quoted lines |
| block-root-object, block-root-array, block-root-colon-in-tag | a tagged root block at column 0 (question 8) |
| misindented-sibling-unchanged | `enabled: true` then `retries: 3` at the same indent |

Refused:

| Case | Why |
|---|---|
| bare-word-tag, bare-word-over-block | a bare word is not a tag (E's form) |
| misindented-sibling | `enabled: true` over a deeper `retries: 3` stays an error |
| fused-on-keyword, fused-nan-unit | no fused name on a keyword |
| fused-beginning-with-e | `10em`: no fused `e…` after a number |
| fused-hex-prefix | `0xff`: no fused `x`, `o`, `b` after `0` |
| fused-dot, fused-colon, fused-slash, fused-dash | not a simple name; parenthesize it |
| fused-digit-first | `<ff>2x`: a simple name begins with a letter or `_` |
| fused-after-paren, two-fused | one fused name, the innermost, only |
| empty-parens, edge-space-raw, tab-in-raw | raw tags that must be quoted |
| unclosed-paren | `(t` without `)` |
| quoted-then-text | `('a'b)`: a quoted tag closes with `)` at once |
| two-spaces-before-paren | one space before `(`; two begin only a comment |
| prefix-on-inline-value, prefix-on-item-inline | inline values take only a suffix |
| prefix-without-block, prefix-at-end-of-document | a tag with no block below |
| prefix-nested-bullet-sibling | `- - (d)` over `  - e`: `- e` is a sibling |
| inline-value-over-block | `a: 10s` over a deeper line |
| property-block-string-text-after-backtick | a property's backtick stays alone on its line |
| prefix-over-one-quoted-line | one quoted line is not a concatenation (question 9) |
| root-block-indented | a tagged root block sits at column 0 (question 8) |
| bang-is-not-a-sigil | `!` begins nothing |
| tag-before-comma-without-space | `[1 (x),2]`: the comma's spacing, as today |
