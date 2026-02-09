"""
YAY lexer - tokenizes YAY input.
"""

import re
from dataclasses import dataclass
from typing import Iterator
from .errors import YaySyntaxError


@dataclass
class Token:
    """A token from the YAY lexer."""

    kind: str
    value: object
    line: int
    col: int

    def __repr__(self) -> str:
        return f"Token({self.kind!r}, {self.value!r}, line={self.line}, col={self.col})"


class Lexer:
    """
    Tokenizes YAY input.

    Handles:
    - Significant whitespace (2-space indents)
    - Comments (# to EOL)
    - Grouped digits (spaces in numbers)
    - All YAY value types
    """

    KEYWORDS = {"null", "true", "false", "infinity", "nan"}

    def __init__(self, source: str):
        self.source = source
        self.pos = 0
        self.line = 1
        self.col = 1
        self.indent_stack = [0]
        self.at_line_start = True
        self.pending_tokens: list[Token] = []
        self.current_line_indent = 0  # Track indent of current line

        # Validate source upfront
        self._validate_source()

    def error(self, message: str) -> YaySyntaxError:
        return YaySyntaxError(message, self.line, self.col)

    @staticmethod
    def _is_allowed_code_point(cp: int) -> bool:
        """Check whether a code point is allowed in a YAY document."""
        return (
            cp == 0x000A
            or (0x0020 <= cp <= 0x007E)
            or (0x00A0 <= cp <= 0xD7FF)
            or (0xE000 <= cp <= 0xFFFD and not (0xFDD0 <= cp <= 0xFDEF))
            or (0x10000 <= cp <= 0x10FFFF and (cp & 0xFFFF) < 0xFFFE)
        )

    def _validate_source(self) -> None:
        """Validate source for illegal characters and patterns."""
        # Check for BOM
        if len(self.source) >= 3:
            if self.source[0] == "\ufeff" or self.source[:3] == "\xef\xbb\xbf":
                raise YaySyntaxError("Illegal BOM", 1, 1)

        # Check for forbidden code points
        line = 1
        col = 1
        for ch in self.source:
            cp = ord(ch)
            if not self._is_allowed_code_point(cp):
                if cp == 0x09:
                    raise YaySyntaxError("Tab not allowed (use spaces)", line, col)
                if 0xD800 <= cp <= 0xDFFF:
                    raise YaySyntaxError("Illegal surrogate", line, col)
                raise YaySyntaxError(f"Forbidden code point U+{cp:04X}", line, col)
            if cp == 0x0A:
                line += 1
                col = 1
            else:
                col += 1

        # Check for trailing spaces line by line
        lines = self.source.split("\n")
        for i, line_str in enumerate(lines):
            line_num = i + 1
            # Check for trailing space (but not on the last empty line)
            if line_str.endswith(" ") and not (i == len(lines) - 1 and line_str == ""):
                raise YaySyntaxError(
                    "Unexpected trailing space", line_num, len(line_str)
                )

    def peek(self, offset: int = 0) -> str:
        """Peek at character at current position + offset."""
        idx = self.pos + offset
        if idx >= len(self.source):
            return ""
        return self.source[idx]

    def advance(self, count: int = 1) -> str:
        """Advance position and return consumed characters."""
        result = self.source[self.pos : self.pos + count]
        for ch in result:
            if ch == "\n":
                self.line += 1
                self.col = 1
                self.at_line_start = True
            else:
                self.col += 1
        self.pos += count
        return result

    def skip_to_eol(self) -> None:
        """Skip to end of line (for comments)."""
        while self.peek() and self.peek() != "\n":
            self.advance()

    def read_indent(self) -> int:
        """Read leading spaces and return indent level."""
        spaces = 0
        while self.peek() == " ":
            self.advance()
            spaces += 1
        if self.peek() == "\t":
            raise self.error("Tab not allowed (use spaces)")
        return spaces

    def read_string(self, quote: str) -> Token:
        """Read a quoted string (single or double quote)."""
        start_line = self.line
        start_col = self.col
        self.advance()  # consume opening quote

        if quote == '"':
            return self._read_double_quoted_string(start_line, start_col)
        else:
            return self._read_single_quoted_string(start_line, start_col)

    def _read_backtick_block_string(self) -> Token:
        """Read a backtick-introduced block string."""
        start_line = self.line
        start_col = self.col
        self.advance()  # consume '`'

        # Check what follows the backtick
        next_ch = self.peek()
        if next_ch == "\n":
            # Backtick alone on line - implicit leading newline
            self.advance()  # consume newline
            self.at_line_start = True
            return self._read_block_string_content(
                start_line, start_col, same_line=False
            )
        elif next_ch == " ":
            # Backtick followed by space - content on same line
            self.advance()  # consume space
            return self._read_block_string_content(
                start_line, start_col, same_line=True
            )
        else:
            raise self.error("Expected space or newline after '`'")

    def _read_block_string_content(
        self, start_line: int, start_col: int, same_line: bool
    ) -> Token:
        """Read block string content after the backtick introducer."""
        lines = []
        base_indent = (
            self.current_line_indent if hasattr(self, "current_line_indent") else 0
        )

        if same_line:
            # First line content is on the same line as the backtick
            first_line = []
            while self.peek() and self.peek() != "\n":
                first_line.append(self.advance())
            lines.append("".join(first_line))
            if self.peek() == "\n":
                self.advance()
                self.at_line_start = True

        # Read subsequent lines
        while self.pos < len(self.source):
            line_start = self.pos
            line_start_line = self.line
            spaces = 0
            while self.peek() == " ":
                spaces += 1
                self.advance()

            # If dedented to base level or less, block ends
            if spaces <= base_indent and self.peek() not in ("\n", ""):
                self.pos = line_start
                self.line = line_start_line
                self.col = 1
                self.at_line_start = True
                break

            # Empty line
            if self.peek() == "\n":
                lines.append("")
                self.advance()
                self.at_line_start = True
                continue

            # EOF
            if self.peek() == "":
                break

            # Strip base indent + 2 spaces
            content_indent = spaces - base_indent
            if content_indent >= 2:
                extra_spaces = " " * (content_indent - 2)
            else:
                extra_spaces = ""

            # Read line content
            line_chars = [extra_spaces]
            while self.peek() and self.peek() != "\n":
                line_chars.append(self.advance())
            lines.append("".join(line_chars))

            if self.peek() == "\n":
                self.advance()
                self.at_line_start = True

        # Build result - trim trailing empty lines
        while lines and lines[-1] == "":
            lines.pop()

        result = "\n".join(lines)
        if same_line:
            result = result + "\n"
        else:
            if result:
                result = "\n" + result + "\n"
            else:
                # Empty block string is not allowed
                raise self.error(
                    'Empty block string not allowed (use "" or "\\n" explicitly)'
                )

        return Token("STRING", result, start_line, start_col)

    def _read_block_bytes(self) -> Token:
        """Read a block byte array starting with >."""
        start_line = self.line
        start_col = self.col
        self.advance()  # consume '>'

        # Check what follows
        next_ch = self.peek()
        has_content = False
        if next_ch == " ":
            self.advance()  # consume space
            # Check if rest of line is just a comment or empty
            if self.peek() == "#":
                has_content = True  # comment counts as content
                self.skip_to_eol()
                next_ch = self.peek()

        # > alone or > # comment - content on subsequent lines
        if next_ch == "\n" or next_ch == "":
            if not has_content and start_col == 1:
                # > alone at the start of a line (root level) is invalid
                # In property context (start_col > 1), it's valid
                raise self.error("Expected hex or comment in hex block")
            if next_ch == "\n":
                self.advance()
                self.at_line_start = True

        base_indent = (
            self.current_line_indent if hasattr(self, "current_line_indent") else 0
        )
        hex_chars = []

        # Read first line (after > or > )
        while self.peek() and self.peek() not in "\n":
            ch = self.peek()
            if ch == "#":
                self.skip_to_eol()
                break
            if ch == " ":
                self.advance()
                continue
            if ch in "0123456789abcdef":
                hex_chars.append(self.advance())
            elif ch in "ABCDEF":
                raise YaySyntaxError(
                    "Uppercase hex digit (use lowercase)", self.line, self.col
                )
            else:
                raise self.error(f"Invalid character in byte array: {ch!r}")

        if self.peek() == "\n":
            self.advance()
            self.at_line_start = True

        # Read continuation lines
        while self.pos < len(self.source):
            line_start = self.pos
            line_start_line = self.line
            spaces = 0
            while self.peek() == " ":
                spaces += 1
                self.advance()

            # If dedented, block ends
            if spaces <= base_indent and self.peek() not in ("\n", ""):
                self.pos = line_start
                self.line = line_start_line
                self.col = 1
                self.at_line_start = True
                break

            # Empty line ends block
            if self.peek() == "\n":
                self.pos = line_start
                self.line = line_start_line
                self.col = 1
                self.at_line_start = True
                break

            # EOF
            if self.peek() == "":
                break

            # Read hex content
            while self.peek() and self.peek() not in "\n":
                ch = self.peek()
                if ch == "#":
                    self.skip_to_eol()
                    break
                if ch == " ":
                    self.advance()
                    continue
                if ch in "0123456789abcdef":
                    hex_chars.append(self.advance())
                elif ch in "ABCDEF":
                    raise YaySyntaxError(
                        "Uppercase hex digit (use lowercase)", self.line, self.col
                    )
                else:
                    raise self.error(f"Invalid character in byte array: {ch!r}")

            if self.peek() == "\n":
                self.advance()
                self.at_line_start = True

        hex_str = "".join(hex_chars)
        if len(hex_str) % 2 != 0:
            raise self.error("Odd number of hex digits in byte literal")

        try:
            value = bytes.fromhex(hex_str)
        except ValueError as e:
            raise self.error(f"Invalid hex: {e}")

        return Token("BYTES", value, start_line, start_col)

    def _read_double_quoted_string(self, start_line: int, start_col: int) -> Token:
        """Read a JSON-style double-quoted string with escape sequences."""
        chars = []
        while True:
            ch = self.peek()
            if ch == "":
                raise self.error("Unterminated string")
            if ch == '"':
                self.advance()
                break
            if ch == "\\":
                self.advance()
                esc = self.peek()
                if esc == "":
                    raise self.error("Unterminated escape sequence")
                if esc == "n":
                    chars.append("\n")
                elif esc == "r":
                    chars.append("\r")
                elif esc == "t":
                    chars.append("\t")
                elif esc == "b":
                    chars.append("\b")
                elif esc == "f":
                    chars.append("\f")
                elif esc == "\\":
                    chars.append("\\")
                elif esc == "/":
                    chars.append("/")
                elif esc == '"':
                    chars.append('"')
                elif esc == "u":
                    self.advance()  # consume 'u'
                    # YAY uses \u{XXXXXX} syntax (variable-length with braces)
                    if self.peek() != "{":
                        raise self.error("Bad escaped character")
                    self.advance()  # consume '{'
                    hex_chars = ""
                    while self.peek() and self.peek() != "}":
                        h = self.peek()
                        if h not in "0123456789abcdefABCDEF":
                            raise self.error("Bad Unicode escape")
                        hex_chars += self.advance()
                        # Max 6 hex digits
                        if len(hex_chars) > 6:
                            raise self.error("Bad Unicode escape")
                    if self.peek() != "}":
                        raise self.error("Bad Unicode escape")
                    self.advance()  # consume '}'
                    if not hex_chars:
                        raise self.error("Bad Unicode escape")
                    codepoint = int(hex_chars, 16)
                    # Check for surrogate code points (U+D800 to U+DFFF)
                    if 0xD800 <= codepoint <= 0xDFFF:
                        raise self.error("Illegal surrogate")
                    if codepoint > 0x10FFFF:
                        raise self.error("Unicode code point out of range")
                    chars.append(chr(codepoint))
                    continue
                else:
                    raise self.error("Bad escaped character")
                self.advance()
            elif ord(ch) < 0x20:
                # Newline means unterminated string, other control chars are bad
                if ch == "\n" or ch == "\r":
                    raise self.error("Unterminated string")
                raise self.error("Bad character in string")
            else:
                chars.append(self.advance())

        return Token("STRING", "".join(chars), start_line, start_col)

    def _read_single_quoted_string(self, start_line: int, start_col: int) -> Token:
        """Read a single-quoted string (literal, no escape sequences except '')."""
        chars = []
        while True:
            ch = self.peek()
            if ch == "":
                raise self.error("Unterminated string")
            if ch == "'":
                self.advance()
                if self.peek() == "'":
                    # Escaped single quote ''
                    chars.append("'")
                    self.advance()
                else:
                    break
            elif ord(ch) < 0x20:
                # Newline means unterminated string, other control chars are bad
                if ch == "\n" or ch == "\r":
                    raise self.error("Unterminated string")
                raise self.error("Bad character in string")
            else:
                # Single-quoted strings are literal - no backslash escapes
                chars.append(self.advance())

        return Token("STRING", "".join(chars), start_line, start_col)

    def read_number(self) -> Token:
        """Read a number (big integer or float)."""
        start_line = self.line
        start_col = self.col

        # Collect all characters that could be part of a number
        # Numbers can have spaces for grouping
        chars = []
        has_dot = False
        has_exponent = False
        last_was_space = False
        space_col = 0

        # Handle leading minus
        if self.peek() == "-":
            chars.append(self.advance())

        # Read digits, dots, exponents, and grouping spaces
        while True:
            ch = self.peek()
            if ch.isdigit():
                chars.append(self.advance())
                last_was_space = False
            elif ch == ".":
                if has_dot or has_exponent:
                    break  # Second dot or dot after exponent ends the number
                # Check for space before dot
                if last_was_space:
                    raise YaySyntaxError(
                        "Unexpected space in number", self.line, space_col
                    )
                has_dot = True
                chars.append(self.advance())
                # Check for space after dot
                if self.peek() == " ":
                    raise YaySyntaxError(
                        "Unexpected space in number", self.line, self.col
                    )
            elif ch == "e" and not has_exponent and len(chars) > 0:
                # Exponent notation (lowercase only)
                has_exponent = True
                chars.append(self.advance())
                # Allow optional +/- after exponent
                if self.peek() in "+-":
                    chars.append(self.advance())
            elif ch == "E" and not has_exponent and len(chars) > 0:
                # Uppercase E is not allowed
                raise YaySyntaxError(
                    "Uppercase exponent (use lowercase 'e')", self.line, self.col
                )
            elif ch == " ":
                # Space for digit grouping - peek ahead to see if more digits follow
                next_ch = self.peek(1)
                if next_ch.isdigit():
                    space_col = self.col
                    self.advance()  # consume space but don't add to chars
                    last_was_space = True
                elif next_ch == ".":
                    # Space before dot is invalid
                    raise YaySyntaxError(
                        "Unexpected space in number", self.line, self.col
                    )
                else:
                    break
            else:
                break

        num_str = "".join(chars)

        if has_dot or has_exponent:
            try:
                value = float(num_str)
            except ValueError:
                raise self.error(f"Invalid float: {num_str}")
            return Token("FLOAT", value, start_line, start_col)
        else:
            try:
                value = int(num_str)
            except ValueError:
                raise self.error(f"Invalid integer: {num_str}")
            return Token("INT", value, start_line, start_col)

    def read_bytes(
        self, already_consumed_open: bool = False, allow_multiline: bool = False
    ) -> Token:
        """Read a byte array <hex> or block byte array.

        Args:
            already_consumed_open: If True, the opening '<' has already been consumed
            allow_multiline: If True, allow newlines (for > block bytes syntax)
        """
        start_line = self.line
        start_col = self.col

        if not already_consumed_open:
            self.advance()  # consume '<'

        hex_chars = []
        last_was_space = False
        space_col = 0
        while True:
            ch = self.peek()
            if ch == "":
                # End of file - only valid in multiline mode
                if allow_multiline:
                    break
                raise self.error("Unterminated byte array")
            if ch == ">":
                # Check for space before >
                if last_was_space:
                    raise YaySyntaxError(
                        'Unexpected space before ">"', self.line, space_col
                    )
                self.advance()
                break
            if ch == " ":
                space_col = self.col
                last_was_space = True
                self.advance()
                continue
            last_was_space = False
            if ch == "\n":
                if allow_multiline:
                    # In multiline mode, check if next line continues the block
                    self.advance()
                    self.at_line_start = True
                    # Peek at indent of next line
                    spaces = 0
                    pos = self.pos
                    while pos < len(self.source) and self.source[pos] == " ":
                        spaces += 1
                        pos += 1
                    # If dedented or at a non-hex char, end the block
                    if spaces < 2:
                        break
                    # Skip the indent
                    for _ in range(spaces):
                        self.advance()
                    self.at_line_start = False
                    continue
                else:
                    # Newline not allowed in inline byte arrays
                    raise YaySyntaxError(
                        "Unmatched angle bracket", start_line, start_col
                    )
            if ch == "#":
                self.skip_to_eol()
                continue
            if ch in "0123456789abcdef":
                hex_chars.append(self.advance())
            elif ch in "ABCDEF":
                raise YaySyntaxError(
                    "Uppercase hex digit (use lowercase)", self.line, self.col
                )
            else:
                if block_mode:
                    # Unknown char ends block mode
                    break
                raise self.error(f"Invalid character in byte array: {ch!r}")

        hex_str = "".join(hex_chars)
        if len(hex_str) % 2 != 0:
            raise self.error("Odd number of hex digits in byte literal")

        try:
            value = bytes.fromhex(hex_str)
        except ValueError as e:
            raise self.error(f"Invalid hex: {e}")

        return Token("BYTES", value, start_line, start_col)

    def read_identifier(self) -> Token:
        """Read an identifier or keyword."""
        start_line = self.line
        start_col = self.col
        chars = []

        while True:
            ch = self.peek()
            if ch.isalnum() or ch == "_" or ch == "-":
                chars.append(self.advance())
            else:
                break

        name = "".join(chars)

        # Check for keywords
        if name == "null":
            return Token("NULL", None, start_line, start_col)
        elif name == "true":
            return Token("BOOL", True, start_line, start_col)
        elif name == "false":
            return Token("BOOL", False, start_line, start_col)
        elif name == "infinity":
            return Token("FLOAT", float("inf"), start_line, start_col)
        elif name == "nan":
            return Token("FLOAT", float("nan"), start_line, start_col)
        else:
            return Token("IDENT", name, start_line, start_col)

    def tokenize(self) -> Iterator[Token]:
        """Generate tokens from the source."""
        self._last_token = None

        def emit(token):
            self._last_token = token
            return token

        while self.pos < len(self.source):
            # Handle line start (indentation)
            if self.at_line_start:
                self.at_line_start = False
                indent = self.read_indent()

                # Skip blank lines and comment-only lines
                ch = self.peek()
                if ch == "\n":
                    self.advance()
                    self.at_line_start = True
                    continue
                if ch == "#":
                    self.skip_to_eol()
                    if self.peek() == "\n":
                        self.advance()
                    self.at_line_start = True
                    continue
                if ch == "":
                    break

                # Emit indent/dedent tokens
                self.current_line_indent = indent
                yield emit(Token("INDENT", indent, self.line, 1))

            ch = self.peek()

            if ch == "":
                break

            if ch == "\n":
                yield emit(Token("NEWLINE", "\n", self.line, self.col))
                self.advance()
                self.at_line_start = True
                continue

            if ch == " ":
                self.advance()
                continue

            if ch == "#":
                self.skip_to_eol()
                continue

            if ch == "\t":
                raise self.error("Tab not allowed (use spaces)")

            # String
            if ch in "\"'":
                yield emit(self.read_string(ch))
                continue

            # Block string (backtick)
            if ch == "`":
                yield emit(self._read_backtick_block_string())
                continue

            # Block byte array (>)
            if ch == ">":
                yield emit(self._read_block_bytes())
                continue

            # Bytes - inline byte array <hex>
            if ch == "<":
                start_col = self.col
                self.advance()  # consume '<'
                next_ch = self.peek()
                if next_ch == ">":
                    # Empty bytes <>
                    self.advance()
                    yield emit(Token("BYTES", b"", self.line, self.col - 2))
                    continue
                elif next_ch == "\n" or next_ch == "":
                    # Unclosed angle bracket - inline byte arrays must be closed on the same line
                    raise YaySyntaxError(
                        "Unmatched angle bracket", self.line, start_col
                    )
                elif next_ch in "ABCDEF":
                    # Uppercase hex digit - reject with specific error
                    raise YaySyntaxError(
                        "Uppercase hex digit (use lowercase)", self.line, self.col
                    )
                elif next_ch in " " or next_ch in "0123456789abcdef":
                    # Inline byte array - read until closing >
                    # already_consumed_open=True since we already consumed '<'
                    # allow_multiline=False since inline byte arrays must close on same line
                    yield emit(
                        self.read_bytes(
                            already_consumed_open=True, allow_multiline=False
                        )
                    )
                    continue
                else:
                    raise self.error(f"Invalid character after '<': {next_ch!r}")

            # Number (starts with digit, dot, or minus followed by digit/dot)
            if ch.isdigit() or ch == ".":
                yield emit(self.read_number())
                continue

            if ch == "-":
                next_ch = self.peek(1)
                if next_ch.isdigit() or next_ch == ".":
                    yield emit(self.read_number())
                    continue
                elif next_ch == "i":
                    # Could be -infinity
                    self.advance()  # consume '-'
                    tok = self.read_identifier()
                    if tok.kind == "FLOAT" and tok.value == float("inf"):
                        yield emit(Token("FLOAT", float("-inf"), tok.line, tok.col - 1))
                        continue
                    else:
                        raise self.error(f"Unexpected: -{tok.value}")
                else:
                    # List item marker
                    yield emit(Token("DASH", "-", self.line, self.col))
                    self.advance()
                    continue

            # Punctuation
            if ch == ":":
                yield emit(Token("COLON", ":", self.line, self.col))
                self.advance()
                continue

            if ch == ",":
                yield emit(Token("COMMA", ",", self.line, self.col))
                self.advance()
                continue

            if ch == "[":
                yield emit(Token("LBRACKET", "[", self.line, self.col))
                self.advance()
                continue

            if ch == "]":
                yield emit(Token("RBRACKET", "]", self.line, self.col))
                self.advance()
                continue

            if ch == "{":
                yield emit(Token("LBRACE", "{", self.line, self.col))
                self.advance()
                continue

            if ch == "}":
                yield emit(Token("RBRACE", "}", self.line, self.col))
                self.advance()
                continue

            # Identifier or keyword
            if ch.isalpha() or ch == "_":
                yield emit(self.read_identifier())
                continue

            # Check if we're in a context where this might be an invalid key
            # (after { or , in an inline object)
            if self._last_token and self._last_token.kind in ("LBRACE", "COMMA"):
                raise self.error("Invalid key")

            raise self.error(f'Unexpected character "{ch}"')

        # Final newline token if needed
        if not self.at_line_start:
            yield emit(Token("NEWLINE", "\n", self.line, self.col))

        yield emit(Token("EOF", None, self.line, self.col))
