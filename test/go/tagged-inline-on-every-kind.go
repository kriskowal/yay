map[string]any{
	"a": yay.Tagged{Tag: "date", Payload: "x"},
	"b": yay.Tagged{Tag: "t", Payload: "y"},
	"c": yay.Tagged{Tag: "rgb", Payload: []byte{0xff}},
	"d": yay.Tagged{Tag: "rgb", Payload: []any{big.NewInt(255), big.NewInt(136), big.NewInt(0)}},
	"e": yay.Tagged{Tag: "point", Payload: map[string]any{"x": big.NewInt(1)}},
	"f": yay.Tagged{Tag: "ms", Payload: 1.5},
	"g": yay.Tagged{Tag: "h", Payload: big.NewInt(-3)},
	"h": yay.Tagged{Tag: "us", Payload: 1000.0},
	"i": yay.Tagged{Tag: "ms", Payload: big.NewInt(10000)},
	"j": yay.Tagged{Tag: "x", Payload: big.NewInt(0)},
}
