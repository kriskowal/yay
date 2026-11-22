map[string]any{
	"mirrors": []any{
		yay.MustReference("https://a.example/x"),
		yay.MustReference("https://b.example/x"),
		[]byte{0xff},
		[]byte{},
	},
}
