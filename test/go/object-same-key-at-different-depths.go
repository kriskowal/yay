map[string]any{
	"a": map[string]any{"a": big.NewInt(1)},
	"b": []any{
		map[string]any{"a": big.NewInt(2)},
		map[string]any{"a": big.NewInt(3)},
	},
	"c": map[string]any{"a": map[string]any{"a": big.NewInt(4)}},
}
