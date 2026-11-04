package yay

import (
	"encoding/hex"
	"encoding/json"
	"math"
	"math/big"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

// The cases of the references design: each .yay parses to its .yson, with
// a reference spelled "$uri", and each .nay and .meh is refused, MEH's
// leniency not being YAY's.
var referenceCasesDir = filepath.Join("..", "doc", "design", "references", "cases")

func referenceCaseNames(t *testing.T, ext string) []string {
	t.Helper()
	entries, err := os.ReadDir(referenceCasesDir)
	if err != nil {
		t.Fatalf("failed to read %s: %v", referenceCasesDir, err)
	}
	var names []string
	for _, entry := range entries {
		if strings.HasSuffix(entry.Name(), "."+ext) {
			names = append(names, entry.Name())
		}
	}
	if len(names) == 0 {
		t.Fatalf("no .%s cases in %s", ext, referenceCasesDir)
	}
	return names
}

// ysonString is a string as YSON writes it: escaped with "!" when it
// begins with "!" through "/".
func ysonString(s string) string {
	if s != "" && s[0] >= '!' && s[0] <= '/' {
		return "!" + s
	}
	return s
}

// toYSON returns a value as the cases' YSON spells it, with float64 for
// JSON numbers.
func toYSON(v any) any {
	switch v := v.(type) {
	case Reference:
		return "$" + string(v)
	case Tagged:
		return map[string]any{"#tag": ysonString(v.Tag), "payload": toYSON(v.Payload)}
	case []byte:
		return "*" + hex.EncodeToString(v)
	case *big.Int:
		if v.Sign() < 0 {
			return v.String()
		}
		return "+" + v.String()
	case float64:
		switch {
		case math.IsNaN(v):
			return "#nan"
		case math.IsInf(v, 1):
			return "#+inf"
		case math.IsInf(v, -1):
			return "#-inf"
		}
		return v
	case string:
		return ysonString(v)
	case []any:
		out := make([]any, len(v))
		for i, item := range v {
			out[i] = toYSON(item)
		}
		return out
	case map[string]any:
		out := make(map[string]any, len(v))
		for k, item := range v {
			if k != "" && strings.ContainsRune("#*!", rune(k[0])) {
				k = "!" + k
			}
			out[k] = toYSON(item)
		}
		return out
	}
	return v
}

func TestReferenceAcceptedCases(t *testing.T) {
	for _, name := range referenceCaseNames(t, "yay") {
		base := strings.TrimSuffix(name, ".yay")
		t.Run(base, func(t *testing.T) {
			input, err := os.ReadFile(filepath.Join(referenceCasesDir, name))
			if err != nil {
				t.Fatal(err)
			}
			want, err := os.ReadFile(filepath.Join(referenceCasesDir, base+".yson"))
			if err != nil {
				t.Fatal(err)
			}
			var w any
			if err := json.Unmarshal(want, &w); err != nil {
				t.Fatal(err)
			}
			got, err := UnmarshalFile(input, name)
			if err != nil {
				t.Fatalf("Unmarshal error: %v", err)
			}
			if d := toYSON(got); !reflect.DeepEqual(d, w) {
				t.Errorf("mismatch\ngot:  %#v\nwant: %#v", d, w)
			}
		})
	}
}

func TestReferenceRefusedCases(t *testing.T) {
	names := append(referenceCaseNames(t, "nay"), referenceCaseNames(t, "meh")...)
	for _, name := range names {
		t.Run(name, func(t *testing.T) {
			input, err := os.ReadFile(filepath.Join(referenceCasesDir, name))
			if err != nil {
				t.Fatal(err)
			}
			if got, err := UnmarshalFile(input, name); err == nil {
				t.Errorf("expected an error, got %#v", got)
			}
		})
	}
}

func TestReferenceRefused(t *testing.T) {
	// The Rust, JavaScript and Python ports' messages and columns
	for _, c := range []struct{ in, err string }{
		{"a: <HTTPS://x>\n", `Uppercase scheme "HTTPS" in reference (use lowercase) at 1:4 of <t>`},
		{"a: <https://x y>\n", "Unexpected space in reference at 1:14 of <t>"},
		{"a: <https://x\"y>\n", `Unexpected character """ in reference at 1:14 of <t>`},
		{"a: <https://x/%2g>\n", "Bad percent-encoding in reference (use % and two hex digits) at 1:15 of <t>"},
		{"a: <https://x/%2>\n", "Bad percent-encoding in reference (use % and two hex digits) at 1:15 of <t>"},
		{"a: <https://x\n", "Unmatched angle bracket at 1:4 of <t>"},
		{"a: <https://x>(t)\n", "Expected one space before a tag at 1:15 of <t>"},
		{"a: <https://x>rgb\n", `Unexpected character "r" at 1:15 of <t>`},
		{"<https://x>: 1\n", "Unexpected character \":\" at 1:12 of <t>"},
	} {
		_, err := UnmarshalFile([]byte(c.in), "t")
		if err == nil || !strings.HasPrefix(err.Error(), strings.SplitN(c.err, " at ", 2)[0]) {
			t.Errorf("%q: got %v, want %q", c.in, err, c.err)
		} else if err.Error() != c.err {
			t.Errorf("%q: got %v, want %q", c.in, err, c.err)
		}
	}
}

func TestReferenceEquality(t *testing.T) {
	got, err := Unmarshal([]byte("<https://x>\n"))
	if err != nil {
		t.Fatal(err)
	}
	if got != Reference("https://x") || !reflect.DeepEqual(got, Reference("https://x")) {
		t.Errorf("got %#v, want the reference https://x", got)
	}
	for _, c := range [][2]any{
		{Reference("https://x/"), Reference("https://x")},
		// By text, so percent-encodings are not normalized
		{Reference("a:%2f"), Reference("a:%2F")},
		// Never a string, nor a tagged value
		{got, "https://x"},
		{Reference("https://x"), Tagged{"https://x", nil}},
	} {
		if c[0] == c[1] || reflect.DeepEqual(c[0], c[1]) || deepEqual(c[0], c[1]) {
			t.Errorf("%#v should not equal %#v", c[0], c[1])
		}
	}
}

func TestReferenceParse(t *testing.T) {
	for _, c := range []struct {
		in   string
		want any
	}{
		{"<ocapn://a.tcp> (Counter)\n", Tagged{"Counter", Reference("ocapn://a.tcp")}},
		// Unchanged in strings and tags, and bytes without a scheme
		{"a: '<https://x>' (<b:c>)\nb: <ff>\n", map[string]any{"a": Tagged{"<b:c>", "<https://x>"}, "b": []byte{0xff}}},
		// A reference's punctuation is its own
		{"a: [<x:a,b]c#d>, 1]  # c\n", map[string]any{"a": []any{Reference("x:a,b]c#d"), big.NewInt(1)}}},
		{"a: {b: <x:(t)>} (u)\n", map[string]any{"a": Tagged{"u", map[string]any{"b": Reference("x:(t)")}}}},
	} {
		got, err := Unmarshal([]byte(c.in))
		if err != nil {
			t.Errorf("%q: %v", c.in, err)
		} else if !deepEqual(got, c.want) {
			t.Errorf("%q\ngot:  %#v\nwant: %#v", c.in, got, c.want)
		}
	}
}
