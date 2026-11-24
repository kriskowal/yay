map[string]any{
	"peers": []any{
		yay.Tagged{Tag: "Counter", Payload: yay.MustReference("ocapn://a.tcp")},
		yay.MustReference("ocapn://b.tcp"),
	},
}
