"""
YAY error types.
"""


class YayError(Exception):
    """Base exception for YAY parsing/serialization errors."""

    def __init__(self, message: str, line: int | None = None, col: int | None = None):
        self.line = line
        self.col = col
        if line is not None and col is not None:
            message = f"{message} (line {line}, col {col})"
        elif line is not None:
            message = f"{message} (line {line})"
        super().__init__(message)


class YaySyntaxError(YayError):
    """Syntax error in YAY input."""

    pass
