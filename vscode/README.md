# YAY for VS Code

Syntax highlighting for [YAY (Yet Another YAML)](https://github.com/kriskowal/yay) files.

## Features

- Syntax highlighting for `.yay` and `.meh` files
- Keywords: `null`, `true`, `false`, `nan`, `infinity`, `-infinity`
- Big integers and floats (including digit-grouping spaces)
- Double-quoted strings with escape sequences (`\"`, `\\`, `\n`, `\u{...}`, etc.)
- Single-quoted strings (literal)
- Block strings (backtick introducer)
- Inline byte arrays (`<hex>`)
- Block byte arrays (`>` introducer with hex lines)
- Inline arrays and objects with proper delimiter highlighting
- Object keys (bare, quoted)
- List item markers (`-`)
- Comments (`#`)
- Auto-closing pairs for brackets, braces, angles, and quotes
- Indent-based folding

## Install from Source

```bash
cd vscode
ln -s "$(pwd)" ~/.vscode/extensions/yay-lang
```

Then reload VS Code.

## License

Apache 2.0
