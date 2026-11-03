map[string]any{
	"a": yay.Tagged{Tag: "a)b", Payload: big.NewInt(1)},
	"b": yay.Tagged{Tag: "back\\slash", Payload: big.NewInt(2)},
	"c": yay.Tagged{Tag: "tab\there", Payload: big.NewInt(3)},
	"d": yay.Tagged{Tag: "😀", Payload: big.NewInt(4)},
	"e": yay.Tagged{Tag: "a/b", Payload: big.NewInt(5)},
	"f": yay.Tagged{Tag: "\n", Payload: big.NewInt(6)},
}
