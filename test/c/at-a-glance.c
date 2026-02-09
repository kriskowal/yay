YAY_OBJECT(
    "and-objects-too", YAY_OBJECT(
        "from-their-floating-friends", yay_float(6.283185307179586),
        "integers-are-distinct", yay_int(42)
    ),
    "arrays", YAY_ARRAY(
        yay_string("may"),
        yay_string("have"),
        yay_string("many"),
        yay_string("values")
    ),
    "block", YAY_OBJECT(
        "array", YAY_ARRAY(
            yay_string("But"),
            yay_string("this"),
            yay_string("one's")
        ),
        "bytes", yay_bytes_from_hex("b0b5c0fffefacade"),
        "object", YAY_OBJECT("mine", yay_null()),
        "string", yay_string("This is a string.\nThere are many like it.\n")
    ),
    "concatenated", yay_string("I'm not dead yet. I feel happy!"),
    "inline", YAY_OBJECT(
        "array", YAY_ARRAY(
            yay_float(INFINITY),
            yay_float(-INFINITY),
            yay_float(NAN)
        ),
        "bytes", yay_bytes_from_hex("f33dface"),
        "object", YAY_OBJECT("bigint", yay_int(1), "float64", yay_float(2.0)),
        "string", yay_string("is concise")
    ),
    "name with spaces", yay_string("works too"),
    "roses-are-red", yay_bool(true),
    "unicode-code-point", yay_string("😀"),
    "violets-are-blue", yay_bool(false)
)
