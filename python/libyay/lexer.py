"""
YAY lexer - tokenizes YAY input.
"""

import re
from dataclasses import dataclass
from typing import Iterator
from .errors import YaySyntaxError
from .reference import Reference, check_uri, scheme_len


SCALAR_KINDS = ("NULL", "BOOL", "INT", "FLOAT", "STRING", "BYTES", "REFERENCE")


def trailing_text_error(source: str, line: int, col: int) -> YaySyntaxError:
    """The error for a scalar at line and col with text fused after it, as
    the reference names it: by the scalar's first character."""
    lines = source.split("\n")
    text = lines[line - 1] if line <= len(lines) else ""
    ch = text[col - 1] if 0 < col <= len(text) else ""
    # A reference: the character after its ">"
    if ch == "<" and scheme_len(text, col) is not None:
        end = text.find(">", col)
        if end >= 0:
            after = text[end + 1 : end + 2]
            return YaySyntaxError(f'Unexpected character "{after}"', line, end + 2)
    # An inline bytes token begins after its "<"
    if ch == "<" or (1 < col <= len(text) + 1 and text[col - 2] == "<"):
        return YaySyntaxError("Unmatched angle bracket", line, col)
    return YaySyntaxError(f'Unexpected character "{ch}"', line, col)


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

    def check_comment_spacing(self) -> None:
        """Refuse a comment, at the current "#", with one space before it
        rather than two."""
        before = self.source[max(0, self.pos - 2) : self.pos]
        if before.endswith(" ") and before != "  ":
            raise self.error("Expected two spaces before a comment")

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

        # The text before the backtick on its line: a property ("key: `") or
        # a bare opener at root or after list leaders ("`", "- `").
        line_start = self.source.rfind("\n", 0, self.pos) + 1
        if self._last_token is not None and self._last_token.kind == "TAG":
            # A tagged block string: the tags before the backtick are not
            # its leader
            prefix = self.source[line_start : self._tag_run_start]
        else:
            prefix = self.source[line_start : self.pos]
        in_property = prefix.rstrip().endswith(":")
        if in_property:
            # The block ends at the key's column, past any list leaders.
            end_column = len(prefix) - len(prefix.lstrip(" -"))
        else:
            end_column = len(prefix) - len(prefix.lstrip(" "))

        self.advance()  # consume '`'

        # Check what follows the backtick
        next_ch = self.peek()
        if next_ch == "\n":
            # Backtick alone on line
            self.advance()  # consume newline
            self.at_line_start = True
            return self._read_block_string_content(
                start_line, start_col, end_column, in_property, same_line=False
            )
        elif next_ch == " ":
            # Backtick followed by space - content on same line
            self.advance()  # consume space
            return self._read_block_string_content(
                start_line, start_col, end_column, in_property, same_line=True
            )
        else:
            raise self.error("Expected space or newline after '`'")

    def _read_block_string_content(
        self,
        start_line: int,
        start_col: int,
        end_column: int,
        in_property: bool,
        same_line: bool,
    ) -> Token:
        """Read block string content after the backtick introducer.

        Content lines are those indented past end_column, and blank lines
        among them. The least indentation of the content lines is stripped.
        A backtick alone on its line gives a leading newline, except as a
        property value. Leading blank lines are dropped, every blank line
        within is kept, and trailing blank lines collapse to the final newline.
        """
        first_line = None
        if same_line:
            # First line content is on the same line as the backtick
            chars = []
            while self.peek() and self.peek() != "\n":
                chars.append(self.advance())
            first_line = "".join(chars)
            if self.peek() == "\n":
                self.advance()
                self.at_line_start = True

        # Read subsequent lines as (indent, text), with None for a blank line
        body: list[tuple[int, str] | None] = []
        while self.pos < len(self.source):
            line_start = self.pos
            line_start_line = self.line
            spaces = 0
            while self.peek() == " ":
                spaces += 1
                self.advance()

            # A line indented to the end column or less ends the block
            if spaces <= end_column and self.peek() not in ("\n", ""):
                self.pos = line_start
                self.line = line_start_line
                self.col = 1
                self.at_line_start = True
                break

            # Empty line
            if self.peek() == "\n":
                body.append(None)
                self.advance()
                self.at_line_start = True
                continue

            # EOF
            if self.peek() == "":
                break

            chars = []
            while self.peek() and self.peek() != "\n":
                chars.append(self.advance())
            body.append((spaces, "".join(chars)))

            if self.peek() == "\n":
                self.advance()
                self.at_line_start = True

        min_indent = min((entry[0] for entry in body if entry is not None), default=0)
        lines = [] if first_line is None else [first_line]
        for entry in body:
            if entry is None:
                lines.append("")
            else:
                indent, text = entry
                lines.append(" " * (indent - min_indent) + text)

        # Trim leading and trailing empty lines
        while lines and lines[-1] == "":
            lines.pop()
        while lines and lines[0] == "":
            lines.pop(0)

        if not lines:
            # Empty block string is not allowed; the shared corpus gives
            # this error no location
            raise YaySyntaxError(
                'Empty block string not allowed (use "" or "\\n" explicitly)'
            )

        result = "\n".join(lines) + "\n"
        if not same_line and not in_property:
            result = "\n" + result

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
                # The shared corpus gives this error no location
                raise YaySyntaxError("Expected hex or comment in hex block")
            if next_ch == "\n":
                self.advance()
                self.at_line_start = True

        base_indent = (
            self.current_line_indent if hasattr(self, "current_line_indent") else 0
        )
        hex_chars = []
        bad = False

        # Read first line (after > or > )
        while self.peek() and self.peek() not in "\n":
            ch = self.peek()
            if ch == "#":
                self.check_comment_spacing()
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
                # Counted, then refused after the odd-length check, as Rust does
                bad = True
                hex_chars.append(self.advance())

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
                    self.check_comment_spacing()
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
                    bad = True
                    hex_chars.append(self.advance())

            if self.peek() == "\n":
                self.advance()
                self.at_line_start = True

        hex_str = "".join(hex_chars)
        if len(hex_str) % 2 != 0:
            # At the start of the line that opens the block
            raise YaySyntaxError(
                "Odd number of hex digits in byte literal", start_line, base_indent + 1
            )
        if bad:
            raise YaySyntaxError("Invalid hex digit", start_line, start_col)

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
                    # A malformed escape is at its "u", and a bad code point
                    # at its "{", as the other ports place them
                    esc_line, u_col = self.line, self.col

                    def escape_error(message: str, col: int) -> YaySyntaxError:
                        return YaySyntaxError(message, esc_line, col)

                    self.advance()  # consume 'u'
                    # YAY uses \u{XXXXXX} syntax (variable-length with braces)
                    if self.peek() != "{":
                        raise escape_error("Bad escaped character", u_col)
                    brace_col = self.col
                    self.advance()  # consume '{'
                    hex_chars = ""
                    while self.peek() and self.peek() != "}":
                        h = self.peek()
                        if h not in "0123456789abcdefABCDEF":
                            raise escape_error("Bad Unicode escape", brace_col)
                        hex_chars += self.advance()
                        # Max 6 hex digits
                        if len(hex_chars) > 6:
                            raise escape_error("Bad Unicode escape", brace_col)
                    if self.peek() != "}":
                        raise escape_error("Bad Unicode escape", brace_col)
                    self.advance()  # consume '}'
                    if not hex_chars:
                        raise escape_error("Bad Unicode escape", brace_col)
                    codepoint = int(hex_chars, 16)
                    # Check for surrogate code points (U+D800 to U+DFFF)
                    if 0xD800 <= codepoint <= 0xDFFF:
                        raise escape_error("Illegal surrogate", brace_col)
                    if codepoint > 0x10FFFF:
                        raise escape_error("Unicode code point out of range", brace_col)
                    chars.append(chr(codepoint))
                    continue
                else:
                    raise self.error("Bad escaped character")
                self.advance()
            elif ord(ch) < 0x20:
                # Newline means unterminated string, at the line's last
                # character; other control chars are bad
                if ch == "\n" or ch == "\r":
                    raise YaySyntaxError("Unterminated string", self.line, self.col - 1)
                raise self.error("Bad character in string")
            else:
                chars.append(self.advance())

        return Token("STRING", "".join(chars), start_line, start_col)

    def _read_single_quoted_string(self, start_line: int, start_col: int) -> Token:
        """Read a single-quoted string: literal but for the escapes \\' and
        \\\\; any other backslash is content."""
        chars = []
        while True:
            ch = self.peek()
            if ch == "":
                raise YaySyntaxError("Unterminated string", self.line, self.col - 1)
            if ch == "\\" and self.peek(1) in ("'", "\\"):
                self.advance()
                chars.append(self.advance())
                continue
            if ch == "'":
                self.advance()
                break
            elif ord(ch) < 0x20:
                # Newline means unterminated string, at the line's last
                # character; other control chars are bad
                if ch == "\n" or ch == "\r":
                    raise YaySyntaxError("Unterminated string", self.line, self.col - 1)
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

        if already_consumed_open:
            # The literal, and its errors, begin at its "<"
            start_col -= 1
        else:
            self.advance()  # consume '<'

        hex_chars = []
        bad = False
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
                # Counted, then refused after the odd-length check, as the
                # block form and the other ports do
                bad = True
                hex_chars.append(self.advance())

        hex_str = "".join(hex_chars)
        if len(hex_str) % 2 != 0:
            raise YaySyntaxError(
                "Odd number of hex digits in byte literal", start_line, start_col
            )
        if bad:
            raise YaySyntaxError("Invalid hex digit", start_line, start_col)

        try:
            value = bytes.fromhex(hex_str)
        except ValueError as e:
            raise self.error(f"Invalid hex: {e}")

        return Token("BYTES", value, start_line, start_col)

    def read_reference(self) -> Token:
        """Read a reference, "<" and a scheme to the next ">" on the line,
        which always closes it. Its text is the URI, verbatim."""
        start_line = self.line
        start_col = self.col
        end = self.source.find(">", self.pos)
        newline = self.source.find("\n", self.pos)
        if end < 0 or 0 <= newline < end:
            raise YaySyntaxError("Unmatched angle bracket", start_line, start_col)
        uri = self.source[self.pos + 1 : end]
        fault = check_uri(uri)
        if fault is not None:
            message, offset = fault
            col = start_col if offset == 0 else start_col + 1 + offset
            raise YaySyntaxError(message, start_line, col)
        self.advance(end + 1 - self.pos)
        return Token("REFERENCE", Reference(uri), start_line, start_col)

    def read_tag(self) -> Token:
        """Read a tag, "(" to ")", escaped as a double-quoted string is but
        with \\) in the place of \\". One space comes before a tag unless it
        begins its line, and after it the line ends, an inline container goes
        on, another tag or a block follows after one space, or an inline
        comment begins."""
        start_line = self.line
        start_col = self.col
        if self.pos > 0 and self.source[self.pos - 1] != "\n":
            if self.source[self.pos - 1] != " ":
                raise self.error("Expected one space before a tag")
            if self.pos > 1 and self.source[self.pos - 2] == " ":
                # At the first of the spaces
                spaces = 2
                while self.pos > spaces and self.source[self.pos - spaces - 1] == " ":
                    spaces += 1
                raise YaySyntaxError(
                    'Unexpected space before "("', self.line, self.col - spaces
                )
        self.advance()  # consume '('
        chars = []
        while True:
            ch = self.peek()
            if ch == "" or ch == "\n":
                raise YaySyntaxError("Unterminated string", start_line, start_col)
            if ch == ")":
                self.advance()
                break
            if ch == "\t":
                raise self.error("Tab not allowed (use spaces)")
            if ch == "\\":
                self.advance()
                esc = self.peek()
                simple = {
                    ")": ")",
                    "\\": "\\",
                    "/": "/",
                    "b": "\b",
                    "f": "\f",
                    "n": "\n",
                    "r": "\r",
                    "t": "\t",
                }
                if esc in simple:
                    chars.append(simple[esc])
                    self.advance()
                    continue
                if esc != "u":
                    raise self.error("Bad escaped character")
                # As in a double-quoted string: a malformed escape is at its
                # "u", and a bad code point at its "{"
                if self.peek(1) != "{":
                    raise self.error("Bad escaped character")
                self.advance()  # consume 'u'
                brace_line, brace_col = self.line, self.col
                self.advance()  # consume '{'
                hex_chars = ""
                while self.peek() != "}":
                    h = self.peek()
                    hex_digit = h != "" and h in "0123456789abcdefABCDEF"
                    if not hex_digit or len(hex_chars) == 6:
                        raise YaySyntaxError(
                            "Bad Unicode escape", brace_line, brace_col
                        )
                    hex_chars += self.advance()
                self.advance()  # consume '}'
                if not hex_chars:
                    raise YaySyntaxError("Bad Unicode escape", brace_line, brace_col)
                codepoint = int(hex_chars, 16)
                if 0xD800 <= codepoint <= 0xDFFF:
                    raise YaySyntaxError("Illegal surrogate", brace_line, brace_col)
                if codepoint > 0x10FFFF:
                    raise YaySyntaxError(
                        "Unicode code point out of range", brace_line, brace_col
                    )
                chars.append(chr(codepoint))
                continue
            chars.append(self.advance())

        after = self.peek()
        if after == "(":
            raise self.error("Expected one space before a tag")
        if after not in ("", "\n", ",", "]", "}", " "):
            raise self.error(f'Unexpected character "{after}"')
        if after == " " and self.peek(1) == " " and self.peek(2) not in ("#", "("):
            raise self.error('Unexpected space after ")"')
        return Token("TAG", "".join(chars), start_line, start_col)

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
        # The columns of the open inline objects' braces
        braces: list[int] = []
        self._tag_run_start = 0

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
                # A lone quote on an indented line is no value, as an
                # inline value under a key is none
                if indent > 0 and ch == '"' and self.peek(1) in ("\n", ""):
                    raise YaySyntaxError("Unexpected indent", self.line, 1)

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
                self.check_comment_spacing()
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
                if scheme_len(self.source, self.pos + 1) is not None:
                    yield emit(self.read_reference())
                    continue
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
                elif next_ch == " ":
                    # GRAMMAR.md: no space after the opening "<"
                    raise self.error('Unexpected space after "<"')
                elif next_ch in "0123456789abcdef":
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
                    # Neither a reference nor bytes: refused as the Rust port
                    # refuses it, reading the rest of the line as bytes
                    end = self.source.find("\n", self.pos)
                    text = self.source[self.pos : end if end >= 0 else None]
                    text = text.split("#", 1)[0].rstrip(" ")
                    if not text.endswith(">"):
                        raise YaySyntaxError(
                            "Unmatched angle bracket", self.line, start_col
                        )
                    if len("".join(text[:-1].split())) % 2 != 0:
                        raise YaySyntaxError(
                            "Odd number of hex digits in byte literal",
                            self.line,
                            start_col,
                        )
                    raise YaySyntaxError("Invalid hex digit", self.line, start_col)

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

            # Tag, in parentheses
            if ch == "(":
                if self._last_token is None or self._last_token.kind != "TAG":
                    self._tag_run_start = self.pos
                yield emit(self.read_tag())
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
                braces.append(self.col)
                yield emit(Token("LBRACE", "{", self.line, self.col))
                self.advance()
                continue

            if ch == "}":
                if braces:
                    braces.pop()
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
                # At the inline object's "{", as the other ports place it
                col = braces[-1] if braces else self.col
                raise YaySyntaxError("Invalid key", self.line, col)

            # Text after a scalar on its line: the scalar is not one
            last = self._last_token
            if last and last.line == self.line and last.kind in SCALAR_KINDS:
                raise trailing_text_error(self.source, last.line, last.col)

            raise self.error(f'Unexpected character "{ch}"')

        # Final newline token if needed
        if not self.at_line_start:
            yield emit(Token("NEWLINE", "\n", self.line, self.col))

        yield emit(Token("EOF", None, self.line, self.col))
