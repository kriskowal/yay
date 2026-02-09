map[string]any{
	"and-objects-too": map[string]any{
		"from-their-floating-friends": 6.283185307179586,
		"integers-are-distinct": big.NewInt(42),
	},
	"arrays": []any{"may", "have", "many", "values"},
	"block": map[string]any{
		"array": []any{"But", "this", "one's"},
		"bytes": []byte{0xb0, 0xb5, 0xc0, 0xff, 0xfe, 0xfa, 0xca, 0xde},
		"object": map[string]any{"mine": nil},
		"string": "This is a string.\nThere are many like it.\n",
	},
	"concatenated": "I'm not dead yet. I feel happy!",
	"inline": map[string]any{
		"array": []any{math.Inf(1), math.Inf(-1), math.NaN()},
		"bytes": []byte{0xf3, 0x3d, 0xfa, 0xce},
		"object": map[string]any{"bigint": big.NewInt(1), "float64": 2.0},
		"string": "is concise",
	},
	"name with spaces": "works too",
	"roses-are-red": true,
	"unicode-code-point": "😀",
	"violets-are-blue": false,
}
