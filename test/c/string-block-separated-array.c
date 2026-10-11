YAY_OBJECT(
    "items", YAY_ARRAY(
        yay_string("\nBare.\n"),
        yay_string("Same line.\n"),
        YAY_OBJECT(
            "body", yay_string("Last property of a list item.\n"),
            "title", yay_string("a")
        ),
        YAY_OBJECT(
            "body", yay_string("First property of a list item.\n"),
            "title", yay_string("b")
        )
    ),
    "next", yay_int(1)
)
