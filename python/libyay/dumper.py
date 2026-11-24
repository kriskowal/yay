"""
YAY dumper - serializes Python objects to YAY format.
"""

import math
import re
from typing import Any, TextIO

from .reference import Reference, check_uri
from .tagged import Tagged


def _is_simple_key(key: str) -> bool:
    """Check if a key can be written without quotes."""
    if not key:
        return False
    if not (key[0].isalpha() or key[0] == "_"):
        return False
    return all(c.isalnum() or c == "_" for c in key)


def _needs_double_quote(s: str) -> bool:
    """Check if string needs double quotes (has escapes)."""
    for ch in s:
        if ch in "\n\r\t\b\f\\":
            return True
        if ord(ch) < 0x20:
            return True
    return False


def _escape_double_quoted(s: str) -> str:
    """Escape a string for double-quoted output."""
    result = []
    for ch in s:
        if ch == '"':
            result.append('\\"')
        elif ch == "\\":
            result.append("\\\\")
        elif ch == "\n":
            result.append("\\n")
        elif ch == "\r":
            result.append("\\r")
        elif ch == "\t":
            result.append("\\t")
        elif ch == "\b":
            result.append("\\b")
        elif ch == "\f":
            result.append("\\f")
        elif ord(ch) < 0x20:
            result.append(f"\\u{ord(ch):04x}")
        else:
            result.append(ch)
    return "".join(result)


def _escape_single_quoted(s: str) -> str:
    """Escape a string for single-quoted output."""
    return s.replace("'", "''")


def _format_tag(tag: str) -> str:
    """Format a tag: its characters as they are, but ")" as "\\)", "\\" as
    "\\\\" and the control characters escaped as in a double-quoted string."""
    result = []
    for ch in tag:
        if ch == ")":
            result.append("\\)")
        elif ch == "\\":
            result.append("\\\\")
        elif ch == "\n":
            result.append("\\n")
        elif ch == "\r":
            result.append("\\r")
        elif ch == "\t":
            result.append("\\t")
        elif ch == "\b":
            result.append("\\b")
        elif ch == "\f":
            result.append("\\f")
        elif ord(ch) < 0x20 or 0x7F <= ord(ch) <= 0x9F:
            result.append("\\u{%X}" % ord(ch))
        else:
            result.append(ch)
    return "(" + "".join(result) + ")"


def _untag(value: Any) -> tuple[list[str], Any]:
    """Split a value into its tags, outermost first, and its payload."""
    tags = []
    while isinstance(value, Tagged):
        tags.append(value.tag)
        value = value.payload
    return tags, value


def _is_block(value: Any) -> bool:
    """Is the payload written as a block, where blocks may go?"""
    return isinstance(value, (dict, list)) and bool(value)


def _format_prefix(tags: list[str]) -> str:
    """Format the tags before a block, outermost first."""
    return " ".join(_format_tag(tag) for tag in tags)


def _format_value(value: Any, indent: int = 0, inline: bool = False) -> str:
    """Format a value as YAY."""
    prefix = "  " * indent

    if isinstance(value, Tagged):
        tags, payload = _untag(value)
        if not inline and _is_block(payload):
            # A tag before the block, on its own line above it
            block = _format_value(payload, indent, inline=False)
            return f"{prefix}{_format_prefix(tags)}\n{block}"
        # Tags after the inline value, innermost first
        suffix = "".join(" " + _format_tag(tag) for tag in reversed(tags))
        return _format_value(payload, indent, inline=True) + suffix

    if value is None:
        return "null"

    if isinstance(value, bool):
        return "true" if value else "false"

    if isinstance(value, int):
        return str(value)

    if isinstance(value, float):
        if math.isnan(value):
            return "nan"
        if math.isinf(value):
            return "-infinity" if value < 0 else "infinity"
        # Format float to preserve distinction from int
        s = repr(value)
        # Ensure there's a decimal point
        if "." not in s and "e" not in s and "E" not in s:
            s += ".0"
        return s

    if isinstance(value, str):
        if _needs_double_quote(value):
            return f'"{_escape_double_quoted(value)}"'
        else:
            # Prefer single quotes for simple strings
            if "'" in value and '"' not in value:
                return f'"{value}"'
            return f"'{_escape_single_quoted(value)}'"

    if isinstance(value, bytes):
        return f"<{value.hex()}>"

    if isinstance(value, Reference):
        # Written as held: a text YAY cannot read back is refused
        fault = check_uri(value.uri)
        if fault is not None:
            raise ValueError(f"Cannot write reference {value.uri!r}: {fault[0]}")
        return f"<{value.uri}>"

    if isinstance(value, list):
        if inline or not value:
            # Inline array
            items = [_format_value(item, 0, inline=True) for item in value]
            return "[" + ", ".join(items) + "]"
        else:
            # Block array
            lines = []
            for item in value:
                tags, payload = _untag(item)
                if tags and isinstance(payload, dict) and payload:
                    # A tagged object: the tags on the item's line
                    lines.append(f"{prefix}- {_format_prefix(tags)}")
                    nested = _format_value(payload, 0, inline=False)
                    for line in nested.split("\n"):
                        lines.append(f"{prefix}  {line}")
                elif isinstance(item, (dict, list)) and item:
                    # Complex nested item
                    item_str = _format_value(item, indent + 1, inline=False)
                    if isinstance(item, list):
                        # Nested list - format inline or as nested block
                        lines.append(f"{prefix}- {_format_value(item, 0, inline=True)}")
                    else:
                        # Nested object
                        lines.append(f"{prefix}-")
                        for line in item_str.split("\n"):
                            if line.strip():
                                lines.append(f"{prefix}  {line}")
                else:
                    item_str = _format_value(item, 0, inline=True)
                    lines.append(f"{prefix}- {item_str}")
            return "\n".join(lines)

    if isinstance(value, dict):
        if inline or not value:
            # Inline object
            items = []
            for k, v in value.items():
                key_str = k if _is_simple_key(k) else f"'{_escape_single_quoted(k)}'"
                val_str = _format_value(v, 0, inline=True)
                items.append(f"{key_str}: {val_str}")
            return "{" + ", ".join(items) + "}"
        else:
            # Block object
            lines = []
            for k, v in value.items():
                key_str = k if _is_simple_key(k) else f"'{_escape_single_quoted(k)}'"
                tags, payload = _untag(v)

                if tags and _is_block(payload):
                    # A tagged block: the tags after the key
                    lines.append(f"{prefix}{key_str}: {_format_prefix(tags)}")
                    nested = _format_value(payload, indent + 1, inline=False)
                    lines.append(nested)
                elif isinstance(v, dict) and v:
                    # Nested object
                    lines.append(f"{prefix}{key_str}:")
                    nested = _format_value(v, indent + 1, inline=False)
                    lines.append(nested)
                elif isinstance(v, list) and v:
                    # Nested array
                    lines.append(f"{prefix}{key_str}:")
                    nested = _format_value(v, indent + 1, inline=False)
                    lines.append(nested)
                else:
                    # Inline value
                    val_str = _format_value(v, 0, inline=True)
                    lines.append(f"{prefix}{key_str}: {val_str}")
            return "\n".join(lines)

    raise TypeError(f"Cannot serialize type {type(value).__name__} to YAY")


def dumps(obj: Any, *, indent: bool = True) -> str:
    """
    Serialize Python objects to a YAY string.

    Args:
        obj: The Python object to serialize
        indent: If True (default), use block format for arrays/objects.
                If False, use inline format.

    Returns:
        YAY-formatted string
    """
    return _format_value(obj, 0, inline=not indent)


def dump(obj: Any, fp: TextIO, *, indent: bool = True) -> None:
    """
    Serialize Python objects to a YAY file.

    Args:
        obj: The Python object to serialize
        fp: File-like object to write to
        indent: If True (default), use block format for arrays/objects.
    """
    fp.write(dumps(obj, indent=indent))
