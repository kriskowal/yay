map[string]any{
	"peers": []any{
		yay.Tagged{Tag: "Counter", Payload: yay.Reference("ocapn://a.tcp")},
		yay.Reference("ocapn://b.tcp"),
	},
}
