map[string]any{
	"a": yay.Tagged{Tag: "com.example.sku", Payload: big.NewInt(1)},
	"b": yay.Tagged{Tag: "match:any", Payload: big.NewInt(2)},
	"c": yay.Tagged{Tag: "image/png", Payload: big.NewInt(3)},
	"d": yay.Tagged{Tag: "my tag", Payload: big.NewInt(4)},
	"e": yay.Tagged{Tag: "a#b", Payload: big.NewInt(5)},
	"f": yay.Tagged{Tag: "a  # b", Payload: big.NewInt(6)},
	"g": yay.Tagged{Tag: "it's", Payload: big.NewInt(7)},
	"h": yay.Tagged{Tag: "a(b", Payload: big.NewInt(8)},
	"i": yay.Tagged{Tag: "\"q\"", Payload: big.NewInt(9)},
	"j": yay.Tagged{Tag: "a, b] }", Payload: big.NewInt(10)},
}
