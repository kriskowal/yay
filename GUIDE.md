# YAY Implementation Guide

This guide is for developers implementing YAY parsers in new languages and for
contributors working on the existing implementations.
It captures lessons learned from the existing implementations in Go, JavaScript,
Python, Rust, C, Scheme, and Java.

## Core Invariants

### Idempotence

The YAY formatter must be idempotent: formatting already-formatted YAY must
produce identical output.

```bash
# This must always be true:
yay file.yay > formatted.yay
yay formatted.yay > reformatted.yay
diff formatted.yay reformatted.yay  # No differences
```

The `test/meh/` fixtures with `.yay` extension test this.
The expected `.yay` file is both the expected output and a valid input that
should reproduce itself.

### Round-Trip Correctness

Parsing and re-encoding must preserve semantic meaning.
The data values must be identical even if formatting differs.

```bash
# These must produce equivalent data:
yay -t json input.yay > a.json
yay input.yay | yay -t json > b.json
diff a.json b.json  # No differences
```

### Strict Validation

The parser must reject invalid YAY files with clear error messages.
Invalid files are tested with `.nay` extension and expected errors in `.error`
files.

What must be rejected:
- Tabs (only spaces allowed)
- Trailing whitespace
- Odd number of hex digits in byte arrays
- Invalid Unicode escapes
- BOM markers
- Malformed structures

### Comment Preservation

Comments must be preserved through formatting.
The formatter may reflow and realign comments, but must not discard them.

Exception: comments are discarded when converting to formats that don't support
them (JSON, YSON, JS, Go, C, etc.).

## Project Structure

The YAY project is organized as follows:

```
yay/
├── README.md          # YAY format specification with examples
├── CLI.md             # Command line tool documentation
├── GUIDE.md           # This implementation and contribution guide
├── SHON.md            # SHON (Shell Object Notation) specification
├── test/              # Test fixtures (.yay, .nay, .error, language outputs)
│   └── reformat/      # Formatter test cases (.meh → .yay)
├── scripts/           # Build and test automation
├── rust/              # Rust implementation (workspace)
│   ├── libyay/        # Parser library
│   └── binyay/        # CLI binary
├── js/                # JavaScript implementation
│   ├── libyay/        # Parser library (npm: libyay)
│   └── binyay*/       # Platform-specific CLI packages
├── go/                # Go implementation
├── python/            # Python implementation
├── c/                 # C implementation
├── scm/               # Scheme (Guile) implementation
├── java/              # Java implementation
└── homebrew/          # Homebrew formula
```

## Development Workflow

### Running Tests

The `scripts/` directory contains all build and test automation.

**Run all tests:**

```bash
./scripts/test.sh
```

This runs tests for all language implementations in order: C, Go, Java,
JavaScript, Python, Rust, Scheme, CLI, README verification, and coverage tests.

**Run tests for a specific language:**

```bash
./scripts/test-rust.sh    # Rust tests (cargo test)
./scripts/test-js.sh      # JavaScript tests (node --test)
./scripts/test-go.sh      # Go tests
./scripts/test-python.sh  # Python tests
./scripts/test-c.sh       # C tests
./scripts/test-scm.sh     # Scheme tests
./scripts/test-java.sh    # Java tests
./scripts/test-cli.sh     # CLI integration tests
```

Each test script exits with code 0 (pass), 1 (fail), or 2 (skip if
dependencies are missing).

### Building the CLI

```bash
./scripts/build.sh
```

This builds the Rust CLI binary at `rust/target/release/yay`.

### Coverage Testing

```bash
./scripts/test-js-coverage.sh    # JavaScript coverage (requires c8)
./scripts/test-rust-coverage.sh  # Rust coverage (requires cargo-llvm-cov)
```

Coverage scripts enforce minimum thresholds and will fail if coverage drops
below the configured minimums.

### Checking Test Fixtures

```bash
./scripts/check-missing.sh
```

Verifies that every test basename has corresponding fixture files for all
languages and that CI workflows exist for each language.

### Synchronizing READMEs

```bash
./scripts/sync-readmes.sh      # Update READMEs from fixtures
./scripts/test-readmes.sh      # Verify READMEs match fixtures
```

The test fixtures are the source of truth. The `sync-readmes.sh` script
updates code blocks in README files to match the current fixture content.
The `test-readmes.sh` script verifies that READMEs are synchronized.

This prevents documentation from becoming stale. When you update a fixture,
run `sync-readmes.sh` to propagate changes to all READMEs.

The sync script also maintains the footer sections (References and License)
across all language READMEs, keeping them consistent with the root README.

### Shell Script Linting

```bash
./scripts/test-shellcheck.sh
```

Verifies that all shell scripts in `scripts/` pass `shfmt` (formatting) and
`shellcheck` (linting). This runs automatically as part of `test.sh`.

### CLI Integration Tests

The CLI test script (`scripts/test-cli.sh`) runs several categories of tests:

- **Idempotence**: YAY → YAY produces identical output
- **Reformat**: MEH (loose YAY) → YAY normalization
- **Round-trip**: YAY → YSON → YAY data integrity
- **Output**: YAY → JS/Go/C code generation
- **Error**: Invalid .nay files are rejected
- **Examples**: CLI.md examples work as documented

Run specific test categories:

```bash
./scripts/test-cli.sh idempotence
./scripts/test-cli.sh reformat
./scripts/test-cli.sh roundtrip
./scripts/test-cli.sh error
./scripts/test-cli.sh examples
```

### Building npm Packages

```bash
./scripts/build-npm-packages.sh
```

Cross-compiles the CLI binary for multiple platforms and populates the
`js/binyay-*` package directories.

## Philosophy

### Literate Programming

YAY parsers benefit from a literate style where the code reads as a narrative.
Use shallow functions with descriptive names that communicate the parse state.
Each function should do one thing and its name should describe what parsing
context it operates in.

Good function names communicate parse state:

- `parseBlockString` - we're inside a block string
- `parseInlineArray` - we're inside an inline `[...]` array
- `parseBlockBytes` - we're parsing `> hex` block byte array
- `extractLeader` - we're extracting the list marker from a line
- `skipBreaksAndStops` - we're advancing past whitespace tokens

Avoid generic names like `parse`, `handle`, or `process` without context.

### Strictness

YAY is a strict format. The parser should reject malformed input rather than
silently accepting it. This strictness makes YAY documents predictable and
reduces ambiguity.

**There is no reference implementation.** All implementations are subject to
the test fixtures. If an implementation accepts input that another rejects,
one of them has a bug. The `.nay` error tests are the authoritative source
for what must be rejected.

**Red flag: If your implementation uses `trim()`, `strip()`, or similar
whitespace-normalizing functions, it is likely not strict enough.**

These functions hide whitespace errors that should be reported. If you find
yourself using `trim()` and all tests pass, you should add test cases for
inputs with unexpected whitespace to verify the parser rejects them.

For example, these should all be errors:
- Trailing spaces on any line
- Tabs anywhere in the document
- Multiple spaces after `:` in `key: value`
- Spaces inside `[]` after `[` or before `]`
- Bare words (unquoted strings that aren't keywords or numbers)
- Properties with no value (`name:` at end of document)
- Unclosed angle brackets (`<` without matching `>`)

## Parser Pipeline

YAY parsing works best as a three-phase pipeline:

```
Source Text → Scanner → Outline Lexer → Value Parser → Result
```

### Phase 1: Scanner

The scanner converts raw source text into **scan lines**. Each scan line
contains:

- `line`: Content after indent and leader
- `indent`: Number of leading spaces
- `leader`: The two-character list marker (`"- "`) or empty string
- `lineNum`: Zero-based line number for error reporting

The scanner performs validation:
- No UTF-8 BOM allowed
- No tab characters
- No trailing spaces on any line
- Only allowed Unicode scalar values (see below)

#### Allowed Characters

A YAY document is a sequence of Unicode scalar values.
Only U+000A (line feed) and printable characters are permitted.
The scanner must reject any code point outside the allowed set.

| Range | Description |
|---|---|
| U+000A | Line feed (the only control character allowed) |
| U+0020 – U+007E | Printable ASCII (space through tilde) |
| U+00A0 – U+D7FF | Non-ASCII printable (Latin Extended through Hangul) |
| U+E000 – U+FFFD | Private Use Area through replacement character |
| U+10000 – U+10FFFF | Supplementary planes (excluding non-characters) |

All other code points are forbidden, including:

| Range | Description |
|---|---|
| U+0000 – U+0008 | C0 controls (NUL through BS) |
| U+0009 | Tab |
| U+000B – U+000C | Vertical tab, form feed |
| U+000D | Carriage return |
| U+000E – U+001F | Remaining C0 controls |
| U+007F | DEL |
| U+0080 – U+009F | C1 controls |
| U+D800 – U+DFFF | Surrogates (not valid scalar values) |
| U+FDD0 – U+FDEF | Non-characters (Arabic Presentation Forms-A) |
| U+FFFE – U+FFFF | Non-characters |
| U+xFFFE – U+xFFFF | Non-characters at end of each supplementary plane |

The surrogate range U+D800–U+DFFF is inherently excluded by UTF-8 encoding
(they are not valid scalar values), but implementations reading from other
encodings or operating on raw code points should check explicitly.

In pseudocode, the allowed predicate is:

```
allowed(cp) =
    cp == 0x000A
    || (0x0020 <= cp && cp <= 0x007E)
    || (0x00A0 <= cp && cp <= 0xD7FF)
    || (0xE000 <= cp && cp <= 0xFFFD
        && !(0xFDD0 <= cp && cp <= 0xFDEF))
    || (0x10000 <= cp && cp <= 0x10FFFF
        && (cp & 0xFFFF) < 0xFFFE)
```

Note that quoted strings can express characters that are forbidden in the
document source via escape sequences (e.g., `\t` for tab, `\n` for
newline, `\u{0000}` for NUL).
The scanner validates the source text, not the values that escape sequences
produce.

### Phase 2: Outline Lexer

The outline lexer converts scan lines into a **token stream**. It tracks
indentation using a stack and emits:

- `Start`: When a list item begins (leader is `"- "`)
- `Stop`: When indentation decreases (block ends)
- `Text`: Line content
- `Break`: Blank lines (coalesced - multiple blank lines become one break)

This phase transforms the flat line structure into a hierarchical token stream
that reflects the document's block structure.

### Phase 3: Value Parser

The value parser recursively interprets the token stream to construct the
result value. It dispatches based on token type and text content to parse:

- Keywords (`null`, `true`, `false`, `nan`, `infinity`, `-infinity`)
- Numbers (big integers and floats)
- Strings (inline quoted and block strings)
- Arrays (inline `[...]` and multiline `- item`)
- Objects (inline `{...}` and multiline `key: value`)
- Byte arrays (inline `<hex>` and multiline)

## Two-Character Leaders

A key insight: **all indentation and leader strings are exactly two characters**.

| String | Meaning |
|--------|---------|
| `"  "` | One level of indentation (two spaces) |
| `"- "` | List item marker (dash + space) |
| `` "` "`` | Block string with same-line content (backtick + space + text) |
| `"> "` | Block byte array (angle + space + hex or comment) |
| key `": "` | Key-value separator (colon + space) |

This uniformity simplifies parsing, for machines and humans alike.
When you see `"- "` at the start of content after indentation, it's a list item.
The space after the dash is mandatory and part of the marker, not a separator.

Block introducers are unambiguous:
- `` ` `` (backtick) introduces block strings - no other value starts with backtick
- `>` introduces block byte arrays - no other value starts with `>`
- `<...>` is inline byte arrays (closed on same line)
- `"..."` and `'...'` are inline strings (closed on same line)

This means `-infinity` is unambiguous: it cannot be confused with a list item
containing `infinity` because list items require `"- "` (dash + space), not
just `"-"`.

Similarly for other constructs that might appear to start with special
characters:
- `"- value"` is a list item with value `value`
- `"-infinity"` is the keyword for negative infinity
- `"-10"` is the integer negative ten

## Type Mapping

YAY has eight value types. Map them to your language's idioms:

| YAY Type | Go | JavaScript | Python | Rust | C | Scheme |
|----------|-----|------------|--------|------|---|--------|
| null | `nil` | `null` | `None` | `None` | `YAY_NULL` | `'null` |
| big integer | `*big.Int` | `BigInt` | `int` | `BigInt` | string digits | exact integer |
| float64 | `float64` | `number` | `float` | `f64` | `double` | inexact number |
| boolean | `bool` | `boolean` | `bool` | `bool` | `bool` | `#t` / `#f` |
| string | `string` | `string` | `str` | `String` | `char*` | string |
| array | `[]any` | `Array` | `list` | `Vec<Value>` | array struct | vector |
| object | `map[string]any` | `object` | `dict` | `HashMap` | object struct | alist |
| bytes | `[]byte` | `Uint8Array` | `bytes` | `Vec<u8>` | byte array | tagged list |

### Numbers

YAY distinguishes integers from floats by the presence of a decimal point or
exponent:
- `10` is a big integer
- `10.0` is a float
- `.5` is a float
- `1.` is a float
- `6.022e23` is a float (scientific notation)

Exponents must use lowercase `e`.
Uppercase `E` is accepted by MEH and canonicalized to lowercase, but strict
YAY rejects it.

Integers have arbitrary precision (big integers).
Floats are IEEE 754 binary64.

### Special Float Values

- `infinity` → positive infinity
- `-infinity` → negative infinity
- `nan` → canonical NaN

Note that `-infinity` is a single keyword, not a negated `infinity`.

## Error Reporting

Error messages must be identical across all implementations.
This consistency is essential for tooling, testing, and user experience.
Users should see the same helpful error regardless of which implementation they use.

Include line and column numbers in error messages.
Use one-based line and column numbers for human-readable output (even if you track zero-based internally).

Format: `"Error message at LINE:COL of <FILENAME>"`

When implementing a new parser, use the existing `.error` fixture files as the authoritative source for error message text.
If you find a case where an error message could be more helpful, update all implementations to use the improved message.

## Test Fixtures

The `test/` directory contains test fixtures:

- `.yay` files: Valid YAY input
- `.nay` files: Invalid YAY input (should error)
- `.error` files: Expected error message substring

For each language, there are additional fixtures for the corresponding
language representation of each `.yay` file with a common basename.
Each language uses helper functions in scope.

- `.js` files: Expected JavaScript output
- `.go` files: Expected Go output
- `.scm` files: Expected Guile Scheme output
- `.py` files: Expected Python output
- `.rs` files: Expected Rust output
- `.c` files: Expected C output

Run `test/check-missing.sh` to verify all fixtures have corresponding files
for each language.

When adding a new implementation:
1. Create fixture files for your language
2. Write a test runner that loads `.yay` files and compares against expected output
3. Write an error test runner that loads `.nay` files and verifies errors

### Structural Equality for Objects

YAY objects are unordered key-value maps. Different implementations may
produce keys in different orders, and this is valid. Your test runner should
compare parsed values using structural equality, not string comparison.

For example, `{a: 1, b: 2}` and `{b: 2, a: 1}` are equivalent YAY objects.
If your test compares string representations, it may fail spuriously when
key order differs.

Languages handle this differently:
- JavaScript and Python dictionaries preserve insertion order but should
  compare as equal regardless of order
- Go maps deliberately randomize enumeration order
- Scheme alists are ordered lists but should be compared as unordered maps

The Scheme implementation exports a `yay-equal?` function that performs
structural comparison—use it as a reference for implementing equality in
other languages.

### Error Tests Must All Pass

Every `.nay` file must cause a parse error.
If your implementation accepts any `.nay` file as valid, it's too lenient.
The error tests are as important as the valid input tests—they define what YAY is *not*.

**No implementation is relieved of the burden of passing all tests.**
Fixture tests must not be skipped.
If a test is skipped, the implementation is incomplete.
A skipped test is a bug waiting to cause interoperability problems.

## Common Pitfalls

### Treating `-` alone as a list marker

The list marker is `"- "` (dash + space), not `"-"`. A line containing only
`"-"` followed by a newline is not valid YAY. Similarly, `-a` (dash without
space) is invalid—it's not a compact array syntax.

### Using string trimming

If you call `trim()`, `strip()`, or equivalent, you're hiding errors. YAY is
strict about whitespace. Parse character by character and report unexpected
whitespace.

### Accepting bare words

YAY does not have bare words. Unquoted text must be a keyword (`null`, `true`,
`false`, `nan`, `infinity`, `-infinity`) or a valid number. If your parser
accepts `hello` as a string value, it's too lenient. The input `hello` should
produce an error like "Unexpected character".

### Confusing inline and multiline forms

Inline arrays `[a, b]` and objects `{a: 1}` must be complete on one line.
If you see `[` without `]` on the same line, that's an error, not a multiline
array.

### Confusing `<...>` and `> ...` byte syntax

These are two different syntaxes for byte arrays:
- `<hex>` is inline bytes—must be closed with `>` on the same line
- `> hex` is block bytes—uses `>` as a leader, content on following lines

A `<` without a matching `>` is always an error. Don't treat it as multiline.

### Forgetting the implicit newlines in block strings

Block strings (backtick-introduced) have:
- An implicit leading newline when the backtick is alone on a line (at root/array level)
- No implicit leading newline in property context
- Trailing empty lines collapse to a single trailing newline
- Empty lines within the block are preserved as newlines

Note: Tab characters are forbidden everywhere in YAY, including inside block
strings.
To include a tab in a string, use the `\t` escape in a quoted string, or use
concatenated quoted lines for multiline content that needs tabs.

### Not handling `-infinity` as a keyword

`-infinity` is a single keyword for negative infinity. Don't parse it as
a list item containing `infinity`.

### Accepting empty property values

A property must have a value. `name:` followed by end of document or another
property at the same indent level is an error, not a property with null value.
The value can be on the next line (nested object, array, block string, or
block bytes), but there must be something.

## Reference Implementations

Study these implementations in order of clarity:

- **Go** (`go/yay.go`): Clean three-phase pipeline with extensive comments
- **Rust** (`rust/libyay/src/`): Well-structured modules for each phase
- **JavaScript** (`js/libyay/yay.js`): Single-file reference implementation
- **Python** (`python/yay/`): Pythonic lexer/parser split
- **C** (`c/yay.c`): Manual memory management example
- **Scheme** (`scm/yay-parser.scm`): Functional style with structural equality
- **Java** (`java/`): Object-oriented implementation

Each implementation follows the same three-phase architecture but adapts to
language idioms.

## Debugging Parser Leniency

When your implementation passes all valid tests but fails error tests
(accepting invalid input), here's a systematic approach:

### Identify the failure pattern

Group failing error tests by category:
- Whitespace errors (tabs, trailing spaces, extra spaces)
- Syntax errors (missing colons, unclosed brackets)
- Value errors (bare words, invalid hex digits)
- Structure errors (empty values, extra content)

### Trace the parse path

For each failing test, trace which code path accepts the invalid input:
1. What tokens does the scanner/lexer produce?
2. Which parser function handles those tokens?
3. Where does it return success instead of error?

### Check for fallback cases

Look for `else` branches that silently accept input:
```
if (is_keyword(s)) return keyword_value;
else if (is_number(s)) return number_value;
else return s;  // BUG: accepts bare words as strings
```

The fix is usually to add explicit validation:
```
else error("Unexpected character");
```

### Avoid JSON fallbacks

If your implementation delegates to `JSON.parse()` or similar, be careful.
JSON has different rules than YAY:
- JSON allows bare `true`/`false`/`null` but YAY requires specific syntax
- JSON string escapes differ from YAY
- JSON doesn't have single-quoted strings, byte arrays, or big integers

Delegating to JSON can cause your parser to accept invalid YAY or reject
valid YAY. Parse YAY syntax directly.

## The CLI Tool

The Rust implementation includes a CLI tool (`binyay`) that serves as the
reference formatter and transcoder.
See [CLI.md](CLI.md) for full documentation.

Key capabilities:

- **Validation**: `yay --check file.yay`
- **Formatting**: `yay file.yay` (canonical YAY output)
- **Transcoding**: `yay -f json -t yay` (convert between formats)
- **Code generation**: `yay -t js`, `yay -t go`, etc.

The CLI supports multiple input formats:

- `yay`: Strict YAY format
- `meh`: Loose YAY (allows formatting variations)
- `json`: Standard JSON
- `yson`: JSON extended with YAY types

## MEH Format and Reformatter Tests

MEH ("meh") is a loose variant of YAY that accepts formatting variations that
strict YAY would reject.
The formatter normalizes MEH input to canonical YAY output.

The `test/reformat/` directory contains reformatter test cases:

- `.yay` files: Expected canonical output
- `.meh` files: Loose input variants (named `basename.variant.meh`)

For example:
- `comment-alignment.yay`: Expected output
- `comment-alignment.unaligned.meh`: Input with unaligned comments
- `comment-alignment.wide.meh`: Input with wide spacing

The formatter handles:

- Comment alignment and joining
- Inline-to-block conversion for long lines
- Hex byte formatting with consistent spacing
- Sentence boundary detection for comment wrapping
- Bullet point formatting with hanging indent

### Formatting Rules

#### Comment Processing

Comments in block byte arrays follow a three-phase transformation:

- **Join**: Consecutive comment-only lines (no hex data) are joined to the
  preceding comment.
  Joining stops at sentence boundaries (line ending with `.`, `!`, or `?` not
  followed by a capitalized word).
- **Align**: Comments are aligned to a consistent column within a block.
  Standalone comments inherit alignment from the preceding comment.
- **Wrap**: Long comments are wrapped at word boundaries.
  Abbreviation pairs (`Capital. Capital`) are kept together.
  Bullet points (`- `) use hanging indent on continuation.

Abbreviation detection is by pattern (word starts with capital, ends with
period, next word starts with capital), not a hardcoded list.
Examples: "Mr. Smith", "Dr. Jones", "St. Louis".

#### Hex Formatting

Hex content in byte arrays is normalized:
- Lowercase hex digits
- Single space between bytes
- Double space between 4-byte words

```
ca fe ba be  de ad be ef
00 11 22 33  44 55 66 77
```

## Style Notes

### Avoid numbered headings

In Markdown documentation, use unnumbered headings and bullet lists rather than
numbered lists.
Numbers create maintenance burden when items are added, removed, or reordered.
Let the document structure speak for itself.

### Sentence-per-line with wrapping

Every sentence begins on a fresh line, but long sentences wrap naturally.
This produces cleaner diffs when prose is edited—changing one sentence doesn't cause the entire paragraph to reflow.
Markdown renderers join these lines into flowing paragraphs, so the output looks the same.

## Adding New Test Cases

When adding new test fixtures:

- Create a `.yay` file for valid input
- Create corresponding language output files (`.js`, `.go`, `.py`, `.rs`,
  `.c`, `.scm`, `.java`)
- For invalid input, create a `.nay` file and a `.error` file with the expected
  error message substring
- Run `./scripts/check-missing.sh` to verify all files are present
- Run `./scripts/sync-readmes.sh` if the test is referenced in README.md

For reformatter tests in `test/reformat/`:

- Create the expected `.yay` output
- Create one or more `.meh` input variants named `basename.variant.meh`

## Continuous Integration

Each language has a corresponding GitHub Actions workflow in `.github/workflows/`:

- `rust.yml`, `js.yml`, `go.yml`, `python.yml`, `c.yml`, `guile.yml`, `java.yml`

Additional workflows verify project-wide consistency:

- `shell.yml`: Shell script formatting and linting
- `check-readmes.yml`: README examples match test fixtures
- `check-readme-footer.yml`: README footer sections (References, License) are consistent
- `check-missing.yml`: All test fixtures have corresponding language output files

The `check-missing.sh` script verifies that workflows exist for all languages
with test fixtures.

## Maintenance Checklist

Use this checklist to verify project invariants are maintained.
Run `./scripts/test.sh` to check most of these automatically.

### Shell Scripts

- [ ] All scripts pass `shfmt` formatting (`./scripts/test-shellcheck.sh`)
- [ ] All scripts pass `shellcheck` linting
- [ ] New scripts added to `test.sh` TARGETS if they should run in CI

### Test Fixtures

- [ ] Every `.yay` file has corresponding output files for all languages
- [ ] Every `.nay` file has a corresponding `.error` file
- [ ] Run `./scripts/check-missing.sh` to verify completeness
- [ ] New fixtures added to language test runners (some have hardcoded lists)

### README Synchronization

- [ ] README examples match test fixtures (`./scripts/test-readmes.sh`)
- [ ] All language READMEs have consistent footer (References + License)
- [ ] `js/libyay/README.md` and `rust/libyay/README.md` are included in sync
- [ ] Run `./scripts/sync-readmes.sh` after updating fixtures

### CLI Documentation

- [ ] `js/binyay/README.md` links to CLI.md
- [ ] `rust/binyay/README.md` links to CLI.md
- [ ] CLI examples in binyay READMEs are valid (`./scripts/test-cli-readmes.sh`)

### GitHub Workflows

- [ ] Each language has a workflow in `.github/workflows/`
- [ ] Workflow file names match the scripts they invoke
- [ ] `check-readmes.yml` uses `test-readmes.sh`
- [ ] `check-readme-footer.yml` uses `test-readme-footer.sh`

### Code Coverage

- [ ] Coverage thresholds in `test-*-coverage.sh` scripts reflect current minimums
- [ ] Coverage does not regress when removing unit tests (prefer fixture coverage)

### Parser Strictness

- [ ] All `.nay` error tests pass in all implementations
- [ ] No implementation is more lenient than others
- [ ] New error cases have corresponding `.nay` fixtures

### Type Mapping Tables

- [ ] Type mapping tables in READMEs are accurate for each language
- [ ] Remove superfluous columns (e.g., empty "Notes" columns)

### Style Conventions

- [ ] Markdown sentences start on fresh lines (but may wrap)
- [ ] No numbered lists (use bullets instead)
- [ ] Preferred byte patterns: `<ff>`, `<c0fe>`, `<f33df4ce>`, `<b0b5c0ffefacade>`

## CI Requirements

All PRs must pass:

- Rust tests: `cargo test` in `rust/`
- CLI tests: `scripts/test-cli.sh`
- Full test suite: `scripts/test.sh`
- Formatting: `cargo fmt --check`
- Shell linting: `shellcheck` on shell scripts
- Shell formatting: `shfmt -d` on shell scripts

## Backward Compatibility

### Output Stability

The canonical YAY output format should remain stable.
Changes to formatting behavior should be documented in release notes, tested
with updated fixtures, and backward compatible when possible.

### CLI Interface

- New options should not change default behavior.
- Deprecated options should warn before removal.
- Exit codes must remain consistent (0 = success, non-zero = error).

## Performance Considerations

- The formatter should handle large files efficiently.
- Directory traversal should not load all files into memory.
- Streaming output is preferred for large inputs.

## Release Process

- Ensure all tests pass.
- Update version numbers.
- Tag release in git (`v*` tag triggers CI).
- CI builds cross-platform binaries and creates a GitHub Release.
- CI updates the Homebrew tap (`kriskowal/homebrew-yippee`).
- Publish npm packages.
