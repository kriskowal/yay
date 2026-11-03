YAY_OBJECT(
    "color", yay_tagged("rgba", yay_bytes_from_hex("ff8800ff")),
    "count", yay_tagged("s", yay_int(10)),
    "odd", yay_tagged("a)b", yay_int(3)),
    "sku", yay_tagged("com.example/sku", yay_string("X-1")),
    "when", yay_tagged("utc", yay_tagged("datetime", yay_string("2026-11-03T05:00:00Z")))
)
