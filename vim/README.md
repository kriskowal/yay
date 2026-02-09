# YAY Vim Plugin

Syntax highlighting, filetype detection, and editor settings for
[YAY](../README.md) documents.

## Installation

### vim-plug

```vim
Plug 'kriskowal/yay', { 'rtp': 'vim' }
```

### Vundle

```vim
Plugin 'kriskowal/yay', { 'rtp': 'vim' }
```

### Pathogen

```bash
cd ~/.vim/bundle
ln -s /path/to/yay/vim yay
```

### Manual

Copy the contents of this directory into your Vim runtime:

```bash
cp -r vim/ftdetect vim/ftplugin vim/syntax ~/.vim/
```

### Neovim (lazy.nvim)

```lua
{ "kriskowal/yay", config = function() vim.opt.rtp:append("vim") end }
```

Or symlink into your Neovim runtime:

```bash
ln -s /path/to/yay/vim ~/.config/nvim/after/
```

## What's Included

- **ftdetect/yay.vim** — Detects `.yay` and `.meh` files as the `yay` filetype.
- **syntax/yay.vim** — Syntax highlighting for all YAY constructs.
- **ftplugin/yay.vim** — Sets two-space indentation, comment format, and
  indent-based folding.

## Highlighted Elements

| Element | Highlight Group | Links To |
|---------|----------------|----------|
| `null` | `yayNull` | `Constant` |
| `true`, `false` | `yayBoolean` | `Boolean` |
| Integers | `yayInteger` | `Number` |
| Floats, `nan`, `infinity` | `yayFloat` | `Float` |
| Double-quoted strings | `yayString` | `String` |
| Single-quoted strings | `yaySingleString` | `String` |
| Escape sequences (`\n`, `\t`, etc.) | `yayEscape` | `SpecialChar` |
| Unicode escapes (`\u{...}`) | `yayUnicodeEscape` | `SpecialChar` |
| Block string delimiter (`` ` ``) | `yayBlockStringDelim` | `Delimiter` |
| Block string body | `yayBlockStringBody` | `String` |
| Inline bytes (`<hex>`) | `yayBytes` | `Special` |
| Hex content | `yayHexContent` | `Number` |
| Block bytes leader (`>`) | `yayBlockBytesLeader` | `Delimiter` |
| Object keys | `yayKey` | `Identifier` |
| Colon separator | `yayColon` | `Delimiter` |
| List marker (`-`) | `yayDash` | `Delimiter` |
| Comments (`# ...`) | `yayComment` | `Comment` |
| `TODO`, `FIXME`, etc. | `yayTodo` | `Todo` |

## Customization

Override highlight groups in your `vimrc` to change colors:

```vim
highlight yayKey ctermfg=Blue guifg=#5f87d7
highlight yayNull ctermfg=Red guifg=#d75f5f
```

## License

Apache 2.0
