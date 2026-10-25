YAY_OBJECT(
    "after", yay_int(1),
    "outer", YAY_OBJECT(
        "inner", YAY_OBJECT("text", yay_string("Two levels in.\n")),
        "sibling", yay_string("One level in.\n")
    )
)
