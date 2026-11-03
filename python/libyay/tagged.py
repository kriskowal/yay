"""
YAY tagged values.
"""

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class Tagged:
    """A tagged value: a tag, which is a string, and one payload, any value.

    Two tagged values are equal when their tags and payloads are equal, and
    a tagged value never equals its payload. YAY gives no tag a meaning.
    """

    tag: str
    payload: Any
