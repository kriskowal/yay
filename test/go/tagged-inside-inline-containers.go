map[string]any{
	"a": []any{
		yay.Tagged{Tag: "s", Payload: big.NewInt(10)},
		yay.Tagged{Tag: "x", Payload: "ff"},
		yay.Tagged{Tag: "a, b", Payload: []any{big.NewInt(1)}},
		yay.Tagged{Tag: "t", Payload: true},
	},
	"b": map[string]any{
		"c": yay.Tagged{Tag: "rgb", Payload: []byte{0xff, 0x88, 0x00}},
		"d": yay.Tagged{Tag: "m", Payload: big.NewInt(2)},
		"e": yay.Tagged{Tag: "x}y", Payload: "v"},
		"f": yay.Tagged{Tag: ")", Payload: big.NewInt(1)},
	},
}
