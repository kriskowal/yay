YAY_OBJECT(
    "top", YAY_OBJECT(
        "list", YAY_ARRAY(
            YAY_OBJECT(
                "bytes", yay_bytes_from_hex("b0b5"),
                "tag", yay_string("x")
            ),
            YAY_OBJECT(
                "more", YAY_OBJECT("inner", yay_bytes_from_hex("0f0f"))
            )
        ),
        "solo", yay_int(1)
    )
)
