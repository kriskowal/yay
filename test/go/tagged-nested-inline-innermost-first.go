map[string]any{
	"a": yay.Tagged{Tag: "utc", Payload: yay.Tagged{Tag: "datetime", Payload: "2026-11-01T15:00:00"}},
	"b": yay.Tagged{Tag: "outer", Payload: yay.Tagged{Tag: "inner", Payload: []any{big.NewInt(1), big.NewInt(2)}}},
}
