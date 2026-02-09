"""
YAY - Yet Another YAML

A Python parser and serializer for the YAY data format.
"""

from .parser import load, loads
from .dumper import dump, dumps
from .errors import YayError, YaySyntaxError

__all__ = ["load", "loads", "dump", "dumps", "YayError", "YaySyntaxError"]
__version__ = "1.0.0"
