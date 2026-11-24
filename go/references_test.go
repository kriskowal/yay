package yay

import (
	"math/big"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

// MEH's leniency is not YAY's, so the corpus's MEH reference cases,
// test/meh/reference-*.meh, are refused as YAY. The corpus reference cases
// in test/yay and test/nay are read with the rest of the corpus.
func TestReferenceMehCasesRefused(t *testing.T) {
	dir := filepath.Join("..", "test", "meh")
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatalf("failed to read %s: %v", dir, err)
	}
	var names []string
	for _, entry := range entries {
		name := entry.Name()
		if strings.HasPrefix(name, "reference-") && strings.HasSuffix(name, ".meh") {
			names = append(names, name)
		}
	}
	if len(names) < 2 {
		t.Fatalf("no MEH reference cases in %s", dir)
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
	if got != MustReference("https://x") || !reflect.DeepEqual(got, MustReference("https://x")) {
		t.Errorf("got %#v, want the reference https://x", got)
	}
	for _, c := range [][2]any{
		{MustReference("https://x/"), MustReference("https://x")},
		// By text, so percent-encodings are not normalized
		{MustReference("a:%2f"), MustReference("a:%2F")},
		// Never a string, nor a tagged value
		{got, "https://x"},
		{MustReference("https://x"), Tagged{"https://x", nil}},
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
		{"<ocapn://a.tcp> (Counter)\n", Tagged{"Counter", MustReference("ocapn://a.tcp")}},
		// Unchanged in strings and tags, and bytes without a scheme
		{"a: '<https://x>' (<b:c>)\nb: <ff>\n", map[string]any{"a": Tagged{"<b:c>", "<https://x>"}, "b": []byte{0xff}}},
		// A reference's punctuation is its own
		{"a: [<x:a,b]c#d>, 1]  # c\n", map[string]any{"a": []any{MustReference("x:a,b]c#d"), big.NewInt(1)}}},
		{"a: {b: <x:(t)>} (u)\n", map[string]any{"a": Tagged{"u", map[string]any{"b": MustReference("x:(t)")}}}},
	} {
		got, err := Unmarshal([]byte(c.in))
		if err != nil {
			t.Errorf("%q: %v", c.in, err)
		} else if !deepEqual(got, c.want) {
			t.Errorf("%q\ngot:  %#v\nwant: %#v", c.in, got, c.want)
		}
	}
}

func TestReferenceMadeOnlyFromReferenceText(t *testing.T) {
	for _, uri := range []string{"", "not a uri", "HTTPS://x", "urn:a b", "https://a>b", "https://x/%zz", "./a.yay"} {
		if r, err := NewReference(uri); err == nil {
			t.Errorf("NewReference(%q) = %v, want an error", uri, r)
		} else if !strings.HasPrefix(err.Error(), "Not a reference: ") {
			t.Errorf("NewReference(%q): %v", uri, err)
		}
		func() {
			defer func() {
				if recover() == nil {
					t.Errorf("MustReference(%q) did not panic", uri)
				}
			}()
			MustReference(uri)
		}()
	}
	for _, uri := range []string{"urn:x", "https://example.com/a?b=c#d", "urn:%C3%A9", "urn:é"} {
		r, err := NewReference(uri)
		if err != nil || r.URI() != uri || r.String() != uri {
			t.Errorf("NewReference(%q) = %v, %v", uri, r, err)
		}
		if got, err := Unmarshal([]byte("<" + uri + ">\n")); err != nil || got != r {
			t.Errorf("%q: got %v, %v, want %v", uri, got, err, r)
		}
	}
}
