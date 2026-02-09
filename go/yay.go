// Package yay implements encoding and decoding of YAY documents.
//
// YAY is Yet Another YAML - a data serialization format that is more
// expressive than JSON (supporting big integers and byte arrays) while
// having fewer surprising behaviors than YAML.
//
// # Parsing Pipeline
//
// The parser operates in three phases:
//
//  1. Scanner: Converts source text into scan lines, validating encoding
//     and extracting indentation and list markers.
//
//  2. Outline Lexer: Converts scan lines into a token stream with explicit
//     block start/stop markers based on indentation changes.
//
//  3. Value Parser: Recursively parses the token stream into Go values.
package yay

import (
	"encoding/hex"
	"fmt"
	"math"
	"math/big"
	"regexp"
	"strconv"
	"strings"
)

// ============================================================================
// Public API
// ============================================================================

// Unmarshal parses YAY-encoded data and returns the result.
//
// The mapping between YAY and Go values is:
//   - null -> nil
//   - boolean -> bool
//   - integer -> *big.Int
//   - float -> float64 (including NaN, Infinity, -Infinity)
//   - string -> string
//   - array -> []any
//   - object -> map[string]any
//   - bytes -> []byte
func Unmarshal(data []byte) (any, error) {
	return unmarshal(data, "")
}

// UnmarshalFile parses YAY-encoded data with a filename for error messages.
func UnmarshalFile(data []byte, filename string) (any, error) {
	return unmarshal(data, filename)
}

// Marshal returns the YAY encoding of v.
func Marshal(v any) ([]byte, error) {
	// TODO: implement encoder
	return nil, fmt.Errorf("not implemented")
}

// ============================================================================
// Internal Types
// ============================================================================

// parseContext carries filename for error reporting through the parse phases.
type parseContext struct {
	filename string
}

// scanLine represents a single line after the scanning phase.
// It captures the line's content, indentation level, and any list marker.
type scanLine struct {
	line    string // Content after indent and leader
	indent  int    // Number of leading spaces
	leader  string // "- " for list items, "" otherwise
	lineNum int    // Zero-based line number for error reporting
}

// tokenType identifies the kind of token in the outline lexer output.
type tokenType int

const (
	tokenStart tokenType = iota // Block start (list item or multiline construct)
	tokenStop                   // Block end (dedent)
	tokenText                   // Text content
	tokenBreak                  // Blank line
)

// token represents a single element in the token stream.
type token struct {
	typ     tokenType
	text    string
	indent  int
	lineNum int
	col     int
}

// ============================================================================
// Error Reporting
// ============================================================================

// locSuffix formats a location suffix for error messages.
// Returns empty string if no filename is set.
// Uses 1-based line and column numbers for human-readable output.
func locSuffix(ctx *parseContext, line, col int) string {
	if ctx == nil || ctx.filename == "" {
		return ""
	}
	return fmt.Sprintf(" at %d:%d of <%s>", line+1, col+1, ctx.filename)
}

// ============================================================================
// Phase 1: Scanner
// ============================================================================
//
// The scanner converts raw source text into scan lines. It performs:
//   - UTF-8 validation (no BOM, no forbidden code points)
//   - Whitespace validation (no tabs, no trailing spaces)
//   - Indentation counting
//   - List marker extraction (the "-" prefix)
//   - Comment filtering

func unmarshal(data []byte, filename string) (any, error) {
	source := string(data)
	ctx := &parseContext{filename: filename}

	// Phase 1: Scan source into lines
	lines, err := scan(source, ctx)
	if err != nil {
		return nil, err
	}

	// Phase 2: Convert lines to token stream
	tokens := outlineLex(lines)

	// Phase 3: Parse tokens into value
	return parseRoot(tokens, ctx)
}

// scan converts source text into scan lines with validation.
func scan(source string, ctx *parseContext) ([]scanLine, error) {
	// Validate: No BOM allowed
	if err := validateNoBOM(source, ctx); err != nil {
		return nil, err
	}

	// Validate: No forbidden code points
	if err := validateCodePoints(source, ctx); err != nil {
		return nil, err
	}

	// Process each line
	return scanLines(source, ctx)
}

// validateNoBOM checks that the source doesn't start with a UTF-8 BOM.
func validateNoBOM(source string, ctx *parseContext) error {
	if len(source) >= 3 && source[0] == 0xEF && source[1] == 0xBB && source[2] == 0xBF {
		return fmt.Errorf("Illegal BOM%s", locSuffix(ctx, 0, 0))
	}
	return nil
}

// isAllowedCodePoint checks whether a code point is allowed in a YAY document.
func isAllowedCodePoint(cp rune) bool {
	return cp == 0x000A ||
		(0x0020 <= cp && cp <= 0x007E) ||
		(0x00A0 <= cp && cp <= 0xD7FF) ||
		(0xE000 <= cp && cp <= 0xFFFD && !(0xFDD0 <= cp && cp <= 0xFDEF)) ||
		(0x10000 <= cp && cp <= 0x10FFFF && (cp&0xFFFF) < 0xFFFE)
}

// validateCodePoints checks that the source contains no forbidden code points.
func validateCodePoints(source string, ctx *parseContext) error {
	line := 0
	col := 0
	for _, r := range source {
		if !isAllowedCodePoint(r) {
			if r == '\t' {
				return fmt.Errorf("Tab not allowed (use spaces)%s", locSuffix(ctx, line, col))
			}
			if r >= 0xD800 && r <= 0xDFFF {
				return fmt.Errorf("Illegal surrogate%s", locSuffix(ctx, line, col))
			}
			return fmt.Errorf("Forbidden code point U+%04X%s", r, locSuffix(ctx, line, col))
		}
		if r == '\n' {
			line++
			col = 0
		} else {
			col++
		}
	}
	return nil
}

// scanLines processes each line of source, extracting indent and leader.
func scanLines(source string, ctx *parseContext) ([]scanLine, error) {
	var lines []scanLine
	lineStrings := strings.Split(source, "\n")

	for lineNum, lineStr := range lineStrings {
		// Validate: No trailing spaces
		if len(lineStr) > 0 && lineStr[len(lineStr)-1] == ' ' {
			return nil, fmt.Errorf("Unexpected trailing space%s", locSuffix(ctx, lineNum, len(lineStr)-1))
		}

		// Count leading spaces (indent)
		indent := countIndent(lineStr)

		rest := lineStr[indent:]

		// Skip top-level comments
		if strings.HasPrefix(rest, "#") && indent == 0 {
			continue
		}

		// Extract leader (list marker) and content
		leader, content, err := extractLeader(rest, lineNum, indent, ctx)
		if err != nil {
			return nil, err
		}

		lines = append(lines, scanLine{
			line:    content,
			indent:  indent,
			leader:  leader,
			lineNum: lineNum,
		})
	}

	return lines, nil
}

// countIndent returns the number of leading spaces in a line.
func countIndent(line string) int {
	indent := 0
	for indent < len(line) && line[indent] == ' ' {
		indent++
	}
	return indent
}

// extractLeader separates the list marker from line content.
// Returns (leader, content, error) where leader is "- " for list items.
// The list marker is always exactly two characters: dash and space.
func extractLeader(rest string, lineNum, indent int, ctx *parseContext) (string, string, error) {
	// "- " prefix is the list marker (dash + space)
	if strings.HasPrefix(rest, "- ") {
		return "- ", rest[2:], nil
	}

	// Compact list syntax (-value without space) is not allowed
	// But "-1", "-.5", and "-infinity" are valid numbers/keywords
	if strings.HasPrefix(rest, "-") && len(rest) >= 2 {
		second := rest[1]
		if second != ' ' && second != '.' && !(second >= '0' && second <= '9') && rest != "-infinity" {
			return "", "", fmt.Errorf("Expected space after \"-\"%s", locSuffix(ctx, lineNum, indent+1))
		}
	}

	// "*" or "* " at top level is an error (asterisk multiline bytes not allowed at root)
	if rest == "*" || strings.HasPrefix(rest, "* ") {
		return "", "", fmt.Errorf("Unexpected character \"*\"%s", locSuffix(ctx, lineNum, indent))
	}

	return "", rest, nil
}

// ============================================================================
// Phase 2: Outline Lexer
// ============================================================================
//
// The outline lexer converts scan lines into a token stream. It tracks
// indentation levels using a stack and emits:
//   - tokenStart: When a list item begins or indent increases
//   - tokenStop: When indent decreases (block ends)
//   - tokenText: Line content
//   - tokenBreak: Blank lines (coalesced)

// outlineLex converts scan lines to a token stream with block markers.
func outlineLex(lines []scanLine) []token {
	var tokens []token
	stack := []int{0} // Indent level stack, starts at 0
	top := 0          // Current indent level
	broken := false   // Whether we just emitted a break

	for _, sl := range lines {
		// Emit stops for each level we dedent past
		tokens, stack, top = emitDedents(tokens, stack, top, sl.indent)

		// Emit start for list items
		tokens, stack, top, broken = emitListStart(tokens, stack, top, broken, sl)

		// Emit text or break
		tokens, broken = emitContent(tokens, broken, sl)
	}

	// Close any remaining open blocks
	tokens = emitFinalStops(tokens, stack)

	return tokens
}

// emitDedents emits stop tokens when indentation decreases.
func emitDedents(tokens []token, stack []int, top, indent int) ([]token, []int, int) {
	for indent < top {
		tokens = append(tokens, token{typ: tokenStop, text: ""})
		stack = stack[:len(stack)-1]
		top = stack[len(stack)-1]
	}
	return tokens, stack, top
}

// emitListStart emits start tokens for list items.
func emitListStart(tokens []token, stack []int, top int, broken bool, sl scanLine) ([]token, []int, int, bool) {
	if len(sl.leader) == 0 {
		return tokens, stack, top, broken
	}

	if sl.indent > top {
		// New nested block
		tokens = append(tokens, token{
			typ:     tokenStart,
			text:    sl.leader,
			indent:  sl.indent,
			lineNum: sl.lineNum,
			col:     sl.indent,
		})
		stack = append(stack, sl.indent)
		top = sl.indent
		broken = false
	} else if sl.indent == top {
		// Sibling item - close previous, start new
		tokens = append(tokens, token{typ: tokenStop, text: ""})
		tokens = append(tokens, token{
			typ:     tokenStart,
			text:    sl.leader,
			indent:  sl.indent,
			lineNum: sl.lineNum,
			col:     sl.indent,
		})
		broken = false
	}

	return tokens, stack, top, broken
}

// emitContent emits text or break tokens for line content.
func emitContent(tokens []token, broken bool, sl scanLine) ([]token, bool) {
	if len(sl.line) > 0 {
		tokens = append(tokens, token{
			typ:     tokenText,
			text:    sl.line,
			indent:  sl.indent,
			lineNum: sl.lineNum,
			col:     sl.indent,
		})
		return tokens, false
	}

	// Empty line - emit break if not already broken
	if !broken {
		tokens = append(tokens, token{
			typ:     tokenBreak,
			text:    "",
			lineNum: sl.lineNum,
			col:     sl.indent,
		})
		return tokens, true
	}

	return tokens, broken
}

// emitFinalStops closes any remaining open blocks.
func emitFinalStops(tokens []token, stack []int) []token {
	for len(stack) > 1 {
		tokens = append(tokens, token{typ: tokenStop, text: ""})
		stack = stack[:len(stack)-1]
	}
	return tokens
}

// ============================================================================
// Phase 3: Value Parser
// ============================================================================
//
// The value parser recursively processes the token stream to build Go values.
// It handles:
//   - Scalars: null, booleans, numbers, strings
//   - Compounds: arrays (multiline and inline), objects
//   - Binary: byte arrays (inline and multiline)
//   - Block strings: multiline string literals

// parseRoot is the entry point for parsing a YAY document.
func parseRoot(tokens []token, ctx *parseContext) (any, error) {
	i := skipBreaksAndStops(tokens, 0)
	if i >= len(tokens) {
		return nil, fmt.Errorf("No value found in document <%s>", ctx.filename)
	}

	t := tokens[i]

	// Validate: No unexpected indent at root
	if t.typ == tokenText && t.indent > 0 {
		return nil, fmt.Errorf("Unexpected indent%s", locSuffix(ctx, t.lineNum, 0))
	}

	// Detect root object (key: value at indent 0)
	// But not inline objects starting with {
	if t.typ == tokenText && strings.Contains(t.text, ":") && t.indent == 0 && !strings.HasPrefix(t.text, "{") {
		value, next, err := parseRootObject(tokens, i, ctx)
		if err != nil {
			return nil, err
		}
		return ensureAtEnd(value, tokens, next, ctx)
	}

	// Parse as single value
	value, next, err := parseValue(tokens, i, ctx)
	if err != nil {
		return nil, err
	}
	return ensureAtEnd(value, tokens, next, ctx)
}

// ensureAtEnd verifies no content remains after parsing.
func ensureAtEnd(value any, tokens []token, i int, ctx *parseContext) (any, error) {
	j := skipBreaksAndStops(tokens, i)
	if j < len(tokens) {
		t := tokens[j]
		return nil, fmt.Errorf("Unexpected extra content%s", locSuffix(ctx, t.lineNum, t.col))
	}
	return value, nil
}

// skipBreaksAndStops advances past break and stop tokens.
func skipBreaksAndStops(tokens []token, i int) int {
	for i < len(tokens) && (tokens[i].typ == tokenStop || tokens[i].typ == tokenBreak) {
		i++
	}
	return i
}

// ============================================================================
// Value Parsing
// ============================================================================

// parseValue parses a single value from the token stream.
// Returns (value, nextIndex, error).
func parseValue(tokens []token, i int, ctx *parseContext) (any, int, error) {
	if i >= len(tokens) {
		return nil, i + 1, nil
	}

	t := tokens[i]

	// Validate text tokens
	if t.typ == tokenText {
		if err := validateTextToken(t, ctx); err != nil {
			return nil, 0, err
		}
	}

	// Handle block starts (list items)
	if t.typ == tokenStart && t.text == "- " {
		return parseMultilineArray(tokens, i, ctx, -1)
	}

	// Handle text content
	if t.typ == tokenText {
		return parseTextValue(tokens, i, ctx)
	}

	return nil, i + 1, nil
}

// validateTextToken checks for invalid text patterns.
func validateTextToken(t token, ctx *parseContext) error {
	if strings.HasPrefix(t.text, " ") {
		return fmt.Errorf("Unexpected leading space%s", locSuffix(ctx, t.lineNum, t.col))
	}
	if t.text == "$" {
		return fmt.Errorf("Unexpected character \"$\"%s", locSuffix(ctx, t.lineNum, t.col))
	}
	return nil
}

// parseTextValue parses a text token into the appropriate value type.
func parseTextValue(tokens []token, i int, ctx *parseContext) (any, int, error) {
	t := tokens[i]
	s := t.text

	// Try keywords
	if v, ok := parseKeyword(s); ok {
		return v, i + 1, nil
	}

	// Try numbers (with strict whitespace validation)
	if num, ok, err := parseNumberStrict(s, ctx, t.lineNum, t.col); err != nil {
		return nil, 0, err
	} else if ok {
		return num, i + 1, nil
	}

	// Try block string
	if isBlockStringStart(s) {
		firstLine := extractBlockStringFirstLine(s)
		// Use token's indent as base - block string content must be indented more
		return parseBlockStringWithIndent(tokens, i, firstLine, false, t.indent)
	}

	// Try quoted string
	if isQuotedString(s) {
		str, err := parseQuotedString(s, ctx, t.lineNum, t.col)
		if err != nil {
			return nil, 0, err
		}
		return str, i + 1, nil
	}

	// Try inline array
	if strings.HasPrefix(s, "[") {
		return parseInlineArrayValue(s, t, i, ctx)
	}

	// Try inline object
	if strings.HasPrefix(s, "{") {
		return parseInlineObjectValue(s, t, i, ctx)
	}

	// Try inline bytes
	if strings.HasPrefix(s, "<") && strings.Contains(s, ">") {
		bytes, err := parseAngleBytesStrict(s, ctx, t.lineNum, t.col)
		if err != nil {
			return nil, 0, err
		}
		return bytes, i + 1, nil
	}

	// Try block bytes (> introducer)
	if strings.HasPrefix(s, ">") {
		return parseBlockBytes(tokens, i, ctx)
	}

	// Try key:value pair
	if colonIdx := findColonOutsideQuotes(s); colonIdx >= 0 {
		return parseKeyValuePair(tokens, i, colonIdx, ctx)
	}

	// Fall back to scalar
	scalar, err := parseScalar(s, ctx, t.lineNum, t.col)
	if err != nil {
		return nil, 0, err
	}
	return scalar, i + 1, nil
}

// ============================================================================
// Keyword Parsing
// ============================================================================

// parseKeyword checks if s is a YAY keyword and returns its value.
func parseKeyword(s string) (any, bool) {
	switch s {
	case "null":
		return nil, true
	case "true":
		return true, true
	case "false":
		return false, true
	case "nan":
		return math.NaN(), true
	case "infinity":
		return math.Inf(1), true
	case "-infinity":
		return math.Inf(-1), true
	default:
		return nil, false
	}
}

// ============================================================================
// Number Parsing
// ============================================================================

var (
	integerRe = regexp.MustCompile(`^-?\d+$`)
	// Float patterns: with decimal point, or with exponent, or both
	floatRe = regexp.MustCompile(`^-?\d*\.\d*([eE][+-]?\d+)?$`)
	// Exponent-only float (no decimal point): e.g., 1e10
	floatExpRe = regexp.MustCompile(`^-?\d+[eE][+-]?\d+$`)
)

// parseNumber attempts to parse s as a number.
// Returns (*big.Int, true) for integers, (float64, true) for floats, (nil, false) otherwise.
func parseNumber(s string) (any, bool) {
	// Remove spaces (allowed as digit grouping)
	trimmed := strings.ReplaceAll(s, " ", "")

	// Try integer
	if integerRe.MatchString(trimmed) {
		n := new(big.Int)
		n.SetString(trimmed, 10)
		return n, true
	}

	// Try float with exponent only (no decimal point)
	if floatExpRe.MatchString(trimmed) {
		f, err := strconv.ParseFloat(trimmed, 64)
		if err == nil {
			return f, true
		}
	}

	// Try float (must have decimal point, but not just "." or "-.")
	if floatRe.MatchString(trimmed) && trimmed != "." && trimmed != "-." {
		f, err := strconv.ParseFloat(trimmed, 64)
		if err == nil {
			return f, true
		}
	}

	return nil, false
}

// parseNumberStrict parses a number with strict whitespace validation.
// Spaces are allowed for digit grouping in integers, but not around decimal points.
func parseNumberStrict(s string, ctx *parseContext, lineNum, col int) (any, bool, error) {
	// Check if it looks like a number at all
	trimmed := strings.ReplaceAll(s, " ", "")
	if trimmed == "" {
		return nil, false, nil
	}

	// Check if first char indicates a number (digit, minus, or leading dot)
	firstChar := trimmed[0]
	if firstChar != '-' && firstChar != '.' && (firstChar < '0' || firstChar > '9') {
		return nil, false, nil
	}

	// Check for uppercase E in exponent (must be lowercase)
	eIdx := strings.Index(s, "E")
	if eIdx >= 0 {
		return nil, false, fmt.Errorf("Uppercase exponent (use lowercase 'e')%s", locSuffix(ctx, lineNum, col+eIdx))
	}

	// Check for spaces around decimal point
	dotIdx := strings.Index(s, ".")
	if dotIdx >= 0 {
		// Check for space before decimal point (but not if dot is at start)
		if dotIdx > 0 && s[dotIdx-1] == ' ' {
			return nil, false, fmt.Errorf("Unexpected space in number%s", locSuffix(ctx, lineNum, col+dotIdx-1))
		}
		// Check for space after decimal point
		if dotIdx < len(s)-1 && s[dotIdx+1] == ' ' {
			return nil, false, fmt.Errorf("Unexpected space in number%s", locSuffix(ctx, lineNum, col+dotIdx+1))
		}
	}

	// Try integer
	if integerRe.MatchString(trimmed) {
		n := new(big.Int)
		n.SetString(trimmed, 10)
		return n, true, nil
	}

	// Try float with exponent only (no decimal point)
	if floatExpRe.MatchString(trimmed) {
		f, err := strconv.ParseFloat(trimmed, 64)
		if err == nil {
			return f, true, nil
		}
	}

	// Try float (must have decimal point, but not just "." or "-.")
	if floatRe.MatchString(trimmed) && trimmed != "." && trimmed != "-." {
		f, err := strconv.ParseFloat(trimmed, 64)
		if err == nil {
			return f, true, nil
		}
	}

	return nil, false, nil
}

// ============================================================================
// String Parsing
// ============================================================================

// isBlockStringStart checks if s starts a block string.
// Block strings start with ` alone or ` followed by space.
func isBlockStringStart(s string) bool {
	return s == "`" || (strings.HasPrefix(s, "`") && len(s) >= 2 && s[1] == ' ')
}

// extractBlockStringFirstLine extracts the first line content from a block string start.
func extractBlockStringFirstLine(s string) string {
	if len(s) > 2 {
		return s[2:] // Content after "` "
	}
	return "" // Backtick alone on line
}

// isQuotedString checks if s is a quoted string (double or single).
func isQuotedString(s string) bool {
	return (strings.HasPrefix(s, "\"") && len(s) > 1) ||
		(strings.HasPrefix(s, "'") && len(s) > 1)
}

// parseQuotedString parses a quoted string value.
func parseQuotedString(s string, ctx *parseContext, lineNum, col int) (string, error) {
	if strings.HasPrefix(s, "\"") {
		return parseDoubleQuotedString(s, ctx, lineNum, col)
	}
	if strings.HasPrefix(s, "'") {
		// Single-quoted strings are literal (no escapes)
		return s[1 : len(s)-1], nil
	}
	return s, nil
}

// parseDoubleQuotedString parses a JSON-style double-quoted string.
func parseDoubleQuotedString(s string, ctx *parseContext, lineNum, col int) (string, error) {
	if len(s) < 2 || s[0] != '"' {
		return s, nil
	}
	if s[len(s)-1] != '"' {
		return "", fmt.Errorf("Unterminated string%s", locSuffix(ctx, lineNum, col+len(s)-1))
	}

	var out strings.Builder
	runes := []rune(s)

	for i := 1; i < len(runes)-1; i++ {
		ch := runes[i]

		if ch == '\\' {
			// Handle escape sequence
			escaped, advance, err := parseEscapeSequence(runes, i, ctx, lineNum, col)
			if err != nil {
				return "", err
			}
			out.WriteString(escaped)
			i += advance
		} else if ch < 0x20 {
			// Control characters not allowed
			return "", fmt.Errorf("Bad character in string%s", locSuffix(ctx, lineNum, col+i))
		} else {
			out.WriteRune(ch)
		}
	}

	return out.String(), nil
}

// parseEscapeSequence parses a backslash escape sequence.
// Returns (unescaped string, characters to advance, error).
func parseEscapeSequence(runes []rune, i int, ctx *parseContext, lineNum, col int) (string, int, error) {
	if i+1 >= len(runes)-1 {
		return "", 0, fmt.Errorf("Bad escaped character%s", locSuffix(ctx, lineNum, col+i+1))
	}

	esc := runes[i+1]
	switch esc {
	case '"':
		return "\"", 1, nil
	case '\\':
		return "\\", 1, nil
	case '/':
		return "/", 1, nil
	case 'b':
		return "\b", 1, nil
	case 'f':
		return "\f", 1, nil
	case 'n':
		return "\n", 1, nil
	case 'r':
		return "\r", 1, nil
	case 't':
		return "\t", 1, nil
	case 'u':
		return parseUnicodeEscape(runes, i, ctx, lineNum, col)
	default:
		return "", 0, fmt.Errorf("Bad escaped character%s", locSuffix(ctx, lineNum, col+i+1))
	}
}

// parseUnicodeEscape parses a \u{XXXXXX} escape sequence (variable-length with braces).
func parseUnicodeEscape(runes []rune, i int, ctx *parseContext, lineNum, col int) (string, int, error) {
	// Column of the 'u' character (for "Bad escaped character" error)
	uCol := col + i + 1
	// Column of the opening brace (for other errors)
	braceCol := col + i + 2

	// Expect opening brace after \u
	if i+2 >= len(runes)-1 || runes[i+2] != '{' {
		// Old-style \uXXXX syntax is not supported - report as bad escaped character
		return "", 0, fmt.Errorf("Bad escaped character%s", locSuffix(ctx, lineNum, uCol))
	}

	// Find closing brace
	start := i + 3
	end := start
	for end < len(runes)-1 && runes[end] != '}' {
		end++
	}

	if end >= len(runes)-1 || runes[end] != '}' {
		return "", 0, fmt.Errorf("Bad Unicode escape%s", locSuffix(ctx, lineNum, braceCol))
	}

	// Validate hex digits
	for j := start; j < end; j++ {
		if !isHexDigit(runes[j]) {
			return "", 0, fmt.Errorf("Bad Unicode escape%s", locSuffix(ctx, lineNum, braceCol))
		}
	}

	if end == start {
		return "", 0, fmt.Errorf("Bad Unicode escape%s", locSuffix(ctx, lineNum, braceCol))
	}

	// Too many hex digits (max 6 for Unicode code points up to 10FFFF)
	if end-start > 6 {
		return "", 0, fmt.Errorf("Bad Unicode escape%s", locSuffix(ctx, lineNum, braceCol))
	}

	// Parse code point
	hexStr := string(runes[start:end])
	var code int64
	fmt.Sscanf(hexStr, "%x", &code)

	// Reject surrogates
	if code >= 0xD800 && code <= 0xDFFF {
		return "", 0, fmt.Errorf("Illegal surrogate%s", locSuffix(ctx, lineNum, braceCol))
	}

	// Reject code points beyond Unicode range
	if code > 0x10FFFF {
		return "", 0, fmt.Errorf("Unicode code point out of range%s", locSuffix(ctx, lineNum, braceCol))
	}

	// Return the character and the number of runes consumed (including \u{...})
	// advance = length of "u{...}" = 1 + 1 + (end-start) + 1 = end - start + 3
	advance := end - i
	return string(rune(code)), advance, nil
}

// isHexDigit checks if r is a hexadecimal digit.
func isHexDigit(r rune) bool {
	return (r >= '0' && r <= '9') || (r >= 'a' && r <= 'f') || (r >= 'A' && r <= 'F')
}

// ============================================================================
// Block String Parsing
// ============================================================================

// parseBlockString parses a multiline block string.
// firstLine is the content on the same line as the opening backtick (empty if backtick alone).
// inPropertyContext indicates if this is a property value (affects leading newline behavior).
func parseBlockString(tokens []token, i int, firstLine string, inPropertyContext bool) (string, int, error) {
	return parseBlockStringWithIndent(tokens, i, firstLine, inPropertyContext, -1)
}

// parseBlockStringWithIndent parses a multiline block string with a base indent constraint.
// baseIndent is the indent of the key; content must be at indent > baseIndent.
// If baseIndent is -1, no indent constraint is applied.
func parseBlockStringWithIndent(tokens []token, i int, firstLine string, inPropertyContext bool, baseIndent int) (string, int, error) {
	var lines []string
	if firstLine != "" {
		lines = append(lines, firstLine)
	}
	i++

	// Collect continuation lines with their indentation
	continuationLines, i := collectBlockStringLinesWithIndent(tokens, i, baseIndent)

	// Normalize indentation
	lines = append(lines, normalizeBlockIndent(continuationLines)...)

	// Build result with appropriate leading newline
	body := buildBlockStringResult(firstLine, lines, inPropertyContext)
	if body == "" {
		return "", i, fmt.Errorf("Empty block string not allowed (use \"\" or \"\\n\" explicitly)")
	}
	return body, i, nil
}

// blockLine represents a line in a block string with its indent.
type blockLine struct {
	indent  int
	text    string
	isBreak bool
}

// collectBlockStringLines gathers continuation lines for a block string.
func collectBlockStringLines(tokens []token, i int) ([]blockLine, int) {
	return collectBlockStringLinesWithIndent(tokens, i, -1)
}

// collectBlockStringLinesWithIndent gathers continuation lines with an indent constraint.
// If baseIndent >= 0, only collect lines with indent > baseIndent.
func collectBlockStringLinesWithIndent(tokens []token, i int, baseIndent int) ([]blockLine, int) {
	var lines []blockLine

	for i < len(tokens) && (tokens[i].typ == tokenText || tokens[i].typ == tokenBreak) {
		if tokens[i].typ == tokenBreak {
			lines = append(lines, blockLine{isBreak: true})
		} else {
			// If we have a base indent constraint, stop when we see a line at or below that indent
			if baseIndent >= 0 && tokens[i].indent <= baseIndent {
				break
			}
			lines = append(lines, blockLine{indent: tokens[i].indent, text: tokens[i].text})
		}
		i++
	}

	return lines, i
}

// normalizeBlockIndent strips the minimum indentation from block lines.
func normalizeBlockIndent(contLines []blockLine) []string {
	// Find minimum indent among non-break lines
	minIndent := int(^uint(0) >> 1) // MaxInt
	for _, cl := range contLines {
		if !cl.isBreak && cl.indent < minIndent {
			minIndent = cl.indent
		}
	}
	if minIndent == int(^uint(0)>>1) {
		minIndent = 0
	}

	// Build lines with relative indentation
	var lines []string
	for _, cl := range contLines {
		if cl.isBreak {
			lines = append(lines, "")
		} else {
			extra := cl.indent - minIndent
			prefix := ""
			if extra > 0 {
				prefix = strings.Repeat(" ", extra)
			}
			lines = append(lines, prefix+cl.text)
		}
	}

	return lines
}

// buildBlockStringResult constructs the final block string.
// At root/array level: adds leading newline when backtick was alone on its line.
// In property context: no leading newline.
// Empty lines in the middle are preserved as newlines.
// Trailing empty lines collapse to a single trailing newline.
func buildBlockStringResult(firstLine string, lines []string, inPropertyContext bool) string {
	// Trim trailing empty lines (they collapse to single trailing newline)
	trimmed := trimTrailingEmpty(lines)

	// Leading newline only when backtick alone and NOT in property context
	leadingNewline := firstLine == "" && len(trimmed) > 0 && !inPropertyContext

	var body string
	if leadingNewline {
		body = "\n"
	}
	body += strings.Join(trimmed, "\n")
	if len(trimmed) > 0 {
		body += "\n"
	}

	return body
}

// ============================================================================
// Inline Array Parsing
// ============================================================================

// parseInlineArrayValue parses an inline array from a text token.
func parseInlineArrayValue(s string, t token, i int, ctx *parseContext) (any, int, error) {
	if !strings.Contains(s, "]") {
		return nil, 0, fmt.Errorf("Unexpected newline in inline array%s", locSuffix(ctx, t.lineNum, t.col))
	}
	arr, err := parseInlineArrayStrict(s, ctx, t.lineNum, t.col)
	if err != nil {
		return nil, 0, err
	}
	return arr, i + 1, nil
}

func parseInlineObjectValue(s string, t token, i int, ctx *parseContext) (any, int, error) {
	if !strings.Contains(s, "}") {
		return nil, 0, fmt.Errorf("Unexpected newline in inline object%s", locSuffix(ctx, t.lineNum, t.col))
	}
	obj, err := parseInlineObjectStrict(s, ctx, t.lineNum, t.col)
	if err != nil {
		return nil, 0, err
	}
	return obj, i + 1, nil
}

// parseInlineArrayStrict parses an inline array with strict whitespace validation.
func parseInlineArrayStrict(s string, ctx *parseContext, lineNum, col int) ([]any, error) {
	s = strings.TrimSpace(s)
	if !strings.HasPrefix(s, "[") {
		return nil, fmt.Errorf("Expected array%s", locSuffix(ctx, lineNum, col))
	}
	if !strings.HasSuffix(s, "]") {
		return nil, fmt.Errorf("Unterminated inline array%s", locSuffix(ctx, lineNum, col))
	}
	if s == "[]" {
		return []any{}, nil
	}

	// Validate whitespace
	if err := validateInlineSyntax(s, ctx, lineNum, col, '[', ']'); err != nil {
		return nil, err
	}

	inner := strings.TrimSpace(s[1 : len(s)-1])
	if inner == "" {
		return []any{}, nil
	}

	var result []any
	remaining := inner
	offset := 1 // Start after '['

	for len(remaining) > 0 {
		remaining = strings.TrimLeft(remaining, " ")

		value, consumed, err := parseInlineValueStrict(remaining, ctx, lineNum, col+offset)
		if err != nil {
			return nil, err
		}

		result = append(result, value)
		remaining = remaining[consumed:]
		offset += consumed
		remaining = strings.TrimLeft(remaining, " ")

		// Skip comma
		if strings.HasPrefix(remaining, ",") {
			remaining = remaining[1:]
			offset++
		}
	}

	return result, nil
}

// validateInlineSyntax validates whitespace in inline arrays/objects.
// Checks for:
// - No tabs anywhere
// - No space after opening bracket/brace
// - No space before closing bracket/brace
// - No space before comma
// - Exactly one space after comma (unless followed by closing bracket/brace)
func validateInlineSyntax(s string, ctx *parseContext, lineNum, col int, openChar, closeChar rune) error {
	runes := []rune(s)

	// Check boundary conditions first (like JS implementation)
	if len(runes) >= 2 && runes[0] == openChar && runes[1] == ' ' {
		return fmt.Errorf("Unexpected space after \"%c\"%s", openChar, locSuffix(ctx, lineNum, col+1))
	}
	if len(runes) >= 2 && runes[len(runes)-1] == closeChar && runes[len(runes)-2] == ' ' {
		return fmt.Errorf("Unexpected space before \"%c\"%s", closeChar, locSuffix(ctx, lineNum, col+len(runes)-2))
	}

	inSingle := false
	inDouble := false
	escape := false
	depth := 0

	for i, ch := range runes {
		if escape {
			escape = false
			continue
		}
		if inSingle {
			if ch == '\\' {
				escape = true
			} else if ch == '\'' {
				inSingle = false
			}
			continue
		}
		if inDouble {
			if ch == '\\' {
				escape = true
			} else if ch == '"' {
				inDouble = false
			}
			continue
		}
		// Check for tabs (outside of strings)
		if ch == '\t' {
			return fmt.Errorf("Tab not allowed (use spaces)%s", locSuffix(ctx, lineNum, col+i))
		}
		if ch == '\'' {
			inSingle = true
			continue
		}
		if ch == '"' {
			inDouble = true
			continue
		}
		if ch == openChar {
			depth++
			// Check nested opening brackets (not the first one, which is already checked)
			if i > 0 && i+1 < len(runes) && runes[i+1] == ' ' {
				return fmt.Errorf("Unexpected space after \"%c\"%s", openChar, locSuffix(ctx, lineNum, col+i+1))
			}
			continue
		}
		if ch == closeChar {
			// Check nested closing brackets (not the last one, which is already checked)
			if i < len(runes)-1 && i > 0 && runes[i-1] == ' ' {
				return fmt.Errorf("Unexpected space before \"%c\"%s", closeChar, locSuffix(ctx, lineNum, col+i-1))
			}
			if depth > 0 {
				depth--
			}
			continue
		}
		if ch == ',' {
			if i > 0 && runes[i-1] == ' ' {
				return fmt.Errorf("Unexpected space before \",\"%s", locSuffix(ctx, lineNum, col+i-1))
			}
			// Check for tab after comma (before checking for space)
			if i+1 < len(runes) && runes[i+1] == '\t' {
				return fmt.Errorf("Tab not allowed (use spaces)%s", locSuffix(ctx, lineNum, col+i+1))
			}
			// Check for space after comma
			if i+1 < len(runes) && runes[i+1] != ' ' && runes[i+1] != closeChar {
				return fmt.Errorf("Expected space after \",\"%s", locSuffix(ctx, lineNum, col+i))
			}
			// Check for double space after comma
			if i+2 < len(runes) && runes[i+1] == ' ' && runes[i+2] == ' ' {
				return fmt.Errorf("Unexpected space after \",\"%s", locSuffix(ctx, lineNum, col+i+2))
			}
			continue
		}
	}
	return nil
}

// parseInlineValueStrict parses a single value with strict validation.
func parseInlineValueStrict(s string, ctx *parseContext, lineNum, col int) (any, int, error) {
	if strings.HasPrefix(s, "[") {
		end := findMatchingBracket(s)
		if end < 0 {
			return nil, 0, fmt.Errorf("Unterminated inline array%s", locSuffix(ctx, lineNum, col))
		}
		arr, err := parseInlineArrayStrict(s[:end+1], ctx, lineNum, col)
		return arr, end + 1, err
	}

	if strings.HasPrefix(s, "{") {
		end := findMatchingBrace(s)
		if end < 0 {
			return nil, 0, fmt.Errorf("Unterminated inline object%s", locSuffix(ctx, lineNum, col))
		}
		obj, err := parseInlineObjectStrict(s[:end+1], ctx, lineNum, col)
		return obj, end + 1, err
	}

	if strings.HasPrefix(s, "<") {
		end := strings.Index(s, ">")
		if end < 0 {
			return nil, 0, fmt.Errorf("Unclosed angle bracket%s", locSuffix(ctx, lineNum, col))
		}
		bytes, err := parseAngleBytesStrict(s[:end+1], ctx, lineNum, col)
		if err != nil {
			return nil, 0, err
		}
		return bytes, end + 1, nil
	}

	if strings.HasPrefix(s, "\"") {
		str, consumed, err := parseInlineString(s)
		if err != nil {
			return nil, 0, fmt.Errorf("%s%s", err.Error(), locSuffix(ctx, lineNum, col))
		}
		return str, consumed, nil
	}

	// Single-quoted strings
	if strings.HasPrefix(s, "'") {
		str, consumed, err := parseInlineSingleQuotedString(s)
		if err != nil {
			return nil, 0, fmt.Errorf("%s%s", err.Error(), locSuffix(ctx, lineNum, col))
		}
		return str, consumed, nil
	}

	if strings.HasPrefix(s, "true") {
		return true, 4, nil
	}

	if strings.HasPrefix(s, "false") {
		return false, 5, nil
	}

	if strings.HasPrefix(s, "null") {
		return nil, 4, nil
	}

	if strings.HasPrefix(s, "nan") {
		return math.NaN(), 3, nil
	}

	if strings.HasPrefix(s, "infinity") {
		return math.Inf(1), 8, nil
	}

	if strings.HasPrefix(s, "-infinity") {
		return math.Inf(-1), 9, nil
	}

	// Try number
	num, consumed, err := parseInlineNumberStrict(s, ctx, lineNum, col)
	if err != nil {
		return nil, 0, err
	}
	if consumed > 0 {
		return num, consumed, nil
	}

	// Bare words are not valid
	if len(s) > 0 {
		firstChar := string(s[0])
		return nil, 0, fmt.Errorf("Unexpected character \"%s\"%s", firstChar, locSuffix(ctx, lineNum, col))
	}

	return nil, 0, fmt.Errorf("Unexpected empty value%s", locSuffix(ctx, lineNum, col))
}

// parseInlineNumberStrict parses a number from inline context with validation.
func parseInlineNumberStrict(s string, ctx *parseContext, lineNum, col int) (any, int, error) {
	// Find the end of the number (up to comma, bracket, brace, or space)
	end := 0
	for i, ch := range s {
		if ch == ',' || ch == ']' || ch == '}' || ch == ' ' {
			break
		}
		end = i + 1
	}
	if end == 0 {
		return nil, 0, nil
	}

	numStr := s[:end]

	// Check if it looks like a number
	if len(numStr) == 0 {
		return nil, 0, nil
	}
	firstChar := numStr[0]
	if firstChar != '-' && (firstChar < '0' || firstChar > '9') {
		return nil, 0, nil
	}

	// Try integer
	if integerRe.MatchString(numStr) {
		n := new(big.Int)
		n.SetString(numStr, 10)
		return n, end, nil
	}

	// Try float
	if floatRe.MatchString(numStr) && numStr != "." && numStr != "-." {
		var f float64
		fmt.Sscanf(numStr, "%f", &f)
		return f, end, nil
	}

	return nil, 0, nil
}

// parseAngleBytesStrict parses angle bracket bytes with validation.
func parseAngleBytesStrict(s string, ctx *parseContext, lineNum, col int) ([]byte, error) {
	if !strings.HasPrefix(s, "<") || !strings.HasSuffix(s, ">") {
		return nil, fmt.Errorf("Invalid byte literal%s", locSuffix(ctx, lineNum, col))
	}
	if s == "<>" {
		return []byte{}, nil
	}

	// Check for space after <
	if len(s) > 1 && s[1] == ' ' {
		return nil, fmt.Errorf("Unexpected space after \"<\"%s", locSuffix(ctx, lineNum, col+1))
	}
	// Check for space before >
	if len(s) > 1 && s[len(s)-2] == ' ' {
		return nil, fmt.Errorf("Unexpected space before \">\"%s", locSuffix(ctx, lineNum, col+len(s)-2))
	}

	inner := s[1 : len(s)-1]

	// Check for uppercase hex digits before lowercasing
	for i, c := range inner {
		if isUppercaseHex(c) {
			return nil, fmt.Errorf("Uppercase hex digit (use lowercase)%s", locSuffix(ctx, lineNum, col+1+i))
		}
	}

	// Remove internal spaces (allowed for grouping)
	inner = strings.ReplaceAll(inner, " ", "")

	if len(inner)%2 != 0 {
		return nil, fmt.Errorf("Odd number of hex digits in byte literal%s", locSuffix(ctx, lineNum, col))
	}

	// Validate hex digits
	for _, c := range inner {
		if !isHexDigit(c) {
			return nil, fmt.Errorf("Invalid hex digit%s", locSuffix(ctx, lineNum, col))
		}
	}

	bytes, err := hex.DecodeString(inner)
	if err != nil {
		return nil, fmt.Errorf("Invalid hex%s", locSuffix(ctx, lineNum, col))
	}
	return bytes, nil
}

// findMatchingBracket finds the index of the closing bracket.
func findMatchingBracket(s string) int {
	depth := 0
	inString := false
	stringChar := rune(0)
	escape := false

	for i, c := range s {
		if escape {
			escape = false
			continue
		}
		if c == '\\' && inString {
			escape = true
			continue
		}
		if (c == '"' || c == '\'') && (!inString || c == stringChar) {
			if inString {
				inString = false
				stringChar = 0
			} else {
				inString = true
				stringChar = c
			}
			continue
		}
		if inString {
			continue
		}
		if c == '[' {
			depth++
		} else if c == ']' {
			depth--
			if depth == 0 {
				return i
			}
		}
	}
	return -1
}

// findMatchingBrace finds the index of the closing brace for an inline object.
func findMatchingBrace(s string) int {
	depth := 0
	inString := false
	stringChar := rune(0)
	escape := false

	for i, c := range s {
		if escape {
			escape = false
			continue
		}
		if c == '\\' && inString {
			escape = true
			continue
		}
		if (c == '"' || c == '\'') && (!inString || c == stringChar) {
			if inString {
				inString = false
				stringChar = 0
			} else {
				inString = true
				stringChar = c
			}
			continue
		}
		if inString {
			continue
		}
		if c == '{' {
			depth++
		} else if c == '}' {
			depth--
			if depth == 0 {
				return i
			}
		}
	}
	return -1
}

// parseInlineObjectStrict parses an inline object with strict whitespace validation.
func parseInlineObjectStrict(s string, ctx *parseContext, lineNum, col int) (map[string]any, error) {
	s = strings.TrimSpace(s)
	if !strings.HasPrefix(s, "{") {
		return nil, fmt.Errorf("Expected object%s", locSuffix(ctx, lineNum, col))
	}
	if !strings.HasSuffix(s, "}") {
		return nil, fmt.Errorf("Unterminated inline object%s", locSuffix(ctx, lineNum, col))
	}
	if s == "{}" {
		return map[string]any{}, nil
	}

	// Validate whitespace
	if err := validateInlineSyntax(s, ctx, lineNum, col, '{', '}'); err != nil {
		return nil, err
	}

	// Check for space before/after colon
	if err := validateColonWhitespace(s, ctx, lineNum, col); err != nil {
		return nil, err
	}

	inner := strings.TrimSpace(s[1 : len(s)-1])
	if inner == "" {
		return map[string]any{}, nil
	}

	result := make(map[string]any)
	remaining := inner
	offset := 1 // Start after '{'

	for len(remaining) > 0 {
		remaining = strings.TrimLeft(remaining, " ")

		// Parse key
		key, keyLen, err := parseInlineKeyStrict(remaining, ctx, lineNum, col+offset, col)
		if err != nil {
			return nil, err
		}
		remaining = remaining[keyLen:]
		offset += keyLen
		remaining = strings.TrimLeft(remaining, " ")

		// Expect colon
		if !strings.HasPrefix(remaining, ":") {
			return nil, fmt.Errorf("Expected colon after key%s", locSuffix(ctx, lineNum, col))
		}
		remaining = remaining[1:]
		offset++
		remaining = strings.TrimLeft(remaining, " ")

		// Parse value
		value, consumed, err := parseInlineValueStrict(remaining, ctx, lineNum, col+offset)
		if err != nil {
			return nil, err
		}

		result[key] = value
		remaining = remaining[consumed:]
		offset += consumed
		remaining = strings.TrimLeft(remaining, " ")

		// Skip comma
		if strings.HasPrefix(remaining, ",") {
			remaining = remaining[1:]
			offset++
		}
	}

	return result, nil
}

// validateColonWhitespace checks for invalid whitespace around colons in inline objects.
func validateColonWhitespace(s string, ctx *parseContext, lineNum, col int) error {
	runes := []rune(s)
	inSingle := false
	inDouble := false
	escape := false

	for i, ch := range runes {
		if escape {
			escape = false
			continue
		}
		if inSingle {
			if ch == '\\' {
				escape = true
			} else if ch == '\'' {
				inSingle = false
			}
			continue
		}
		if inDouble {
			if ch == '\\' {
				escape = true
			} else if ch == '"' {
				inDouble = false
			}
			continue
		}
		if ch == '\'' {
			inSingle = true
			continue
		}
		if ch == '"' {
			inDouble = true
			continue
		}
		if ch == ':' {
			// Check for space before colon
			if i > 0 && runes[i-1] == ' ' {
				return fmt.Errorf("Unexpected space before \":\"%s", locSuffix(ctx, lineNum, col+i-1))
			}
			// Check for space after colon (required unless followed by closing brace)
			if i+1 < len(runes) && runes[i+1] != ' ' && runes[i+1] != '}' {
				return fmt.Errorf("Expected space after \":\"%s", locSuffix(ctx, lineNum, col+i))
			}
		}
	}
	return nil
}

// parseInlineKeyStrict parses an object key with strict validation.
// braceCol is the column of the opening brace, used for "Invalid key" errors.
func parseInlineKeyStrict(s string, ctx *parseContext, lineNum, col, braceCol int) (string, int, error) {
	if strings.HasPrefix(s, "\"") {
		str, consumed, err := parseInlineString(s)
		if err != nil {
			return "", 0, fmt.Errorf("%s%s", err.Error(), locSuffix(ctx, lineNum, col))
		}
		return str, consumed, nil
	}
	if strings.HasPrefix(s, "'") {
		str, consumed, err := parseInlineSingleQuotedString(s)
		if err != nil {
			return "", 0, fmt.Errorf("%s%s", err.Error(), locSuffix(ctx, lineNum, col))
		}
		return str, consumed, nil
	}

	// Unquoted key: alphanumeric characters, underscores, and hyphens
	i := 0
	for i < len(s) && (isAlphanumeric(s[i]) || s[i] == '_' || s[i] == '-') {
		i++
	}
	if i == 0 {
		// Report at brace column for "Invalid key" (first char invalid)
		return "", 0, fmt.Errorf("Invalid key%s", locSuffix(ctx, lineNum, braceCol))
	}
	return s[:i], i, nil
}
func isAlphanumeric(c byte) bool {
	return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9')
}

// parseInlineSingleQuotedString parses a single-quoted string.
func parseInlineSingleQuotedString(s string) (string, int, error) {
	if !strings.HasPrefix(s, "'") {
		return "", 0, fmt.Errorf("expected single-quoted string")
	}

	var out strings.Builder
	escape := false

	for i := 1; i < len(s); i++ {
		c := s[i]

		if escape {
			switch c {
			case '\'', '\\':
				out.WriteByte(c)
			default:
				out.WriteByte('\\')
				out.WriteByte(c)
			}
			escape = false
			continue
		}

		if c == '\\' {
			escape = true
			continue
		}

		if c == '\'' {
			return out.String(), i + 1, nil
		}

		out.WriteByte(c)
	}

	return "", 0, fmt.Errorf("unterminated string")
}

// parseInlineString parses a double-quoted string in inline notation.
func parseInlineString(s string) (string, int, error) {
	if !strings.HasPrefix(s, "\"") {
		return "", 0, fmt.Errorf("expected string")
	}

	var out strings.Builder
	escape := false

	for i := 1; i < len(s); i++ {
		c := s[i]

		if escape {
			switch c {
			case '"', '\\', '/':
				out.WriteByte(c)
			case 'b':
				out.WriteByte('\b')
			case 'f':
				out.WriteByte('\f')
			case 'n':
				out.WriteByte('\n')
			case 'r':
				out.WriteByte('\r')
			case 't':
				out.WriteByte('\t')
			case 'u':
				if i+4 >= len(s) {
					return "", 0, fmt.Errorf("invalid unicode escape")
				}
				var code int
				fmt.Sscanf(s[i+1:i+5], "%x", &code)
				out.WriteRune(rune(code))
				i += 4
			default:
				out.WriteByte(c)
			}
			escape = false
			continue
		}

		if c == '\\' {
			escape = true
			continue
		}

		if c == '"' {
			return out.String(), i + 1, nil
		}

		out.WriteByte(c)
	}

	return "", 0, fmt.Errorf("unterminated string")
}

// ============================================================================
// Byte Array Parsing
// ============================================================================

// isUppercaseHex checks if r is an uppercase hex digit.
func isUppercaseHex(r rune) bool {
	return r >= 'A' && r <= 'F'
}

// parseAngleBytes parses an inline byte array: <hexdigits>
func parseAngleBytes(s string, ctx *parseContext, lineNum, col int) ([]byte, error) {
	if s == "<>" {
		return []byte{}, nil
	}

	// Check for unclosed angle bracket
	if len(s) < 2 || !strings.HasSuffix(s, ">") {
		return nil, fmt.Errorf("Unmatched angle bracket%s", locSuffix(ctx, lineNum, col))
	}

	inner := s[1 : len(s)-1]

	// Check for uppercase hex digits before lowercasing
	for i, c := range inner {
		if isUppercaseHex(c) {
			return nil, fmt.Errorf("Uppercase hex digit (use lowercase)%s", locSuffix(ctx, lineNum, col+1+i))
		}
	}

	hexStr := strings.ReplaceAll(inner, " ", "")

	if len(hexStr)%2 != 0 {
		return nil, fmt.Errorf("Odd number of hex digits in byte literal%s", locSuffix(ctx, lineNum, col))
	}

	// Validate hex digits
	for _, c := range hexStr {
		if !isHexDigit(c) {
			return nil, fmt.Errorf("Invalid hex digit%s", locSuffix(ctx, lineNum, col))
		}
	}

	return hex.DecodeString(hexStr)
}

// parseBlockBytes parses a block byte array starting with >
// The > leader must have hex or comment on the line (not empty).
func parseBlockBytes(tokens []token, i int, ctx *parseContext) ([]byte, int, error) {
	first := tokens[i]
	baseIndent := first.indent

	// Validate: > alone on a line is invalid
	if first.text == ">" {
		return nil, 0, fmt.Errorf("Expected hex or comment in hex block%s", locSuffix(ctx, first.lineNum, first.col))
	}

	// Extract hex from first line (after >)
	hexPart := first.text[1:]
	if strings.HasPrefix(first.text, "> ") {
		hexPart = first.text[2:]
	}
	hexPart = stripComment(hexPart)
	hexPart = strings.ReplaceAll(hexPart, " ", "")

	var hexStr strings.Builder
	hexStr.WriteString(strings.ToLower(hexPart))
	i++

	// Collect continuation lines
	for i < len(tokens) && tokens[i].typ == tokenText && tokens[i].indent > baseIndent {
		line := stripComment(tokens[i].text)
		line = strings.ReplaceAll(line, " ", "")
		hexStr.WriteString(strings.ToLower(line))
		i++
	}

	hexResult := hexStr.String()
	if len(hexResult)%2 != 0 {
		return nil, 0, fmt.Errorf("Odd number of hex digits in byte literal%s", locSuffix(ctx, first.lineNum, first.col))
	}

	result, err := hex.DecodeString(hexResult)
	if err != nil {
		return nil, 0, err
	}
	return result, i, nil
}

// parseBlockBytesFromKeyLine parses block bytes after a key: >
// In property context, > must be followed only by comment or newline (no hex on same line).
// valuePart is the part after the colon (e.g., ">" or "> # comment").
func parseBlockBytesFromKeyLine(tokens []token, i int, ctx *parseContext, keyIndent int, valuePart string) ([]byte, int, error) {
	startToken := tokens[i]

	// Validate: in property context, hex on same line is invalid
	afterLeader := valuePart
	if strings.HasPrefix(afterLeader, "> ") {
		afterLeader = afterLeader[2:]
	} else if strings.HasPrefix(afterLeader, ">") {
		afterLeader = afterLeader[1:]
	}
	afterComment := stripComment(afterLeader)
	afterComment = strings.ReplaceAll(afterComment, " ", "")
	if afterComment != "" {
		return nil, 0, fmt.Errorf("Expected newline after block leader in property%s", locSuffix(ctx, startToken.lineNum, startToken.col))
	}

	i++

	var hexStr strings.Builder
	for i < len(tokens) && tokens[i].typ == tokenText && tokens[i].indent > keyIndent {
		line := stripComment(tokens[i].text)
		line = strings.ReplaceAll(line, " ", "")
		hexStr.WriteString(strings.ToLower(line))
		i++
	}

	hexResult := hexStr.String()
	if len(hexResult)%2 != 0 {
		return nil, 0, fmt.Errorf("Odd number of hex digits in byte literal%s", locSuffix(ctx, startToken.lineNum, startToken.col))
	}

	result, err := hex.DecodeString(hexResult)
	if err != nil {
		return nil, 0, err
	}
	return result, i, nil
}

// stripComment removes a # comment from a line (not inside quotes).
func stripComment(line string) string {
	inDouble := false
	inSingle := false
	escape := false

	for i, c := range line {
		if escape {
			escape = false
			continue
		}
		if c == '\\' {
			escape = true
			continue
		}
		if c == '"' && !inSingle {
			inDouble = !inDouble
		} else if c == '\'' && !inDouble {
			inSingle = !inSingle
		} else if c == '#' && !inDouble && !inSingle {
			return strings.TrimRight(line[:i], " ")
		}
	}
	return line
}

// ============================================================================
// Multiline Array Parsing
// ============================================================================

var inlineListItemRe = regexp.MustCompile(`^-\s+`)

// parseMultilineArray parses a multiline array (list items with - prefix).
// minIndent specifies the minimum indent level for array items (-1 means no limit).
func parseMultilineArray(tokens []token, i int, ctx *parseContext, minIndent int) ([]any, int, error) {
	var arr []any

	for i < len(tokens) && tokens[i].typ == tokenStart && tokens[i].text == "- " {
		listIndent := tokens[i].indent
		// Stop if we encounter a list item at a lower indent than expected
		if minIndent >= 0 && listIndent < minIndent {
			break
		}
		i++

		// Skip breaks after list marker
		i = skipBreaks(tokens, i)
		if i >= len(tokens) {
			break
		}

		// Parse the array item
		value, nextI, err := parseArrayItem(tokens, i, listIndent, ctx)
		if err != nil {
			return nil, 0, err
		}
		arr = append(arr, value)
		i = nextI

		// Skip stops and breaks between items
		i = skipBreaksAndStops(tokens, i)
	}

	return arr, i, nil
}

// parseArrayItem parses a single array item.
func parseArrayItem(tokens []token, i, listIndent int, ctx *parseContext) (any, int, error) {
	next := tokens[i]

	// Nested array: empty text followed by list start
	if next.typ == tokenText && next.text == "" && i+1 < len(tokens) &&
		tokens[i+1].typ == tokenStart && tokens[i+1].text == "- " {
		return parseMultilineArray(tokens, i+1, ctx, -1)
	}

	// Nested array: direct list start
	if next.typ == tokenStart && next.text == "- " {
		return parseMultilineArray(tokens, i, ctx, -1)
	}

	// Inline nested list: "- value" as text
	if next.typ == tokenText && inlineListItemRe.MatchString(next.text) {
		return parseInlineNestedList(tokens, i, listIndent, ctx)
	}

	// Regular value (possibly an object with multiple properties)
	if next.typ == tokenText || next.typ == tokenStart {
		return parseArrayItemValue(tokens, i, listIndent, ctx)
	}

	return nil, i + 1, nil
}

// parseInlineNestedList parses inline nested list items like "- a" as text.
func parseInlineNestedList(tokens []token, i, listIndent int, ctx *parseContext) ([]any, int, error) {
	var group []any
	j := i

	// Collect inline items
	for j < len(tokens) && tokens[j].typ == tokenText && inlineListItemRe.MatchString(tokens[j].text) {
		text := tokens[j].text
		// Check for double space after dash (e.g., "-  a")
		if len(text) >= 3 && text[0] == '-' && text[1] == ' ' && text[2] == ' ' {
			return nil, 0, fmt.Errorf("Unexpected space after \"-\"%s", locSuffix(ctx, tokens[j].lineNum, tokens[j].col+2))
		}
		valStr := strings.TrimSpace(inlineListItemRe.ReplaceAllString(text, ""))
		// Recursively handle nested inline bullets
		// Column offset: token col + 2 for the "- " prefix we stripped
		val, err := parseNestedInlineBullet(valStr, ctx, tokens[j].lineNum, tokens[j].col+2)
		if err != nil {
			return nil, 0, err
		}
		group = append(group, val)
		j++
	}

	// Continue with nested start tokens at deeper indent
	for j < len(tokens) && tokens[j].typ == tokenStart && tokens[j].text == "- " && tokens[j].indent > listIndent {
		j++
		j = skipBreaks(tokens, j)
		if j >= len(tokens) {
			break
		}

		subVal, nextJ, err := parseValue(tokens, j, ctx)
		if err != nil {
			return nil, 0, err
		}
		group = append(group, subVal)
		j = nextJ
		j = skipStops(tokens, j)
	}

	return group, j, nil
}

// parseArrayItemValue parses a regular array item value.
// Handles objects that span multiple lines with properties at the same indent.
func parseArrayItemValue(tokens []token, i, listIndent int, ctx *parseContext) (any, int, error) {
	value, j, err := parseValue(tokens, i, ctx)
	if err != nil {
		return nil, 0, err
	}

	// If value is an object, check for additional properties at the same level
	if obj, isObj := value.(map[string]any); isObj {
		j = mergeAdditionalObjectProperties(tokens, j, listIndent, obj, ctx)
		value = obj
	}

	// Check for nested list items after this value
	k := skipBreaks(tokens, j)
	if k < len(tokens) {
		afterBreak := tokens[k]
		if afterBreak.typ == tokenStart && afterBreak.text == "- " && afterBreak.indent > listIndent {
			return collectNestedListGroup(tokens, k, listIndent, value, ctx)
		}
	}

	return value, j, nil
}

// mergeAdditionalObjectProperties merges additional properties into an object.
// Properties at indent > listIndent are part of the same array item object.
func mergeAdditionalObjectProperties(tokens []token, j, listIndent int, obj map[string]any, ctx *parseContext) int {
	for j < len(tokens) {
		j = skipBreaks(tokens, j)
		if j >= len(tokens) {
			break
		}

		t := tokens[j]
		if t.typ == tokenText && t.indent > listIndent && findColonOutsideQuotes(t.text) >= 0 {
			propVal, nextJ, err := parseValue(tokens, j, ctx)
			if err != nil {
				break
			}
			if propObj, ok := propVal.(map[string]any); ok {
				for k, v := range propObj {
					obj[k] = v
				}
			}
			j = nextJ
		} else {
			break
		}
	}
	return j
}

// collectNestedListGroup collects nested list items into a group.
func collectNestedListGroup(tokens []token, i, listIndent int, firstValue any, ctx *parseContext) ([]any, int, error) {
	group := []any{firstValue}

	for i < len(tokens) && tokens[i].typ == tokenStart && tokens[i].text == "- " && tokens[i].indent > listIndent {
		i++
		i = skipBreaks(tokens, i)
		if i >= len(tokens) {
			break
		}

		subVal, nextI, err := parseValue(tokens, i, ctx)
		if err != nil {
			return nil, 0, err
		}
		group = append(group, subVal)
		i = nextI
		i = skipStops(tokens, i)
	}

	return group, i, nil
}

// skipBreaks advances past break tokens.
func skipBreaks(tokens []token, i int) int {
	for i < len(tokens) && tokens[i].typ == tokenBreak {
		i++
	}
	return i
}

// skipStops advances past stop tokens.
func skipStops(tokens []token, i int) int {
	for i < len(tokens) && tokens[i].typ == tokenStop {
		i++
	}
	return i
}

// ============================================================================
// Object Parsing
// ============================================================================

// parseKeyValuePair parses a key:value pair from a text token.
func parseKeyValuePair(tokens []token, i, colonIdx int, ctx *parseContext) (any, int, error) {
	t := tokens[i]
	s := t.text

	keyRaw := strings.TrimSpace(s[:colonIdx])
	key := parseKeyName(keyRaw)
	valuePart := strings.TrimSpace(s[colonIdx+1:])

	// Calculate column for value part
	afterColon := s[colonIdx+1:]
	valueOffset := strings.Index(afterColon, valuePart)
	valueCol := t.col + colonIdx + 1
	if valueOffset >= 0 {
		valueCol += valueOffset
	}

	// Empty value part means nested content follows
	if valuePart == "" && len(key) > 0 {
		return parseObjectOrNamedArray(tokens, i, key, ctx)
	}

	// Block bytes
	if len(key) > 0 && isBlockBytesStart(valuePart) {
		bytes, j, err := parseBlockBytesFromKeyLine(tokens, i, ctx, t.indent, valuePart)
		if err != nil {
			return nil, 0, err
		}
		return map[string]any{key: bytes}, j, nil
	}

	// Inline value
	if len(key) > 0 {
		var value any
		if valuePart != "" {
			var err error
			value, err = parseScalar(valuePart, ctx, t.lineNum, valueCol)
			if err != nil {
				return nil, 0, err
			}
		}
		return map[string]any{key: value}, i + 1, nil
	}

	return nil, i + 1, nil
}

// isBlockBytesStart checks if valuePart starts block bytes.
func isBlockBytesStart(valuePart string) bool {
	trimmed := strings.TrimSpace(valuePart)
	return strings.HasPrefix(trimmed, ">")
}

// findColonOutsideQuotes finds the first colon not inside quotes.
func findColonOutsideQuotes(s string) int {
	inDouble := false
	inSingle := false

	for i, c := range s {
		if c == '"' && !inSingle {
			inDouble = !inDouble
		} else if c == '\'' && !inDouble {
			inSingle = !inSingle
		} else if c == ':' && !inDouble && !inSingle {
			return i
		}
	}
	return -1
}

// parseKeyName extracts the key name, handling quoted keys.
func parseKeyName(s string) string {
	s = strings.TrimSpace(s)

	// Double-quoted key
	if strings.HasPrefix(s, "\"") && strings.HasSuffix(s, "\"") && len(s) >= 2 {
		inner := s[1 : len(s)-1]
		inner = strings.ReplaceAll(inner, "\\\"", "\"")
		inner = strings.ReplaceAll(inner, "\\\\", "\\")
		return inner
	}

	// Single-quoted key
	if strings.HasPrefix(s, "'") && strings.HasSuffix(s, "'") && len(s) >= 2 {
		return s[1 : len(s)-1]
	}

	return s
}

// isPropertyBlockLeaderOnly checks if a value part is just a block leader (backtick or >)
// optionally followed by spaces and/or a comment.
func isPropertyBlockLeaderOnly(valuePart string, leader rune) bool {
	if len(valuePart) == 0 {
		return false
	}
	if rune(valuePart[0]) != leader {
		return false
	}
	if len(valuePart) == 1 {
		return true
	}
	// Skip spaces after leader
	i := 1
	for i < len(valuePart) && valuePart[i] == ' ' {
		i++
	}
	if i >= len(valuePart) {
		return true
	}
	// Only a comment is allowed after spaces
	return valuePart[i] == '#'
}

// validateUnquotedKey validates that an unquoted key contains only valid characters.
// Returns (isQuoted, error). If isQuoted is true, no validation is needed.
func validateUnquotedKey(s string, ctx *parseContext, lineNum, col int) error {
	s = strings.TrimSpace(s)

	// Quoted keys don't need character validation
	if (strings.HasPrefix(s, "\"") && strings.HasSuffix(s, "\"")) ||
		(strings.HasPrefix(s, "'") && strings.HasSuffix(s, "'")) {
		return nil
	}

	// Unquoted key: validate each character
	// Allowed: alphanumeric, underscore, hyphen
	for i, c := range s {
		isAlpha := (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')
		isDigit := c >= '0' && c <= '9'
		isUnderscore := c == '_'
		isHyphen := c == '-'
		if !isAlpha && !isDigit && !isUnderscore && !isHyphen {
			if i == 0 {
				return fmt.Errorf("Invalid key%s", locSuffix(ctx, lineNum, col))
			}
			return fmt.Errorf("Invalid key character%s", locSuffix(ctx, lineNum, col+i))
		}
	}
	return nil
}

// parseObjectOrNamedArray parses content after "key:" (no inline value).
func parseObjectOrNamedArray(tokens []token, i int, key string, ctx *parseContext) (any, int, error) {
	i++

	// Skip to next content
	i = skipBreaksAndStops(tokens, i)

	baseIndent := 0
	if i < len(tokens) {
		baseIndent = tokens[i].indent
	}

	if i >= len(tokens) {
		return map[string]any{key: nil}, i, nil
	}

	first := tokens[i]

	// Named array - pass baseIndent as minIndent so array stops at object's level
	if first.typ == tokenStart && first.text == "- " {
		arr, next, err := parseMultilineArray(tokens, i, ctx, baseIndent)
		if err != nil {
			return nil, 0, err
		}
		return map[string]any{key: arr}, next, nil
	}

	// Block bytes on next line - this is invalid in strict YAY
	// The > must be on the same line as the key
	if first.typ == tokenText && isBlockBytesStart(first.text) {
		return nil, 0, fmt.Errorf("Unexpected indent at %d:%d of <%s>", first.lineNum+1, 1, ctx.filename)
	}

	// Block string on next line - this is invalid in strict YAY
	// The backtick must be on the same line as the key
	if first.typ == tokenText && strings.TrimSpace(first.text) == "`" {
		return nil, 0, fmt.Errorf("Unexpected indent at %d:%d of <%s>", first.lineNum+1, 1, ctx.filename)
	}

	// Nested object
	obj, next, err := parseNestedObjectContent(tokens, i, baseIndent, ctx)
	if err != nil {
		return nil, 0, err
	}

	if len(obj) > 0 {
		return map[string]any{key: obj}, next, nil
	}
	return map[string]any{key: nil}, next, nil
}

// parseNestedObjectContent parses the content of a nested object.
func parseNestedObjectContent(tokens []token, i, baseIndent int, ctx *parseContext) (map[string]any, int, error) {
	obj := make(map[string]any)

	for i < len(tokens) {
		t := tokens[i]

		if t.typ == tokenStop || t.typ == tokenBreak {
			i++
			continue
		}

		if t.typ == tokenText {
			// Reject inline values on separate line (they look like keys starting with special chars)
			if len(t.text) > 0 && (t.text[0] == '{' || t.text[0] == '[' || t.text[0] == '<') {
				return nil, 0, fmt.Errorf("Unexpected indent at %d:%d of <%s>", t.lineNum+1, 1, ctx.filename)
			}

			colonIdx := findColonOutsideQuotes(t.text)
			if colonIdx < 0 {
				// Text without colon in nested object context is invalid
				return nil, 0, fmt.Errorf("Unexpected indent at %d:%d of <%s>", t.lineNum+1, 1, ctx.filename)
			}
			if t.indent < baseIndent {
				break
			}

			kRaw := strings.TrimSpace(t.text[:colonIdx])
			k := parseKeyName(kRaw)
			vPart := strings.TrimSpace(t.text[colonIdx+1:])

			if k == "" {
				i++
				continue
			}

			value, nextI, err := parseObjectPropertyValue(tokens, i, t, k, vPart, baseIndent, ctx)
			if err != nil {
				return nil, 0, err
			}
			obj[k] = value
			i = nextI
		} else {
			i++
		}
	}

	return obj, i, nil
}

// parseObjectPropertyValue parses the value of an object property.
func parseObjectPropertyValue(tokens []token, i int, t token, key, vPart string, baseIndent int, ctx *parseContext) (any, int, error) {
	// Empty object
	if vPart == "{}" {
		return map[string]any{}, i + 1, nil
	}

	// Block string in property context: backtick alone on line
	if strings.TrimSpace(vPart) == "`" {
		body, next, err := parseBlockStringWithIndent(tokens, i, "", true, t.indent)
		if err != nil {
			return nil, 0, err
		}
		return body, next, nil
	}

	// Block bytes in property context: > alone on line (or with comment)
	if strings.HasPrefix(vPart, ">") {
		bytes, next, err := parseBlockBytesFromKeyLine(tokens, i, ctx, t.indent, vPart)
		if err != nil {
			return nil, 0, err
		}
		return bytes, next, nil
	}

	// Inline value
	if vPart != "" {
		scalar, err := parseScalar(vPart, ctx, t.lineNum, t.col)
		if err != nil {
			return nil, 0, err
		}
		return scalar, i + 1, nil
	}

	// Nested content
	j := i + 1
	j = skipBreaksAndStops(tokens, j)

	if j >= len(tokens) {
		return nil, i + 1, nil
	}

	nextT := tokens[j]

	// Named array - pass baseIndent as minIndent so array stops at object's level
	if nextT.typ == tokenStart && nextT.text == "- " {
		arr, next, err := parseMultilineArray(tokens, j, ctx, baseIndent)
		if err != nil {
			return nil, 0, err
		}
		return arr, next, nil
	}

	// Block string
	if nextT.typ == tokenText && strings.TrimSpace(nextT.text) == "`" {
		body, next, err := parseBlockString(tokens, j, "", true)
		if err != nil {
			return nil, 0, err
		}
		return body, next, nil
	}

	// Nested object
	if nextT.typ == tokenText && nextT.indent > t.indent {
		nestedObj, next, err := parseNestedObjectContent(tokens, j, nextT.indent, ctx)
		if err != nil {
			return nil, 0, err
		}
		return nestedObj, next, nil
	}

	return nil, j, nil
}

// skipToNextKey advances past content to find the next sibling key.
func skipToNextKey(tokens []token, i, baseIndent int) int {
	for i < len(tokens) && tokens[i].typ != tokenStop && tokens[i].indent > baseIndent {
		i++
	}
	for i < len(tokens) && tokens[i].typ == tokenStop {
		i++
	}
	return i
}

// ============================================================================
// Root Object Parsing
// ============================================================================

// parseRootObject parses an object at the document root level.
func parseRootObject(tokens []token, i int, ctx *parseContext) (any, int, error) {
	obj := make(map[string]any)

	for i < len(tokens) {
		t := tokens[i]

		if t.typ == tokenStop || t.typ == tokenBreak {
			i++
			continue
		}

		if t.typ != tokenText || t.indent != 0 {
			i++
			continue
		}

		colonIdx := findColonOutsideQuotes(t.text)
		if colonIdx < 0 {
			i++
			continue
		}

		// Validate: no space before colon
		if colonIdx > 0 && t.text[colonIdx-1] == ' ' {
			return nil, 0, fmt.Errorf("Unexpected space before \":\" at %d:%d of <%s>", t.lineNum+1, t.col+colonIdx, ctx.filename)
		}

		kRaw := strings.TrimSpace(t.text[:colonIdx])

		// Validate key characters
		if err := validateUnquotedKey(kRaw, ctx, t.lineNum, t.col); err != nil {
			return nil, 0, err
		}

		k := parseKeyName(kRaw)

		// Validate: space after colon (if there's content)
		afterColon := t.text[colonIdx+1:]
		if len(afterColon) > 0 && afterColon[0] == '\t' {
			return nil, 0, fmt.Errorf("Tab not allowed (use spaces) at %d:%d of <%s>", t.lineNum+1, t.col+colonIdx+2, ctx.filename)
		}
		if len(afterColon) > 0 && afterColon[0] != ' ' {
			return nil, 0, fmt.Errorf("Expected space after \":\" at %d:%d of <%s>", t.lineNum+1, t.col+colonIdx+1, ctx.filename)
		}
		// Validate: no double space after colon
		if len(afterColon) > 1 && afterColon[0] == ' ' && afterColon[1] == ' ' {
			return nil, 0, fmt.Errorf("Unexpected space after \":\" at %d:%d of <%s>", t.lineNum+1, t.col+colonIdx+3, ctx.filename)
		}

		vPart := strings.TrimSpace(afterColon)
		// Calculate column of value part (colon + 1 for space + 1 for 1-based)
		vCol := t.col + colonIdx + 2

		value, nextI, err := parseRootObjectProperty(tokens, i, t, k, vPart, vCol, ctx)
		if err != nil {
			return nil, 0, err
		}
		obj[k] = value
		i = nextI
	}

	return obj, i, nil
}

// parseRootObjectProperty parses a single property in a root object.
func parseRootObjectProperty(tokens []token, i int, t token, key, vPart string, vCol int, ctx *parseContext) (any, int, error) {
	// Block bytes
	if isBlockBytesStart(vPart) {
		bytes, j, err := parseBlockBytesFromKeyLine(tokens, i, ctx, 0, vPart)
		if err != nil {
			return nil, 0, err
		}
		return bytes, j, nil
	}

	// Empty object
	if vPart == "{}" {
		return map[string]any{}, i + 1, nil
	}

	// Block string
	if strings.HasPrefix(vPart, "`") {
		// In property context, backtick must be alone (or followed only by spaces/comment)
		if !isPropertyBlockLeaderOnly(vPart, '`') {
			return nil, 0, fmt.Errorf("Expected newline after block leader in property")
		}
		return parseRootBlockString(tokens, i+1)
	}

	// Nested content
	if vPart == "" {
		return parseRootNestedContent(tokens, i, ctx)
	}

	// Inline scalar
	scalar, err := parseScalar(vPart, ctx, t.lineNum, vCol)
	if err != nil {
		return nil, 0, err
	}
	return scalar, i + 1, nil
}

// parseRootBlockString parses a block string in a root object property.
func parseRootBlockString(tokens []token, i int) (string, int, error) {
	i = skipBreaksAndStops(tokens, i)

	// Collect indented lines
	var lines []blockLine
	for i < len(tokens) && ((tokens[i].typ == tokenText && tokens[i].indent > 0) || tokens[i].typ == tokenBreak) {
		if tokens[i].typ == tokenBreak {
			lines = append(lines, blockLine{isBreak: true})
		} else {
			lines = append(lines, blockLine{indent: tokens[i].indent, text: tokens[i].text})
		}
		i++
	}

	// Normalize and build result
	normalized := normalizeBlockIndent(lines)
	trimmed := trimTrailingEmpty(normalized)

	body := strings.Join(trimmed, "\n")
	if len(trimmed) > 0 {
		body += "\n"
	}

	if body == "" {
		return "", 0, fmt.Errorf("Empty block string not allowed (use \"\" or \"\\n\" explicitly)")
	}

	return body, i, nil
}

// trimTrailingEmpty removes trailing empty lines.
func trimTrailingEmpty(lines []string) []string {
	end := len(lines)
	for end > 0 && lines[end-1] == "" {
		end--
	}
	return lines[:end]
}

// parseRootNestedContent parses nested content after "key:" at root level.
func parseRootNestedContent(tokens []token, i int, ctx *parseContext) (any, int, error) {
	t := tokens[i]
	colonIdx := findColonOutsideQuotes(t.text)

	j := i + 1
	j = skipBreaksAndStops(tokens, j)

	if j >= len(tokens) {
		// Empty property with no nested content is invalid
		return nil, 0, fmt.Errorf("Expected value after property%s", locSuffix(ctx, t.lineNum, t.col+colonIdx+1))
	}

	nextT := tokens[j]

	// Named array at root level - no indent constraint
	if nextT.typ == tokenStart && nextT.text == "- " {
		arr, next, err := parseMultilineArray(tokens, j, ctx, -1)
		if err != nil {
			return nil, 0, err
		}
		return arr, next, nil
	}

	// Concatenated quoted strings (multiple quoted strings on consecutive lines)
	if nextT.typ == tokenText && nextT.indent > 0 {
		trimmed := strings.TrimSpace(nextT.text)
		if (strings.HasPrefix(trimmed, "\"") && strings.HasSuffix(trimmed, "\"") && len(trimmed) >= 2) ||
			(strings.HasPrefix(trimmed, "'") && strings.HasSuffix(trimmed, "'") && len(trimmed) >= 2) {
			concatStr, next, err := parseConcatenatedStrings(tokens, j, nextT.indent, ctx)
			if err != nil {
				return nil, 0, err
			}
			if concatStr != nil {
				return concatStr, next, nil
			}
			// Single string on new line is invalid - fall through to error
			return nil, 0, fmt.Errorf("Unexpected indent%s", locSuffix(ctx, nextT.lineNum, 0))
		}
	}

	// Nested object
	if nextT.typ == tokenText && nextT.indent > 0 {
		nestedObj, next, err := parseNestedObjectContent(tokens, j, nextT.indent, ctx)
		if err != nil {
			return nil, 0, err
		}
		return nestedObj, next, nil
	}

	// Empty property with no nested content is invalid
	return nil, 0, fmt.Errorf("Expected value after property%s", locSuffix(ctx, t.lineNum, t.col+colonIdx+1))
}

// ============================================================================
// Concatenated Strings
// ============================================================================

// parseConcatenatedStrings parses multiple quoted strings on consecutive lines.
// Returns nil if there's only one string (single string on new line is invalid).
func parseConcatenatedStrings(tokens []token, i, baseIndent int, ctx *parseContext) (any, int, error) {
	var parts []string

	for i < len(tokens) {
		t := tokens[i]

		if t.typ == tokenBreak || t.typ == tokenStop {
			i++
			continue
		}

		if t.typ != tokenText || t.indent < baseIndent {
			break
		}

		trimmed := strings.TrimSpace(t.text)

		// Check if this line is a quoted string
		isDoubleQuoted := strings.HasPrefix(trimmed, "\"") && strings.HasSuffix(trimmed, "\"") && len(trimmed) >= 2
		isSingleQuoted := strings.HasPrefix(trimmed, "'") && strings.HasSuffix(trimmed, "'") && len(trimmed) >= 2

		if !isDoubleQuoted && !isSingleQuoted {
			break
		}

		// Parse the quoted string
		parsed, err := parseQuotedString(trimmed, ctx, t.lineNum, t.col)
		if err != nil {
			return nil, 0, err
		}
		parts = append(parts, parsed)
		i++
	}

	// Require at least 2 strings for concatenation
	// A single string on a new line is invalid (use inline syntax instead)
	if len(parts) < 2 {
		return nil, i, nil
	}

	return strings.Join(parts, ""), i, nil
}

// ============================================================================
// Scalar Parsing
// ============================================================================

// parseNestedInlineBullet recursively parses inline bullet values.
// If the text starts with "- ", it wraps the result in an array.
func parseNestedInlineBullet(text string, ctx *parseContext, lineNum, col int) (any, error) {
	if inlineListItemRe.MatchString(text) {
		// Check for double space after dash
		if len(text) >= 3 && text[0] == '-' && text[1] == ' ' && text[2] == ' ' {
			return nil, fmt.Errorf("Unexpected space after \"-\"%s", locSuffix(ctx, lineNum, col+2))
		}
		innerText := strings.TrimSpace(inlineListItemRe.ReplaceAllString(text, ""))
		innerVal, err := parseNestedInlineBullet(innerText, ctx, lineNum, col+2)
		if err != nil {
			return nil, err
		}
		return []any{innerVal}, nil
	}
	return parseScalar(text, ctx, lineNum, col)
}

// parseScalar parses a scalar value from a string.
func parseScalar(s string, ctx *parseContext, lineNum, col int) (any, error) {
	// Strip inline comments first
	s = stripComment(s)

	// Keywords
	if v, ok := parseKeyword(s); ok {
		return v, nil
	}

	// Numbers (with whitespace validation)
	if num, ok, err := parseNumberStrict(s, ctx, lineNum, col); err != nil {
		return nil, err
	} else if ok {
		return num, nil
	}

	// Double-quoted string
	if strings.HasPrefix(s, "\"") && strings.HasSuffix(s, "\"") {
		return parseQuotedString(s, ctx, lineNum, col)
	}

	// Single-quoted string
	if strings.HasPrefix(s, "'") && strings.HasSuffix(s, "'") {
		return s[1 : len(s)-1], nil
	}

	// Inline array
	if strings.HasPrefix(s, "[") {
		return parseInlineArrayStrict(s, ctx, lineNum, col)
	}

	// Inline object
	if strings.HasPrefix(s, "{") {
		return parseInlineObjectStrict(s, ctx, lineNum, col)
	}

	// Inline bytes
	if strings.HasPrefix(s, "<") {
		return parseAngleBytes(s, ctx, lineNum, col)
	}

	// Bare words are not valid - strings must be quoted
	if len(s) > 0 {
		firstChar := string(s[0])
		return nil, fmt.Errorf("Unexpected character \"%s\"%s", firstChar, locSuffix(ctx, lineNum, col))
	}

	return nil, fmt.Errorf("Unexpected empty value%s", locSuffix(ctx, lineNum, col))
}
