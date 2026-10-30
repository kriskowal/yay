# binyay

Command-line tool for the [YAY](https://github.com/kriskowal/yay) data format.

## Installation

```bash
cargo install binyay
```

## Usage

```bash
# Validate a file
yay --check config.yay

# Check that a file is already formatted: that -t yay would write it
# unchanged (with -t, --check compares bytes; without -t, only validity)
yay --check -t yay config.yay

# Convert to JSON
yay -t json config.yay

# Format and overwrite
yay -w config.yay

# Convert all .yay files in a directory to JSON
yay -t json -w ./configs/

# Convert JSON to YAY
yay -f json -t yay data.json
```

See [CLI.md](https://github.com/kriskowal/yay/blob/main/CLI.md) for full documentation.

## License

Apache 2.0

Copyright (C) 2026 Kris Kowal
