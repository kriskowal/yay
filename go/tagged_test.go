package yay

import (
	"encoding/hex"
	"encoding/json"
	"math"
	"math/big"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strconv"
	"strings"
	"testing"
)

// The cases of the tagged values design: each .yay parses to its .json,
// with a tagged value as {"#tag": tag, "payload": payload} and bytes as
// "*hex", and each .nay and .meh is refused, MEH's leniency not being YAY's.
var casesDir = filepath.Join("..", "doc", "design", "tagged-values-g", "cases")

func caseNames(t *testing.T, ext string) []string {
	t.Helper()
	entries, err := os.ReadDir(casesDir)
	if err != nil {
		t.Fatalf("failed to read %s: %v", casesDir, err)
	}
	var names []string
	for _, entry := range entries {
		if strings.HasSuffix(entry.Name(), "."+ext) {
			names = append(names, entry.Name())
		}
	}
	sort.Strings(names)
	if len(names) == 0 {
		t.Fatalf("no .%s cases in %s", ext, casesDir)
	}
	return names
}

// desugar returns a value as the cases' JSON spells it, with integers as
// "i:<digits>" and floats as float64, to compare with expected.
func desugar(v any) any {
	switch v := v.(type) {
	case Tagged:
		return map[string]any{"#tag": v.Tag, "payload": desugar(v.Payload)}
	case []byte:
		return "*" + hex.EncodeToString(v)
	case *big.Int:
		return "i:" + v.String()
	case float64:
		switch {
		case math.IsNaN(v):
			return "NaN"
		case math.IsInf(v, 1):
			return "Infinity"
		case math.IsInf(v, -1):
			return "-Infinity"
		}
		return v
	case []any:
		out := make([]any, len(v))
		for i, item := range v {
			out[i] = desugar(item)
		}
		return out
	case map[string]any:
		out := make(map[string]any, len(v))
		for k, item := range v {
			out[k] = desugar(item)
		}
		return out
	}
	return v
}

// expected reads a case's JSON with numbers as desugar spells them.
func expected(v any) any {
	switch v := v.(type) {
	case json.Number:
		s := string(v)
		if strings.ContainsAny(s, ".eE") {
			f, _ := strconv.ParseFloat(s, 64)
			return f
		}
		return "i:" + s
	case []any:
		for i, item := range v {
			v[i] = expected(item)
		}
	case map[string]any:
		for k, item := range v {
			v[k] = expected(item)
		}
	}
	return v
}

func TestTaggedAcceptedCases(t *testing.T) {
	for _, name := range caseNames(t, "yay") {
		base := strings.TrimSuffix(name, ".yay")
		t.Run(base, func(t *testing.T) {
			input, err := os.ReadFile(filepath.Join(casesDir, name))
			if err != nil {
				t.Fatal(err)
			}
			want, err := os.ReadFile(filepath.Join(casesDir, base+".json"))
			if err != nil {
				t.Fatal(err)
			}
			text := strings.ReplaceAll(string(want), "-Infinity", `"-Infinity"`)
			dec := json.NewDecoder(strings.NewReader(text))
			dec.UseNumber()
			var w any
			if err := dec.Decode(&w); err != nil {
				t.Fatal(err)
			}
			got, err := UnmarshalFile(input, name)
			if err != nil {
				t.Fatalf("Unmarshal error: %v", err)
			}
			if d := desugar(got); !reflect.DeepEqual(d, expected(w)) {
				t.Errorf("mismatch\ngot:  %#v\nwant: %#v", d, expected(w))
			}
		})
	}
}

func TestTaggedRefusedCases(t *testing.T) {
	names := append(caseNames(t, "nay"), caseNames(t, "meh")...)
	for _, name := range names {
		t.Run(name, func(t *testing.T) {
			input, err := os.ReadFile(filepath.Join(casesDir, name))
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
