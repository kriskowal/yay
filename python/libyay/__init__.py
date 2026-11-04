"""
YAY - Yet Another YAML

A Python parser and serializer for the YAY data format.
"""

from .parser import load, loads
from .dumper import dump, dumps
from .errors import YayError, YaySyntaxError
from .reference import Reference
from .tagged import Tagged

__all__ = [
    "load",
    "loads",
    "dump",
    "dumps",
    "Reference",
    "Tagged",
    "YayError",
    "YaySyntaxError",
]
__version__ = "2.0.0"
