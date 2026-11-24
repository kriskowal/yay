YAY_OBJECT(
    "a", yay_tagged("date", yay_string("x")),
    "b", yay_tagged("t", yay_string("y")),
    "c", yay_tagged("rgb", yay_bytes_from_hex("ff")),
    "d", yay_tagged("rgb", YAY_ARRAY(yay_int(255), yay_int(136), yay_int(0))),
    "e", yay_tagged("point", YAY_OBJECT("x", yay_int(1))),
    "f", yay_tagged("ms", yay_float(1.5)),
    "g", yay_tagged("h", yay_int(-3)),
    "h", yay_tagged("us", yay_float(1000.0)),
    "i", yay_tagged("ms", yay_int(10000)),
    "j", yay_tagged("x", yay_int(0))
)
