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

# Convert YAY to JSON
yay -t json config.yay

# Convert JSON to YAY
yay -f json -t yay data.json

# Generate Go code from YAY
yay -t go config.yay > config.go
```

## Supported Platforms

- macOS (Apple Silicon and Intel)
- Linux (x64 and ARM64)
- Windows (x64)

## Documentation

See [CLI.md](https://github.com/kriskowal/yay/blob/main/CLI.md) for full documentation.

## License

Apache 2.0

Copyright (C) 2026 Kris Kowal
