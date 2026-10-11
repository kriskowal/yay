map[string]any{
	"after": big.NewInt(1),
	"outer": map[string]any{
		"inner": map[string]any{"text": "Two levels in.\n"},
		"sibling": "One level in.\n",
	},
}
