# Tagged values, option G: grammar patch and evidence

Status: decided, 2026-11-03, against binyay 2.0.0 (`733849b`). The
companion of [tagged-values.md](tagged-values.md) § 2: the patch G makes to
[`GRAMMAR.md`](../../GRAMMAR.md), the check that it changes no existing
document, and the cases for every rule. It replaces option F's companion,
`doc/design/tagged-values-f.md` at `d459f0d` on
`design-tagged-values-parens`.

## The patch

New productions:

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

`tag` is `double-quoted` with `(` and `)` for its delimiters and `\)` in
the place of `\"`: an empty tag is `()`, spaces are literal anywhere,
`\(` and `\"` are unknown escapes and refused, and `unicode-escape` refuses
surrogates and code points above U+10FFFF as it does in a string.

**Inline suffix.** Every inline value takes a suffix:

```
inline-value = inline-atom suffix ;
inline-atom  = /* the alternatives of inline-value in GRAMMAR.md */ ;
```

and every single-line value in block position does too: the scalar,
inline-string, inline-bytes, inline-array and inline-object alternatives of
`value⟨n⟩` and `property-value`'s `SP inline-value` are each followed by
`suffix`, before any `inline-comment`. After a `suffix`, the next character
is the end of the line, `,`, `]`, `}`, or the two spaces of an
`inline-comment`, so `1 (t)s` and `3 (a)b)` are refused.

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
the following lines deeper than the dash. `nested-content⟨n⟩` keeps its
rule that one quoted line is not a concatenation, so `a: (t)` over `'x'` is
refused. A `prefix` with no block below is refused: a tag is never a value.
A prefix then an inline value on the same line (`a: (rgb) [255, 136, 0]`)
matches nothing and is refused.

**The root.** The root-object test skips a `tag` (honoring `\)`) as it
skips quoted text when it looks for a colon, so `(match:splitRecord)` and
`'x' (a: b)` on the first line are not root objects.

**MEH.** `tag-space` is any run of spaces, none included, in MEH input; in
YAY input and output it is one space. Nothing else about tags differs
between MEH and YAY.

**Data model.** The table gains `| Tagged | A tag (a string) and one payload
(any value) |`.

**Normal form**, for the writer and `--check -t yay`: one space before each
tag in a suffix and between the tags of a prefix; a tag's characters
literal but `)` as `\)`, `\` as `\\`, and the control characters as the
double-quoted writer escapes them.

## The check against the corpus

A throwaway prototype, [`tagged-values-g/desugar.py`](tagged-values-g/desugar.py),
reads G and rewrites each tagged value as the reserved-object spelling
`{"#tag": tag, payload: value}`, which the unmodified Python port then
parses. `desugar(src, meh=True)` reads MEH's spacing. It is a line-level
rewrite for this check, not a port.
[`tagged-values-g/check.py`](tagged-values-g/check.py) runs it (`python3
-I doc/design/tagged-values-g/check.py`, 2026-11-03, on 2.0.0's corpus) and
exits non-zero on any failure, a refused document becoming valid included:

| Set | Files | Result |
|---|---|---|
| `test/yay/*.yay` | 136 | every one rewrites to itself, read as YAY and as MEH: no valid document changes meaning |
| `test/meh/*.meh` | 55 | every one rewrites to itself, read as MEH |
| `test/nay/*.nay` | 91 | none becomes valid, read as YAY or as MEH |
| `tagged-values-g/cases/*.yay` | 29 | each parses to its `.json` (tags as `{"#tag", "payload"}`, bytes as `*hex`), as YAY and as MEH |
| `tagged-values-g/cases/*.meh` | 3 | each parses to its `.json` as MEH and is refused as YAY |
| `tagged-values-g/cases/*.nay` | 28 | each refused as YAY |

No document in 2.0.0's corpus holds a `(`, valid or refused, so G touches
nothing in it. Every accepted case but `misindented-sibling-unchanged` is
refused by the unmodified 2.0.0 port. F's check read files with universal
newlines, which hid `unicode-invalid-c0-cr.nay`'s carriage return; this one
reads them byte for byte, and counts a refused document that becomes valid
as a failure.

## The cases

Accepted, each with the value in its `.json`:

| Case | Shows |
|---|---|
| kris-example, kris-example-blocks | the example of 2026-11-03, inline and over blocks |
| inline-on-every-kind | a tag on single- and double-quoted strings, bytes, arrays, objects, floats, negatives, exponents, grouped digits and `0` |
| paren-on-keywords | `true (t)`, `null (null)`, `nan (ms)`, `-infinity (x)` |
| tag-text | dots, colons, slashes, inner spaces, `#`, two spaces and `#`, `'`, `(`, `"`, `,`, `]` and `}` as text |
| tag-escapes | `\)`, `\\`, `\t`, `\u{1F600}`, `\/`, `\n` |
| tag-empty-and-spaces | `()`, `( edge )`, `( )` |
| tag-quotes-are-text | `('a')` is the tag `'a'`, quotes and all |
| nested-inline-innermost-first | `'…' (datetime) (utc)`, `[1, 2] (inner) (outer)` |
| inside-inline-containers | tags in arrays and objects, `,`, `}` and `\)` inside parentheses |
| inline-then-comment | a comment after a tag, and `  #` inside one |
| inline-root, inline-root-colon-in-tag | a tagged inline root, one whose tag holds `: ` |
| block-item-array, block-property-object | `- (rgb)` and `at: (point)` over their blocks |
| block-nested-outer-first | `- (outer) (inner)` |
| block-then-comment | `color: (rgb)  # …` |
| block-under-item-key | `- color: (rgb)` over items, then a sibling key |
| block-escaped-tag, block-empty-tag | `a: (x\)y)` and `a: ()` over a block |
| block-string-property, block-string-item-same-line | `` (markdown) ` `` alone and with text after |
| block-bytes-property, block-bytes-item-same-line | `(sha256) >` alone and with hex after |
| block-concatenated-string | `msg: (note)` over two quoted lines |
| block-root-object, block-root-array, block-root-colon-in-tag | a tagged root block at column 0 |
| misindented-sibling-unchanged | `enabled: true` then `retries: 3` at the same indent |

Accepted as MEH, refused as YAY:

| Case | Shows |
|---|---|
| meh-fused-paren | `'X-1'(com.example/sku)`, `3(a\)b)`, `[1(x), 2]` |
| meh-spacing | `1  (t)`, `2(a)(b)`, three spaces and a comment |
| meh-block-tags | `- (a)  (b)` over a block |

Refused:

| Case | Why |
|---|---|
| fused-name, fused-name-on-bytes, fused-name-on-string | `10s`, `<ff8800ff>rgba`, `'x'date`: no fused tags (F's form) |
| fused-after-paren | `1 (t)s` |
| bare-word-tag, bare-word-over-block | a bare word is not a tag (E's form) |
| bang-is-not-a-sigil | `!` begins nothing |
| unescaped-close-paren | `3 (a)b)`: the tag ends at the first unescaped `)` |
| unknown-escape, quote-escape-is-not-a-tag-escape | `\(` and `\"` are not tag escapes |
| tab-in-tag | a tab, as in a string |
| unclosed-paren, unclosed-paren-escaped | `(t` and `(t\)` without a closing `)` |
| fused-paren-strict, fused-paren-between-tags-strict | no space before `(` in YAY |
| two-spaces-before-paren-strict, two-spaces-between-block-tags-strict | two spaces in YAY |
| tag-before-comma-without-space | `[1 (x),2]`: the comma's spacing, as today |
| inline-value-over-block, misindented-sibling | a value over a deeper line stays an error |
| prefix-on-inline-value, prefix-on-item-inline | inline values take only a suffix |
| prefix-without-block, prefix-at-end-of-document | a tag with no block below |
| prefix-nested-bullet-sibling | `- - (d)` over `  - e`: `- e` is a sibling |
| property-block-string-text-after-backtick | a property's backtick stays alone on its line |
| prefix-over-one-quoted-line | one quoted line is not a concatenation |
| root-block-indented | a tagged root block sits at column 0 |
