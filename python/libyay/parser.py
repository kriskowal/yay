"""
YAY parser - parses YAY input into Python objects.
"""

import math
from typing import Any, TextIO
from .lexer import Lexer, Token
from .errors import YaySyntaxError


class Parser:
    """
    Recursive descent parser for YAY.

    Handles:
    - All YAY value types (null, int, float, bool, string, array, object, bytes)
    - Block and inline forms
    - Significant indentation
    """

    def __init__(self, source: str):
        self.source = source
        self.source_lines = source.split("\n")
        self.lexer = Lexer(source)
        self.tokens: list[Token] = list(self.lexer.tokenize())
        self.pos = 0

    def error(self, message: str, token: Token | None = None) -> YaySyntaxError:
        if token is None:
            token = self.peek()
        return YaySyntaxError(message, token.line, token.col)

    def char_at(self, line: int, col: int) -> str:
        """Get character at given line (1-based) and column (1-based)."""
        if line < 1 or line > len(self.source_lines):
            return ""
        line_content = self.source_lines[line - 1]
        if col < 1 or col > len(line_content):
            return ""
        return line_content[col - 1]

    def check_no_space_after(self, token: Token, char: str) -> None:
        """Check that there's no space immediately after the token."""
        next_col = token.col + len(char)
        if self.char_at(token.line, next_col) == " ":
            raise YaySyntaxError(
                f'Unexpected space after "{char}"', token.line, next_col
            )

    def check_no_space_before(self, token: Token, char: str) -> None:
        """Check that there's no space immediately before the token."""
        prev_col = token.col - 1
        if prev_col >= 1 and self.char_at(token.line, prev_col) == " ":
            raise YaySyntaxError(
                f'Unexpected space before "{char}"', token.line, prev_col
            )

    def validate_inline_syntax(
        self, line: int, start_col: int, open_char: str, close_char: str
    ) -> None:
        """Pre-validate inline array/object syntax for whitespace errors.

        Checks in order matching JavaScript reference:
        1. Space after opening bracket
        2. Space before closing bracket
        3. Comma spacing (with lookahead for trailing space before close)
        """
        line_content = (
            self.source_lines[line - 1] if line <= len(self.source_lines) else ""
        )
        s = line_content[start_col - 1 :] if start_col <= len(line_content) else ""

        in_single = False
        in_double = False
        escape = False
        depth = 0

        for i, ch in enumerate(s):
            if escape:
                escape = False
                continue
            if in_single:
                if ch == "\\":
                    escape = True
                elif ch == "'":
                    in_single = False
                continue
            if in_double:
                if ch == "\\":
                    escape = True
                elif ch == '"':
                    in_double = False
                continue
            if ch == "'":
                in_single = True
                continue
            if ch == '"':
                in_double = True
                continue
            if ch == open_char:
                depth += 1
                if i + 1 < len(s) and s[i + 1] == " ":
                    raise YaySyntaxError(
                        f'Unexpected space after "{open_char}"', line, start_col + i + 1
                    )
                continue
            if ch == close_char:
                if i > 0 and s[i - 1] == " ":
                    raise YaySyntaxError(
                        f'Unexpected space before "{close_char}"',
                        line,
                        start_col + i - 1,
                    )
                if depth > 0:
                    depth -= 1
                continue
            if ch == ",":
                if i > 0 and s[i - 1] == " ":
                    raise YaySyntaxError(
                        f'Unexpected space before ","', line, start_col + i - 1
                    )
                if i + 1 < len(s) and s[i + 1] != " " and s[i + 1] != close_char:
                    # Lookahead to see if next closing bracket at same depth has space before it
                    # If so, report that error instead of missing space after comma
                    lookahead_depth = depth
                    in_s = False
                    in_d = False
                    esc = False
                    next_is_closing_with_space = False
                    for j in range(i + 1, len(s)):
                        cj = s[j]
                        if esc:
                            esc = False
                            continue
                        if in_s:
                            if cj == "\\":
                                esc = True
                            elif cj == "'":
                                in_s = False
                            continue
                        if in_d:
                            if cj == "\\":
                                esc = True
                            elif cj == '"':
                                in_d = False
                            continue
                        if cj == "'":
                            in_s = True
                            continue
                        if cj == '"':
                            in_d = True
                            continue
                        if cj == open_char:
                            lookahead_depth += 1
                            continue
                        if cj == close_char:
                            if lookahead_depth == depth:
                                next_is_closing_with_space = j > 0 and s[j - 1] == " "
                                break
                            if lookahead_depth > 0:
                                lookahead_depth -= 1
                            continue
                        if cj == "," and lookahead_depth == depth:
                            break
                    if not next_is_closing_with_space:
                        raise YaySyntaxError(
                            f'Expected space after ","', line, start_col + i + 1
                        )
                if i + 2 < len(s) and s[i + 1] == " " and s[i + 2] == " ":
                    raise YaySyntaxError(
                        f'Unexpected space after ","', line, start_col + i + 2
                    )
                continue

    def peek(self, offset: int = 0) -> Token:
        """Peek at token at current position + offset."""
        idx = self.pos + offset
        if idx >= len(self.tokens):
            return self.tokens[-1]  # EOF
        return self.tokens[idx]

    def advance(self) -> Token:
        """Consume and return current token."""
        token = self.peek()
        self.pos += 1
        return token

    def expect(self, kind: str) -> Token:
        """Consume token of expected kind or raise error."""
        token = self.peek()
        if token.kind != kind:
            raise self.error(f"Expected {kind}, got {token.kind}")
        return self.advance()

    def skip_newlines(self) -> None:
        """Skip NEWLINE and INDENT tokens."""
        while self.peek().kind in ("NEWLINE", "INDENT"):
            self.advance()

    def parse(self) -> Any:
        """Parse the entire document."""
        # Check if document is comment-only (has content but no values)
        has_content = bool(self.source.strip())

        # Skip leading newlines only (not indents)
        while self.peek().kind == "NEWLINE":
            self.advance()

        # Check for unexpected indentation at root level
        if self.peek().kind == "INDENT":
            indent = self.peek().value
            if indent > 0:
                raise YaySyntaxError("Unexpected indent", self.peek().line, 1)
            self.advance()

        if self.peek().kind == "EOF":
            # If there was content (comments) but no value, that's an error
            if has_content:
                raise YaySyntaxError("No value found in document", 1, 1)
            return None

        result = self.parse_value(0)

        # Skip trailing whitespace
        self.skip_newlines()

        if self.peek().kind != "EOF":
            raise self.error("Unexpected extra content")

        return result

    def parse_value(self, min_indent: int) -> Any:
        """Parse a value at the given indentation level."""
        token = self.peek()

        # Inline values
        if token.kind == "NULL":
            self.advance()
            return None

        if token.kind == "BOOL":
            self.advance()
            return token.value

        if token.kind == "INT":
            self.advance()
            return token.value

        if token.kind == "FLOAT":
            self.advance()
            return token.value

        if token.kind == "STRING":
            # Check if this is a quoted object key
            if self.peek(1).kind == "COLON":
                return self.parse_block_object(min_indent)
            self.advance()
            return token.value

        if token.kind == "BYTES":
            self.advance()
            return token.value

        # Inline array
        if token.kind == "LBRACKET":
            return self.parse_inline_array()

        # Inline object
        if token.kind == "LBRACE":
            return self.parse_inline_object()

        # Block array (starts with dash)
        if token.kind == "DASH":
            return self.parse_block_array(min_indent)

        # Block string (quote alone or with content)
        if token.kind == "IDENT" and token.value == "":
            # This shouldn't happen, but handle gracefully
            raise self.error("Empty identifier")

        # Identifier could be object key
        if token.kind == "IDENT":
            # Look ahead to see if this is a key: value pair
            if self.peek(1).kind == "COLON":
                return self.parse_block_object(min_indent)
            elif self.peek(1).kind == "IDENT":
                # Invalid key character (space in key name)
                # The space is at the position between the two identifiers
                space_col = token.col + len(token.value)
                raise YaySyntaxError("Invalid key character", token.line, space_col)
            else:
                # Bare words are not valid - strings must be quoted
                first_char = token.value[0] if token.value else "?"
                raise YaySyntaxError(
                    f'Unexpected character "{first_char}"', token.line, token.col
                )

        raise self.error(f"Unexpected token: {token.kind}")

    def parse_inline_array(self) -> list:
        """Parse an inline array [a, b, c]."""
        lbracket = self.peek()
        self.expect("LBRACKET")

        # Check for newline immediately after [ (multiline inline array is invalid)
        if self.peek().kind == "NEWLINE":
            raise YaySyntaxError(
                "Unexpected newline in inline array", lbracket.line, lbracket.col
            )

        # Pre-validate whitespace in the correct order
        self.validate_inline_syntax(lbracket.line, lbracket.col, "[", "]")

        items = []

        while self.peek().kind != "RBRACKET":
            if self.peek().kind == "EOF":
                raise self.error("Unterminated array")
            if self.peek().kind == "NEWLINE":
                raise YaySyntaxError(
                    "Unexpected newline in inline array", lbracket.line, lbracket.col
                )

            items.append(self.parse_inline_value())

            if self.peek().kind == "COMMA":
                self.advance()
            elif self.peek().kind != "RBRACKET":
                raise self.error(f"Expected ',' or ']', got {self.peek().kind}")

        self.expect("RBRACKET")
        return items

    def parse_inline_object(self) -> dict:
        """Parse an inline object {a: 1, b: 2}."""
        lbrace = self.peek()
        self.expect("LBRACE")

        # Check for newline immediately after { (multiline inline object is invalid)
        if self.peek().kind == "NEWLINE":
            raise YaySyntaxError(
                "Unexpected newline in inline object", lbrace.line, lbrace.col
            )

        # Pre-validate whitespace in the correct order
        self.validate_inline_syntax(lbrace.line, lbrace.col, "{", "}")

        obj = {}

        while self.peek().kind != "RBRACE":
            if self.peek().kind == "EOF":
                raise self.error("Unterminated object")
            if self.peek().kind == "NEWLINE":
                raise YaySyntaxError(
                    "Unexpected newline in inline object", lbrace.line, lbrace.col
                )

            # Parse key
            key = self.parse_key()

            # Check colon spacing
            colon = self.peek()
            if colon.kind != "COLON":
                raise YaySyntaxError(
                    "Expected colon after key", lbrace.line, lbrace.col
                )
            self.expect("COLON")
            self.check_no_space_before(colon, ":")
            # Colon must be followed by exactly one space
            next_col = colon.col + 1
            if self.char_at(colon.line, next_col) != " ":
                raise YaySyntaxError('Expected space after ":"', colon.line, next_col)
            if self.char_at(colon.line, next_col + 1) == " ":
                raise YaySyntaxError(
                    'Unexpected space after ":"', colon.line, next_col + 1
                )

            # Parse value
            value = self.parse_inline_value()
            obj[key] = value

            if self.peek().kind == "COMMA":
                self.advance()
            elif self.peek().kind != "RBRACE":
                raise self.error(f"Expected ',' or '}}', got {self.peek().kind}")

        self.expect("RBRACE")
        return obj

    def parse_inline_value(self) -> Any:
        """Parse an inline value (no block forms)."""
        token = self.peek()

        if token.kind == "NULL":
            self.advance()
            return None

        if token.kind == "BOOL":
            self.advance()
            return token.value

        if token.kind == "INT":
            self.advance()
            return token.value

        if token.kind == "FLOAT":
            self.advance()
            return token.value

        if token.kind == "STRING":
            self.advance()
            return token.value

        if token.kind == "BYTES":
            self.advance()
            return token.value

        if token.kind == "LBRACKET":
            return self.parse_inline_array()

        if token.kind == "LBRACE":
            return self.parse_inline_object()

        raise self.error(f"Expected value, got {token.kind}")

    def parse_key(self) -> str:
        """Parse an object key (identifier or quoted string)."""
        token = self.peek()

        if token.kind == "IDENT":
            self.advance()
            return token.value

        if token.kind == "STRING":
            self.advance()
            return token.value

        raise self.error(f"Expected key, got {token.kind}")

    def parse_block_array(self, base_indent: int) -> list:
        """Parse a block array (dash-prefixed items)."""
        items = []

        while True:
            token = self.peek()

            # Check if we're still in the array
            if token.kind == "INDENT":
                indent = token.value
                if indent < base_indent:
                    break
                if indent > base_indent and items:
                    # This is continuation of previous item
                    break
                self.advance()
                token = self.peek()

            if token.kind != "DASH":
                break

            dash_token = token
            self.advance()  # consume dash

            # Check for exactly one space after dash
            next_col = dash_token.col + 1
            if self.char_at(dash_token.line, next_col) != " ":
                raise YaySyntaxError(
                    'Expected space after "-"', dash_token.line, next_col
                )
            if self.char_at(dash_token.line, next_col + 1) == " ":
                # Check if this is the first dash at the root level (leading space error)
                # or a nested/indented dash (space after dash error)
                if base_indent == 0 and dash_token.col == 1:
                    raise YaySyntaxError("Unexpected leading space", dash_token.line, 1)
                else:
                    raise YaySyntaxError(
                        'Unexpected space after "-"', dash_token.line, dash_token.col
                    )

            # Parse the item value
            item = self.parse_array_item(base_indent + 2)
            items.append(item)

            # Skip to next line
            if self.peek().kind == "NEWLINE":
                self.advance()

        return items

    def parse_array_item(self, item_indent: int) -> Any:
        """Parse a single array item value."""
        token = self.peek()

        # Nested array (dash immediately after dash)
        if token.kind == "DASH":
            return self.parse_block_array(item_indent)

        # Inline values
        if token.kind in ("NULL", "BOOL", "INT", "FLOAT", "STRING", "BYTES"):
            return self.parse_inline_value()

        if token.kind == "LBRACKET":
            return self.parse_inline_array()

        if token.kind == "LBRACE":
            return self.parse_inline_object()

        # Object value
        if token.kind == "IDENT" and self.peek(1).kind == "COLON":
            return self.parse_block_object(item_indent)

        # Bare words are not valid - strings must be quoted
        if token.kind == "IDENT":
            first_char = token.value[0] if token.value else "?"
            raise YaySyntaxError(
                f'Unexpected character "{first_char}"', token.line, token.col
            )

        raise self.error(f"Expected array item value, got {token.kind}")

    def parse_block_object(self, base_indent: int) -> dict:
        """Parse a block object (key: value pairs)."""
        obj = {}
        current_indent = base_indent

        while True:
            token = self.peek()

            # Handle indentation
            if token.kind == "INDENT":
                indent = token.value
                if indent < base_indent:
                    break
                if indent > base_indent and obj:
                    # This shouldn't happen at object level
                    break
                current_indent = indent
                self.advance()
                token = self.peek()

            # Check for key
            if token.kind not in ("IDENT", "STRING"):
                break

            # Check this is actually a key (followed by colon)
            if self.peek(1).kind != "COLON":
                break

            key_token = self.peek()
            key = self.parse_key()

            # Check colon and spacing
            colon = self.peek()
            self.expect("COLON")

            # Check for space before colon (only for unquoted keys)
            if key_token.kind == "IDENT":
                self.check_no_space_before(colon, ":")
            else:
                # For quoted keys, check space before colon
                prev_col = colon.col - 1
                if prev_col >= 1 and self.char_at(colon.line, prev_col) == " ":
                    raise YaySyntaxError(
                        f'Unexpected space before ":"', colon.line, prev_col
                    )

            # Check for exactly one space after colon (if value is on same line)
            next_token = self.peek()
            if next_token.kind != "NEWLINE":
                next_col = colon.col + 1
                if self.char_at(colon.line, next_col) != " ":
                    raise YaySyntaxError(
                        'Expected space after ":"', colon.line, next_col
                    )
                if self.char_at(colon.line, next_col + 1) == " ":
                    raise YaySyntaxError(
                        'Unexpected space after ":"', colon.line, next_col + 1
                    )

            # Parse value
            value = self.parse_object_value(current_indent)
            obj[key] = value

            # Skip newline
            if self.peek().kind == "NEWLINE":
                self.advance()

        return obj

    def parse_object_value(self, key_indent: int) -> Any:
        """Parse the value part of a key: value pair."""
        token = self.peek()

        # Empty object
        if token.kind == "LBRACE":
            next_token = self.peek(1)
            if next_token.kind == "RBRACE":
                # Validate no space inside empty object
                if next_token.col != token.col + 1:
                    raise YaySyntaxError(
                        'Unexpected space after "{"', token.line, token.col + 1
                    )
                self.advance()
                self.advance()
                return {}
            return self.parse_inline_object()

        # Inline values on same line
        if token.kind in ("NULL", "BOOL", "INT", "FLOAT"):
            return self.parse_inline_value()

        if token.kind == "STRING":
            # Check if this is a block string with content on same line (invalid in property context)
            # Block strings start with ` or " followed by space and content
            line_content = (
                self.source_lines[token.line - 1]
                if token.line <= len(self.source_lines)
                else ""
            )
            if token.col <= len(line_content):
                char_at_token = line_content[token.col - 1] if token.col > 0 else ""
                if char_at_token == "`":
                    # Backtick block string - check if there's content after it on the same line
                    rest_of_line = line_content[token.col :]
                    if rest_of_line.startswith(" ") and len(rest_of_line.strip()) > 0:
                        raise YaySyntaxError(
                            "Expected newline after block leader in property",
                            token.line,
                            token.col,
                        )
            return self.parse_inline_value()

        if token.kind == "BYTES":
            # Check if this is a block byte array with content on same line (invalid in property context)
            # Block byte arrays start with > followed by content
            line_content = (
                self.source_lines[token.line - 1]
                if token.line <= len(self.source_lines)
                else ""
            )
            if token.col <= len(line_content):
                char_at_token = line_content[token.col - 1] if token.col > 0 else ""
                if char_at_token == ">":
                    # Check if there's hex content after > on the same line
                    rest_of_line = line_content[token.col :]
                    # Skip optional space
                    if rest_of_line.startswith(" "):
                        rest_of_line = rest_of_line[1:]
                    # Check if there's hex content (not just a comment or empty)
                    if rest_of_line and rest_of_line[0] in "0123456789abcdef":
                        raise YaySyntaxError(
                            "Expected newline after block leader in property",
                            token.line,
                            token.col,
                        )
            return self.parse_inline_value()

        if token.kind == "LBRACKET":
            return self.parse_inline_array()

        # Block string starting with quote
        # (handled by lexer as STRING token)

        # Newline means nested block value
        if token.kind == "NEWLINE":
            self.advance()

            # Check indent of next line
            if self.peek().kind != "INDENT":
                raise self.error("Expected value after property")

            indent_token = self.peek()
            child_indent = indent_token.value

            # For named arrays, the array can be at same indent as key
            # e.g., "arrayName:\n- item1\n- item2"
            if child_indent < key_indent:
                raise self.error("Expected indentation for nested value")

            self.advance()  # consume INDENT

            next_token = self.peek()

            # Block array (can be at same indent for named arrays)
            if next_token.kind == "DASH":
                return self.parse_block_array(child_indent)

            # Block object
            if next_token.kind in ("IDENT", "STRING") and self.peek(1).kind == "COLON":
                return self.parse_block_object(child_indent)

            # Concatenated quoted strings (multiple quoted strings on consecutive lines)
            if next_token.kind == "STRING":
                result = self.parse_concatenated_strings(child_indent)
                if result is not None:
                    return result
                # Single string on new line is invalid

            raise self.error("Unexpected indent")

        raise self.error(f"Expected value after colon, got {token.kind}")

    def parse_concatenated_strings(self, base_indent: int) -> str | None:
        """Parse multiple quoted strings on consecutive lines.

        Returns None if there's only one string (single string on new line is invalid).
        """
        parts = []

        while True:
            token = self.peek()

            # Skip newlines and check indent
            if token.kind == "NEWLINE":
                self.advance()
                if self.peek().kind != "INDENT":
                    break
                indent_token = self.peek()
                if indent_token.value < base_indent:
                    break
                self.advance()  # consume INDENT
                token = self.peek()

            if token.kind != "STRING":
                break

            parts.append(token.value)
            self.advance()

        # Require at least 2 strings for concatenation
        # A single string on a new line is invalid (use inline syntax instead)
        if len(parts) < 2:
            return None

        return "".join(parts)


def loads(s: str) -> Any:
    """Parse a YAY string into Python objects."""
    # Handle block strings specially - they need raw source access
    parser = Parser(s)
    return parser.parse()


def load(fp: TextIO) -> Any:
    """Parse a YAY file into Python objects."""
    return loads(fp.read())
