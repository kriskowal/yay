//go:generate go run cmd/gen_fixtures/main.go

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

func TestFixtures(t *testing.T) {
	for name, expected := range fixtures {
		t.Run(name, func(t *testing.T) {
			yayPath := filepath.Join("..", "test", "yay", name+".yay")
			input, err := os.ReadFile(yayPath)
			if err != nil {
				t.Fatalf("failed to read %s: %v", yayPath, err)
			}

			got, err := UnmarshalFile(input, name+".yay")
			if err != nil {
				t.Fatalf("Unmarshal error: %v", err)
			}

			if !deepEqual(got, expected) {
				t.Errorf("mismatch\ngot:  %#v\nwant: %#v", got, expected)
			}
		})
	}
}

func TestErrorCases(t *testing.T) {
	nayDir := filepath.Join("..", "test", "nay")
	entries, err := os.ReadDir(nayDir)
	if err != nil {
		t.Fatalf("failed to read nay dir: %v", err)
	}

	for _, entry := range entries {
		if !strings.HasSuffix(entry.Name(), ".nay") {
			continue
		}

		baseName := strings.TrimSuffix(entry.Name(), ".nay")
		t.Run(baseName, func(t *testing.T) {
			nayPath := filepath.Join(nayDir, entry.Name())
			errorPath := filepath.Join(nayDir, baseName+".error")

			input, err := os.ReadFile(nayPath)
			if err != nil {
				t.Fatalf("failed to read %s: %v", nayPath, err)
			}

			expectedError, err := os.ReadFile(errorPath)
			if err != nil {
				t.Fatalf("failed to read %s: %v", errorPath, err)
			}
			expectedPattern := strings.TrimSpace(string(expectedError))

			_, parseErr := UnmarshalFile(input, entry.Name())
			if parseErr == nil {
				t.Fatalf("expected error containing %q, got success", expectedPattern)
			}

			if !strings.Contains(parseErr.Error(), expectedPattern) {
				t.Errorf("error mismatch\ngot:  %s\nwant: contains %q", parseErr.Error(), expectedPattern)
			}
		})
	}
}

// deepEqual compares two values, handling special cases like NaN and *big.Int
func deepEqual(a, b any) bool {
	// Handle NaN
	if af, ok := a.(float64); ok {
		if bf, ok := b.(float64); ok {
			if math.IsNaN(af) && math.IsNaN(bf) {
				return true
			}
		}
	}

	// Handle *big.Int
	if ai, ok := a.(*big.Int); ok {
		if bi, ok := b.(*big.Int); ok {
			return ai.Cmp(bi) == 0
		}
	}

	// Handle []byte
	if ab, ok := a.([]byte); ok {
		if bb, ok := b.([]byte); ok {
			if len(ab) != len(bb) {
				return false
			}
			for i := range ab {
				if ab[i] != bb[i] {
					return false
				}
			}
			return true
		}
	}

	// Handle []any
	if as, ok := a.([]any); ok {
		if bs, ok := b.([]any); ok {
			if len(as) != len(bs) {
				return false
			}
			for i := range as {
				if !deepEqual(as[i], bs[i]) {
					return false
				}
			}
			return true
		}
	}

	// Handle map[string]any
	if am, ok := a.(map[string]any); ok {
		if bm, ok := b.(map[string]any); ok {
			if len(am) != len(bm) {
				return false
			}
			for k, av := range am {
				bv, ok := bm[k]
				if !ok || !deepEqual(av, bv) {
					return false
				}
			}
			return true
		}
	}

	return reflect.DeepEqual(a, b)
}
