[]any{
	map[string]any{
		"a": map[string]any{"b": []any{big.NewInt(1), big.NewInt(2)}},
		"c": big.NewInt(3),
	},
	[]any{
		"d",
		map[string]any{"e": map[string]any{"f": big.NewInt(4)}},
	},
}
