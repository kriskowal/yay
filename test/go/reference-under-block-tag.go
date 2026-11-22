map[string]any{
	"links": yay.Tagged{Tag: "mirrors", Payload: []any{
		yay.MustReference("https://a.example/x"),
		yay.MustReference("https://b.example/x"),
	}},
}
