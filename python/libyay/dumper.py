"""
YAY dumper - serializes Python objects to YAY format.
"""

import math
import re
from typing import Any, TextIO


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


def _format_value(value: Any, indent: int = 0, inline: bool = False) -> str:
    """Format a value as YAY."""
    prefix = "  " * indent

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

    if isinstance(value, list):
        if inline or not value:
            # Inline array
            items = [_format_value(item, 0, inline=True) for item in value]
            return "[" + ", ".join(items) + "]"
        else:
            # Block array
            lines = []
            for item in value:
                if isinstance(item, (dict, list)) and item:
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

                if isinstance(v, dict) and v:
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
