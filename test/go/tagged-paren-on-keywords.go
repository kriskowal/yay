map[string]any{
	"a": yay.Tagged{Tag: "t", Payload: true},
	"b": yay.Tagged{Tag: "null", Payload: nil},
	"c": yay.Tagged{Tag: "ms", Payload: math.NaN()},
	"d": yay.Tagged{Tag: "x", Payload: math.Inf(-1)},
}
