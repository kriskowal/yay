YAY_OBJECT(
    "a", YAY_ARRAY(
        yay_tagged("s", yay_int(10)),
        yay_tagged("x", yay_string("ff")),
        yay_tagged("a, b", YAY_ARRAY(yay_int(1))),
        yay_tagged("t", yay_bool(true))
    ),
    "b", YAY_OBJECT(
        "c", yay_tagged("rgb", yay_bytes_from_hex("ff8800")),
        "d", yay_tagged("m", yay_int(2)),
        "e", yay_tagged("x}y", yay_string("v")),
        "f", yay_tagged(")", yay_int(1))
    )
)
