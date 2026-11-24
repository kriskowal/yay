package yay

import (
	"math"
	"math/big"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

// MEH's leniency is not YAY's, so the corpus's MEH tagged cases,
// test/meh/tagged-*.meh, are refused as YAY. The corpus tagged cases in
// test/yay and test/nay are read with the rest of the corpus.
func TestTaggedMehCasesRefused(t *testing.T) {
	dir := filepath.Join("..", "test", "meh")
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatalf("failed to read %s: %v", dir, err)
	}
	var names []string
	for _, entry := range entries {
		name := entry.Name()
		if strings.HasPrefix(name, "tagged-") && strings.HasSuffix(name, ".meh") {
			names = append(names, name)
		}
	}
	if len(names) < 3 {
		t.Fatalf("no MEH tagged cases in %s", dir)
	}
	for _, name := range names {
		t.Run(name, func(t *testing.T) {
			input, err := os.ReadFile(filepath.Join(dir, name))
			if err != nil {
				t.Fatal(err)
			}
			if got, err := UnmarshalFile(input, name); err == nil {
				t.Errorf("expected an error, got %#v", got)
			}
		})
	}
}

func TestTaggedEquality(t *testing.T) {
	// The test helper and reflect.DeepEqual agree where no NaN is involved
	if !deepEqual(Tagged{"s", big.NewInt(10)}, Tagged{"s", big.NewInt(10)}) ||
		!reflect.DeepEqual(Tagged{"s", big.NewInt(10)}, Tagged{"s", big.NewInt(10)}) {
		t.Error("equal tags and payloads should be equal")
	}
	if !deepEqual(Tagged{"t", []any{big.NewInt(1), map[string]any{"a": []byte{1}}}},
		Tagged{"t", []any{big.NewInt(1), map[string]any{"a": []byte{1}}}}) {
		t.Error("equal nested payloads should be equal")
	}
	if !deepEqual(Tagged{"n", math.NaN()}, Tagged{"n", math.NaN()}) {
		t.Error("deepEqual should see NaN payloads as equal, as it does untagged")
	}
	for _, c := range [][2]any{
		{Tagged{"s", big.NewInt(10)}, Tagged{"ms", big.NewInt(10)}},
		{Tagged{"s", big.NewInt(10)}, Tagged{"s", big.NewInt(11)}},
		// A tagged value never equals its payload, nor its YSON spelling
		{Tagged{"s", big.NewInt(10)}, big.NewInt(10)},
		{Tagged{"s", big.NewInt(10)}, map[string]any{"#tag": "s", "payload": big.NewInt(10)}},
		{Tagged{"a", Tagged{"b", big.NewInt(1)}}, Tagged{"b", Tagged{"a", big.NewInt(1)}}},
	} {
		if deepEqual(c[0], c[1]) || reflect.DeepEqual(c[0], c[1]) {
			t.Errorf("%#v should not equal %#v", c[0], c[1])
		}
	}
}

func TestTaggedParse(t *testing.T) {
	for _, c := range []struct {
		in   string
		want any
	}{
		// Suffixes innermost first, prefixes outermost first
		{"1 (a) (b)\n", Tagged{"b", Tagged{"a", big.NewInt(1)}}},
		{"- (a) (b)\n  - 1\n", []any{Tagged{"a", Tagged{"b", []any{big.NewInt(1)}}}}},
		{"(a) (b)\n- 1\n", Tagged{"a", Tagged{"b", []any{big.NewInt(1)}}}},
		// Escapes
		{"1 (a\\)b\\\\c)\n", Tagged{"a)b\\c", big.NewInt(1)}},
		{"1 (\\u{1F600})\n", Tagged{"\U0001F600", big.NewInt(1)}},
		// The empty tag and edge spaces
		{"1 ()\n", Tagged{"", big.NewInt(1)}},
		{"1 ( a )\n", Tagged{" a ", big.NewInt(1)}},
		// A tag's punctuation is its own
		{"1 (a: [b, #c)\n", Tagged{"a: [b, #c", big.NewInt(1)}},
		{"a: 1 (t)  # c\n", map[string]any{"a": Tagged{"t", big.NewInt(1)}}},
		// Inside inline containers, and on them
		{"x: [1 (a), {b: 2 (c)}] (d)\n", map[string]any{"x": Tagged{"d", []any{Tagged{"a", big.NewInt(1)}, map[string]any{"b": Tagged{"c", big.NewInt(2)}}}}}},
		// Before a block at a nested key, and a block string or bytes
		{"a:\n  b: (t)\n    c: 1\n", map[string]any{"a": map[string]any{"b": Tagged{"t", map[string]any{"c": big.NewInt(1)}}}}},
		{"a: (t) `\n  x\n", map[string]any{"a": Tagged{"t", "x\n"}}},
		{"a: (t) >\n  ff\n", map[string]any{"a": Tagged{"t", []byte{0xff}}}},
		{"- a: (t)\n    - 1\n", []any{map[string]any{"a": Tagged{"t", []any{big.NewInt(1)}}}}},
	} {
		got, err := Unmarshal([]byte(c.in))
		if err != nil {
			t.Errorf("%q: %v", c.in, err)
		} else if !deepEqual(got, c.want) {
			t.Errorf("%q\ngot:  %#v\nwant: %#v", c.in, got, c.want)
		}
	}
}

func TestTaggedRefused(t *testing.T) {
	for _, c := range []struct{ in, err string }{
		{"1(t)\n", "Expected one space before a tag"},
		{"1  (t)\n", `Unexpected space before "("`},
		{"1 (t)  (u)\n", `Unexpected space before "("`},
		{"1 (t)x\n", `Unexpected character "x"`},
		{"(t) 1\n", "A tag before a value needs a block below it"},
		{"a: (t) 1\n", "A tag before a value needs a block below it"},
		{"- (t) 1\n", "A tag before a value needs a block below it"},
		{"a:\n  b: (t)\n", "A tag before a value needs a block below it"},
		{"(t)\n", "A tag before a value needs a block below it"},
		{"1 (\\\")\n", "Bad escaped character"},
		{"1 (\\()\n", "Bad escaped character"},
		{"1 (\\x)\n", "Bad escaped character"},
		{"1 (\\u{D800})\n", "Illegal surrogate"},
		{"1 (\\u{110000})\n", "Unicode code point out of range"},
		{"1 (\\u{})\n", "Bad Unicode escape"},
		{"1 (a\tb)\n", "Tab not allowed"},
		{"1 (a\n", "Unterminated string"},
	} {
		_, err := Unmarshal([]byte(c.in))
		if err == nil || !strings.Contains(err.Error(), c.err) {
			t.Errorf("%q: got %v, want an error containing %q", c.in, err, c.err)
		}
	}
}
