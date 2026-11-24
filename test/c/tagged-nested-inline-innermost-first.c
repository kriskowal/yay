YAY_OBJECT(
    "a", yay_tagged("utc", yay_tagged("datetime", yay_string("2026-11-01T15:00:00"))),
    "b", yay_tagged("outer", yay_tagged("inner", YAY_ARRAY(yay_int(1), yay_int(2))))
)
