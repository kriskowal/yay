({
  "a": [
    tagged("s", 10n),
    tagged("x", "ff"),
    tagged("a, b", [1n]),
    tagged("t", true),
  ],
  "b": {
    "c": tagged("rgb", Uint8Array.from([0xff, 0x88, 0x00])),
    "d": tagged("m", 2n),
    "e": tagged("x}y", "v"),
    "f": tagged(")", 1n),
  },
})
