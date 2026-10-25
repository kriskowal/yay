YAY_OBJECT(
    "", yay_string("the empty key"),
    "a\"b", yay_string("an escaped quote"),
    "a/b", yay_string("a slash"),
    "a\\b", yay_string("an escaped backslash"),
    "line\nbreak", yay_string("an escaped newline")
)
