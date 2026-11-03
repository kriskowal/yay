map[string]any{
	"a": yay.Tagged{Tag: "t", Payload: "x"},
	"b": yay.Tagged{Tag: "s", Payload: big.NewInt(1)},
	"c": yay.Tagged{Tag: "a)  # b", Payload: big.NewInt(2)},
}
