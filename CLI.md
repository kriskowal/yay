# YAY Command Line Tool

The `yay` command line tool parses, validates, formats, and converts
[YAY](README.md) files.

## Installation

### Homebrew (macOS/Linux)

```bash
brew install kriskowal/yippee/yay
```

### npm

```bash
npm install -g binyay
```

### Rust

```bash
cargo install binyay
```

### From Source

```bash
cd rust
cargo build --release
# Binary is at rust/target/release/yay
```

## Usage

```
yay [OPTIONS] [FILE|DIR]
```

When no file is specified, reads from stdin.
When a directory is specified, processes all `.yay` files recursively.

## Options

| Option | Description |
|--------|-------------|
| `-f, --from FORMAT` | Input format (default: `meh`); supported: `meh`, `yay`, `json`, `yson` |
| `-t, --to FORMAT` | Output format (default: `yay`); supported: `yay`, `json`, `yson`, `js`, `go`, `python`, `rust`, `c`, `java`, `scheme` |
| `-w, --write` | Write output to file with inferred extension |
| `-o, --output FILE` | Write output to specified file (not valid with directory input) |
| `--check` | Validate without producing output (exit 0 if valid, 1 if invalid) |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

### Input Formats

- **`meh`** (default): Accepts loose formatting (MEH - "Meh, close enough to YAY").
  This is lenient and reformats to canonical YAY.
- **`yay`**: Enforces strict YAY syntax.
  Use this to validate that files conform to the canonical format.
- **`json`**: Standard JSON input.
- **`yson`**: JSON extended with YAY features (big integers, byte arrays).

## Output Formats

### YAY (default)

Reformats the input as canonical YAY.
This is useful for:
- Normalizing whitespace and formatting
- Aligning comments
- Wrapping long lines
- Converting between inline and block notation

```bash
yay input.yay                # Format and print to stdout
yay input.yay -o output.yay  # Format and write to file
```

### JSON

Converts YAY to JSON.
Note that some YAY features don't have direct JSON equivalents:
- Comments are discarded
- `infinity`, `-infinity`, and `nan` become `null`

The following YAY features cannot be represented in JSON and will cause an error:
- Integers (YAY integers are BigInts, which JSON cannot represent)
- Byte arrays

Use YSON format (`-t yson`) instead if your data contains these types.

```bash
yay -t json strings.yay
```

### YSON

YSON is JSON extended with YAY features (big integers, byte arrays, special
float values).
YSON is itself a subset of the [Endo SmallCaps](https://endojs.org) encoding.

```bash
yay -t yson input.yay
```

### JavaScript

Generates JavaScript code that evaluates to the YAY value:
- Big integers use `BigInt` notation (`123n`)
- Byte arrays become `Uint8Array.from([...])`
- Objects are wrapped in parentheses for expression context

```bash
yay -t js input.yay
```

### Go

Generates Go code representing the value using `any` type.

```bash
yay -t go input.yay
```

### Python

Generates Python code representing the value.

```bash
yay -t python input.yay
```

### Rust

Generates Rust code representing the value.

```bash
yay -t rust input.yay
```

### C

Generates C code with appropriate type declarations.

```bash
yay -t c input.yay
```

### Java

Generates Java code representing the value.

```bash
yay -t java input.yay
```

### Scheme

Generates Scheme code representing the value.

```bash
yay -t scheme input.yay
```

## Validation Mode

Use `--check` to validate files without producing output:

```bash
yay --check input.yay        # Validate with lenient MEH parser (default)
yay --check directory/       # Validate all .yay files in directory
```

For strict validation (ensuring files conform to canonical YAY syntax):

```bash
yay --from yay --check input.yay        # Strict validation
yay --from yay --check directory/       # Strict validation of all files
```

## Formatting Behavior

The YAY formatter (default output) applies several transformations.

### Line Wrapping

Lines are wrapped at the configured width (default 80, configurable via `YAY_WRAP` environment variable).

**Inline to Block Conversion**:
Inline arrays and objects that exceed the line width are automatically converted to block form:

```yay
# Input (too long)
items: ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta"]

# Output (converted to block)
items:
- "alpha"
- "beta"
- "gamma"
- "delta"
- "epsilon"
- "zeta"
- "eta"
- "theta"
```

**Preserving User Choice**:
Short inline notation is preserved when it fits within the line width.
Block notation is always preserved (never converted to inline).

### Comment Handling

The formatter intelligently handles comments in block byte arrays.

**Comment Alignment**:
Comments are aligned to a consistent column within a block:

```yay
data: >
  ca fe ba be  # First comment
  de ad        # Second comment (aligned)
```

**Comment Joining**:
Fragmented comments (continuation lines without data) are joined:

```yay
# Input
data: >
  00 11 22 33  # This is a long
               # comment that spans lines.

# Output (joined if it fits)
data: >
  00 11 22 33  # This is a long comment that spans lines.
```

**Sentence Boundary Detection**:
Comments are NOT joined across sentence boundaries.
A line ending with `.`, `!`, or `?` marks a sentence end, UNLESS followed by a capitalized word (which indicates an abbreviation like "Mr. Smith"):

```yay
# These stay separate (period followed by lowercase = sentence end)
data: >
  00 11 22 33  # First sentence ends here.
               # the next thought continues.

# These get joined (period followed by capital = abbreviation pattern)
data: >
  00 11 22 33  # Please contact Mr.
               # Smith for details.
# Becomes:
data: >
  00 11 22 33  # Please contact Mr. Smith for details.
```

**Abbreviation Pair Detection**:
When wrapping, pairs of capitalized words separated by a period (like "Mr. Smith" or "Dr. Jones") are kept together.
This pattern-based detection avoids maintaining a fragile list of honorifics:

```yay
# Input
data: >
  00 11 22 33  # Please contact Mr. Smith for details about this matter.

# Output (Mr. Smith kept together)
data: >
  00 11 22 33  # Please contact
               # Mr. Smith for details about this matter.
```

The rule is: `Capital. Capital` indicates an abbreviation pair, not a sentence boundary.
Authors should ensure sentences end with punctuation.
If the next line must stay separate, start it with a lowercase word or use `!` or `?` as the sentence terminator.

**Bullet Points**:
Comments starting with `- ` are recognized as bullet points and wrapped with hanging indent:

```yay
# Input
data: >
  00 11 22 33  # - This is a bullet item that is quite long and needs wrapping.

# Output (hanging indent on continuation)
data: >
  00 11 22 33  # - This is a bullet item that is quite long and needs
               #   wrapping.
```

### Hex Formatting

Hex content in byte arrays is normalized with consistent spacing:
- Single space between bytes
- Double space between 4-byte words

```yay
data: >
  ca fe ba be  de ad be ef
  00 11 22 33  44 55 66 77
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `YAY_WRAP` | Line wrap width for formatting (default: 80) |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Parse error or invalid input |

## Examples

### Format a file

```bash
yay config.yay
```

### Validate all YAY files in a project (lenient)

```bash
yay --check .
```

### Strictly validate all YAY files in a project

```bash
yay --from yay --check .
```

### Convert to YSON

```bash
yay -t yson data.yay > data.yson
```

### Convert JSON to YAY

```bash
yay -f json -t yay data.json
```

### Convert all YAY files in a directory to YSON (write to files)

```bash
yay -t yson -w ./configs/
```

### Format with custom line width

```bash
YAY_WRAP=120 yay wide-file.yay
```

### Process stdin (JSON to YAY)

```bash
echo '{"a": 1, "b": 2}' | yay -f json -t yay
# Output:
# {a: 1.0, b: 2.0}
```

### Generate Go code from YAY

```bash
yay -t go config.yay > config.go
```

## Error Handling

### JSON Incompatibility

When converting to JSON, the tool will report an error if the document contains
types that cannot be represented in JSON:

```bash
echo '<cafe>' | yay -t json
# Error:
```

This produces:

```
Error: Cannot convert to JSON because the document contains byte arrays.
Hint: Try using YSON format instead (-t yson), which supports these types.
```

```bash
echo '9007199254740992' | yay -t json
# Error:
```

This produces:

```
Error: Cannot convert to JSON because the document contains big integers.
Hint: Try using YSON format instead (-t yson), which supports these types.
```

Use YSON format to preserve these types:

```bash
echo '<cafe>' | yay -t yson
# Output:
# "*cafe"
```

```bash
echo '9007199254740992' | yay -t yson
# Output:
# "#9007199254740992"
```
