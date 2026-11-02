# binyay

YAY command-line tool for parsing, formatting, and transcoding YAY documents.

## Installation

```bash
npm install -g binyay
```

## Usage

```bash
# Parse and validate a YAY file
yay --check config.yay

# Check that a file is already formatted: that -t yay would write it
# unchanged (with -t, --check compares bytes; without -t, only validity)
yay --check -t yay config.yay

# Convert YAY to JSON
yay -t json config.yay

# Convert JSON to YAY
yay -f json -t yay data.json

# Generate Go code from YAY
yay -t go config.yay > config.go

# Read a value from stdin into a document built with SHON
echo hi | yay [ --content -s - ] -t md
```

On macOS, `npx` (npm's launcher) does not pass a shell's process
substitution `<(…)` on to any command it runs, so `npx binyay -s <(…)` fails
with `Bad file descriptor`. Pipe the content in and read it with `-s -` (or
`-b -` for bytes) instead.

## Supported Platforms

- macOS (Apple Silicon and Intel)
- Linux (x64 and ARM64)
- Windows (x64)

## Documentation

See [CLI.md](https://github.com/kriskowal/yay/blob/main/CLI.md) for full documentation.

## License

Apache 2.0

Copyright (C) 2026 Kris Kowal
