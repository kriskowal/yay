# SHON (Shell Object Notation) for YAY

SHON is a command-line notation for structured data.
It is designed to be shell-friendly: values rarely need quoting, variables
interpolate naturally, and the syntax avoids characters that shells treat
specially.

SHON for YAY extends the [original SHON](https://kriskowal.com/shon) to cover
all eight YAY value types: null, boolean, integer, float, string, bytes, array,
and object.

SHON is activated by positional arguments `[`, `-x`, `-b`, or `-s` in the CLI.
It is not a file format — there is no `-f shon` or `-t shon`.
Root scalars are not expressible as SHON; use a file or stdin for those.

## Scalars

### Null and Booleans

| YAY value | SHON |
|-----------|------|
| `null`    | `-n` |
| `true`    | `-t` |
| `false`   | `-f` |

### Numbers

Bare tokens that look like numbers are parsed according to YAY's rules:
no decimal point means big integer, decimal point or exponent means float64.
Both `e` and `E` are accepted in exponents.

| YAY value              | SHON                  |
|------------------------|-----------------------|
| `42`                   | `42`                  |
| `-7`                   | `-7`                  |
| `6.283185307179586`    | `6.283185307179586`   |
| `.5`                   | `.5`                  |
| `1.`                   | `1.`                  |
| `-0.0`                 | `-0.0`                |
| `6.022e23`             | `6.022e23`            |
| `6.022E23`             | `6.022E23`            |

### Special Floats

| YAY value    | SHON  | Mnemonic                          |
|--------------|-------|-----------------------------------|
| `infinity`   | `-I`  | Big I, big infinity               |
| `-infinity`  | `-i`  | Little i, little (negative)       |
| `nan`        | `-N`  | Big N for NaN (vs `-n` for null)  |

## Strings

Bare words that do not parse as a number or reserved flag are strings:

```
hello               → "hello"
world               → "world"
```

Single-quoted strings preserve spaces and special characters literally:

```
'hello world'       → "hello world"
```

Inside `[ ]`, the `--` escape forces the next token to be interpreted as a
string, even if it looks like a number, flag, or bracket:

```
-- 42               → "42"         (string, not integer)
-- -7               → "-7"         (string, not integer)
-- -t               → "-t"         (string, not boolean)
-- --               → "--"         (string)
-- [                → "["          (string, not array start)
```

## Bytes

The `-x` flag interprets the next token as hexadecimal bytes:

```
-x cafe             → <cafe>
-x CAFE             → <cafe>
-x CaFe             → <cafe>
-x b0b5c0ff         → <b0b5c0ff>
-x ''               → <>           (empty byte array)
```

Hex digits may be uppercase or lowercase (or mixed) and must be even in count.
The value is normalized to lowercase internally.

## File Reading

Two flags read file contents into values:

| Flag | Meaning                          | YAY type |
|------|----------------------------------|----------|
| `-b` | Read next token as file → bytes  | bytes    |
| `-s` | Read next token as file → string | string   |

```
-b image.png        → <...bytes of image.png...>
-s message.txt      → "...contents of message.txt..."
```

## Arrays

Arrays use square brackets with spaces:

```
[ 1 2 3 ]           → [1, 2, 3]
[ hello world ]     → ["hello", "world"]
[ ]                 → []
[]                  → []
```

Arrays may contain any value, including nested arrays and objects:

```
[ [ 1 2 ] [ 3 4 ] ]
→ [[1, 2], [3, 4]]
```

## Objects

Objects are arrays that contain `--key value` pairs.
Because SHON reserves only single-character flags, `--word` is unambiguously an
object key:

```
[ --name hello --count 42 ]
→ {name: "hello", count: 42}
```

An object with no keys is written `[--]`:

```
[--]                → {}
```

Nested objects and arrays:

```
[ --servers [ localhost:8080 localhost:8081 ] --options [ --verbose -t ] ]
→ {servers: ["localhost:8080", "localhost:8081"], options: {verbose: true}}
```

## Reserved Flags

All reserved flags are single characters:

| Flag | Value              |
|------|--------------------|
| `-n` | null               |
| `-t` | true               |
| `-f` | false              |
| `-I` | infinity           |
| `-i` | -infinity          |
| `-N` | NaN                |
| `-x` | next token as hex  |
| `-b` | next token as file → bytes  |
| `-s` | next token as file → string |

Everything else (inside `[ ]`):

- `--word` is always an object key.
- `--` (bare) escapes the next token as a string literal.
- `[` and `]` delimit nested arrays and objects.
- Tokens matching number patterns are integers or floats.
- All other tokens are strings.

## Grammar

```
value       = null | bool | float-special | bytes | file
            | number | escaped-string | array | object | string

null        = '-n'
bool        = '-t' | '-f'
float-special = '-I' | '-i' | '-N'
bytes       = '-x' token                                (hex is case-insensitive)
file-bytes  = '-b' token
file-string = '-s' token
escaped-string = '--' token

number      = integer | float
integer     = /^-?[0-9]+$/
float       = /^-?[0-9]*\.[0-9]*([eE][+-]?[0-9]+)?$/   (both e and E)

array       = '[' value* ']'
object      = '[' ('--' key value)+ ']' | '[--]'
key         = /^[a-zA-Z_][a-zA-Z0-9_-]*$/

string      = token  (anything not matching the above)
```

## CLI Integration

SHON is an input method, not a format.
There is no `-f shon` or `-t shon`.
Instead, a SHON expression appears directly in the command arguments as an
alternative to reading from a file or stdin.

The CLI recognizes four tokens as SHON triggers when they appear in a positional
argument slot:

| Trigger | Meaning                           |
|---------|-----------------------------------|
| `[`     | Compound value (array or object)  |
| `-x`    | Root hex byte array               |
| `-b`    | Root file → bytes                 |
| `-s`    | Root file → string                |

Everything else in a positional slot is treated as a filename (or `-` for
stdin), preserving the existing CLI behavior.

### Flag sharing: `-t` and `-f`

The CLI flags `-t` (output format) and `-f` (input format) overlap with the
SHON flags `-t` (true) and `-f` (false).
The CLI resolves this with greedy consumption:

- **`-f`** always tries to consume the next token as an input format name.
  If the next token is a recognized format, it's consumed.
  `-f` without a valid format is always an error.
  Use SHON `-f` only inside `[` brackets.
- **`-t`** always tries to consume the next token as an output format name.
  If the next token is a recognized format, it's consumed.
  If not (or at end of args), `-t` is not treated as SHON `true` either — it's
  an error.
  Use SHON `-t` only inside `[` brackets.

```bash
$ yay -t json [ --x 1.0 --y 2.0 ]    # -t json = output format, [ starts SHON
$ yay [ --verbose -t ]                 # -t inside brackets = SHON true
$ yay -f json [ 1 2 3 ]               # -f json = input format... + SHON?
                                       # ERROR: cannot have both input and SHON
$ yay -f json                          # -f json = input format, read stdin
```

### Compound values

```bash
$ yay [ --name hello --count 42 ]
{count: 42, name: "hello"}
```

```bash
$ yay [ 1 2 3 ]
[1, 2, 3]
```

```bash
$ yay [ --servers [ localhost:8080 localhost:8081 ] --options [ --verbose -t ] ]
options: {verbose: true}
servers: ["localhost:8080", "localhost:8081"]
```

### With output format

```bash
$ yay -t json [ --x 1.0 --y 2.0 ]
{
  "x": 1,
  "y": 2
}
```

```bash
$ yay -t yson [ --name hello --values [ 1 2 3 ] ]
{
  "name": "hello",
  "values": [
    "#1",
    "#2",
    "#3"
  ]
}
```

```bash
$ yay -t js [ --name hello --count 42 ]
({ "count": 42n, "name": "hello" })
```

```bash
$ yay -t yaml [ --name hello --count 42 ]
count: 42
name: hello
```

### Root byte arrays

```bash
$ yay -t yson -x cafe
"*cafe"
```

### File reading

```bash
$ yay -t yson [ --icon -b icon.png --readme -s README.md ]
{
  "icon": "*89504e...",
  "readme": "# YAY\n..."
}
```

```bash
$ yay -b icon.png -o icon.yay
```

```bash
$ yay -s message.txt
`
  ...contents of message.txt...
```

### String escaping inside brackets

The `--` escape is only meaningful inside `[ ]`:

```bash
$ yay [ -- 42 -- -t -- [ ]
["42", "-t", "["]
```

