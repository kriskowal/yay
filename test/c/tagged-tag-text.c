YAY_OBJECT(
    "a", yay_tagged("com.example.sku", yay_int(1)),
    "b", yay_tagged("match:any", yay_int(2)),
    "c", yay_tagged("image/png", yay_int(3)),
    "d", yay_tagged("my tag", yay_int(4)),
    "e", yay_tagged("a#b", yay_int(5)),
    "f", yay_tagged("a  # b", yay_int(6)),
    "g", yay_tagged("it's", yay_int(7)),
    "h", yay_tagged("a(b", yay_int(8)),
    "i", yay_tagged("\"q\"", yay_int(9)),
    "j", yay_tagged("a, b] }", yay_int(10))
)
