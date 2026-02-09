//! Phase 3: Value Parser
//!
//! The value parser recursively processes the token stream to build Rust values.
//! It handles:
//! - Scalars: null, booleans, numbers, strings
//! - Compounds: arrays (multiline and inline), objects
//! - Binary: byte arrays (inline and multiline)
//! - Block strings: multiline string literals

use crate::error::{ParseContext, ParseError, Result};
use crate::lexer::{Token, TokenType};
use crate::value::Value;
use num_bigint::BigInt;
use std::collections::HashMap;

/// Parse the root of a YAY document.
pub fn parse_root(tokens: &[Token], ctx: &ParseContext, had_comments: bool) -> Result<Value> {
    let i = skip_breaks_and_stops(tokens, 0);

    if i >= tokens.len() {
        // If there were comments but no actual content, error
        if had_comments {
            let suffix = match &ctx.filename {
                Some(name) => format!(" <{}>", name),
                None => String::new(),
            };
            return Err(ParseError::NoValueFound(suffix));
        }
        return Ok(Value::Null);
    }

    let t = &tokens[i];

    // Validate: No unexpected indent at root
    if t.typ == TokenType::Text && t.indent > 0 {
        return Err(ParseError::UnexpectedIndent(String::new()).with_location(ctx, t.line_num, 0));
    }

    // Detect root object (key: value at indent 0)
    // But not inline objects starting with {
    if t.typ == TokenType::Text && t.text.contains(':') && t.indent == 0 && !t.text.starts_with('{')
    {
        let (value, next) = parse_root_object(tokens, i, ctx)?;
        return ensure_at_end(value, tokens, next, ctx);
    }

    // Parse as single value
    let (value, next) = parse_value(tokens, i, ctx)?;
    ensure_at_end(value, tokens, next, ctx)
}

/// Verify no content remains after parsing.
fn ensure_at_end(value: Value, tokens: &[Token], i: usize, ctx: &ParseContext) -> Result<Value> {
    let j = skip_breaks_and_stops(tokens, i);
    if j < tokens.len() {
        let t = &tokens[j];
        return Err(ParseError::ExtraContent(String::new()).with_location(ctx, t.line_num, t.col));
    }
    Ok(value)
}

/// Skip past break and stop tokens.
fn skip_breaks_and_stops(tokens: &[Token], mut i: usize) -> usize {
    while i < tokens.len()
        && (tokens[i].typ == TokenType::Stop || tokens[i].typ == TokenType::Break)
    {
        i += 1;
    }
    i
}

/// Skip past break tokens only.
fn skip_breaks(tokens: &[Token], mut i: usize) -> usize {
    while i < tokens.len() && tokens[i].typ == TokenType::Break {
        i += 1;
    }
    i
}

/// Skip past stop tokens only.
fn skip_stops(tokens: &[Token], mut i: usize) -> usize {
    while i < tokens.len() && tokens[i].typ == TokenType::Stop {
        i += 1;
    }
    i
}

// ============================================================================
// Value Parsing
// ============================================================================

/// Parse a single value from the token stream.
fn parse_value(tokens: &[Token], i: usize, ctx: &ParseContext) -> Result<(Value, usize)> {
    if i >= tokens.len() {
        return Ok((Value::Null, i + 1));
    }

    let t = &tokens[i];

    // Validate text tokens
    if t.typ == TokenType::Text {
        validate_text_token(t, ctx)?;
    }

    // Handle block starts (list items)
    if t.typ == TokenType::Start && t.text == "- " {
        return parse_multiline_array(tokens, i, ctx);
    }

    // Handle text content
    if t.typ == TokenType::Text {
        return parse_text_value(tokens, i, ctx);
    }

    Ok((Value::Null, i + 1))
}

/// Check for invalid text patterns.
fn validate_text_token(t: &Token, ctx: &ParseContext) -> Result<()> {
    if t.text.starts_with(' ') {
        return Err(ParseError::LeadingSpace(String::new()).with_location(ctx, t.line_num, t.col));
    }
    if t.text == "$" {
        return Err(
            ParseError::UnexpectedChar('$', String::new()).with_location(ctx, t.line_num, t.col)
        );
    }
    Ok(())
}

/// Parse a text token into the appropriate value type.
fn parse_text_value(tokens: &[Token], i: usize, ctx: &ParseContext) -> Result<(Value, usize)> {
    let t = &tokens[i];
    let s = &t.text;

    // Try keywords
    if let Some(v) = parse_keyword(s) {
        return Ok((v, i + 1));
    }

    // Validate number spaces before parsing
    if looks_like_number(s) {
        if let Some(space_col) = validate_number_spaces(s) {
            return Err(
                ParseError::UnexpectedSpaceInNumber(String::new()).with_location(
                    ctx,
                    t.line_num,
                    t.col + space_col,
                ),
            );
        }
    }

    // Check for uppercase E in exponent (must be lowercase)
    if let Some(e_pos) = s.find('E') {
        // Only error if it looks like a number with exponent
        let before_e = &s[..e_pos];
        let trimmed: String = before_e.chars().filter(|c| *c != ' ').collect();
        if !trimmed.is_empty()
            && (trimmed
                .chars()
                .all(|c| c.is_ascii_digit() || c == '.' || c == '-'))
        {
            return Err(ParseError::UppercaseExponent(String::new()).with_location(
                ctx,
                t.line_num,
                t.col + e_pos,
            ));
        }
    }

    // Try numbers
    if let Some(num) = parse_number(s) {
        return Ok((num, i + 1));
    }

    // Try block string
    if is_block_string_start(s) {
        let first_line = extract_block_string_first_line(s);
        return parse_block_string(tokens, i, first_line);
    }

    // Try quoted string
    if is_quoted_string(s) {
        let str_val = parse_quoted_string(s, ctx, t.line_num, t.col)?;
        return Ok((Value::String(str_val), i + 1));
    }

    // Try inline array
    if s.starts_with('[') {
        return parse_inline_array_value(s, t, i, ctx);
    }

    // Try inline object
    if s.starts_with('{') {
        return parse_inline_object_value(s, t, i, ctx);
    }

    // Try inline bytes
    if s.starts_with('<') && s.contains('>') {
        let bytes = parse_angle_bytes(s, ctx, t.line_num, t.col)?;
        return Ok((Value::Bytes(bytes), i + 1));
    }

    // Unclosed angle bracket is invalid
    if s.starts_with('<') {
        return Err(ParseError::UnmatchedAngle(String::new()).with_location(ctx, t.line_num, t.col));
    }

    // Try block bytes (> hex)
    if s.starts_with('>') {
        return parse_block_bytes(tokens, i, ctx);
    }

    // Try key:value pair
    if let Some(colon_idx) = find_colon_outside_quotes(s) {
        return parse_key_value_pair(tokens, i, colon_idx, ctx);
    }

    // Fall back to scalar (strip inline comments first)
    let s_no_comment = strip_inline_comment(s);
    let scalar = parse_scalar(s_no_comment, ctx, t.line_num, t.col)?;
    Ok((scalar, i + 1))
}

// ============================================================================
// Comment Handling
// ============================================================================

/// Strip inline comments from a string.
/// Returns the value part (trimmed) without the comment.
fn strip_inline_comment(s: &str) -> &str {
    // Find # not inside quotes
    let mut in_double = false;
    let mut in_single = false;
    let mut escape = false;

    for (i, c) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            continue;
        }
        if c == '"' && !in_single {
            in_double = !in_double;
        } else if c == '\'' && !in_double {
            in_single = !in_single;
        } else if c == '#' && !in_double && !in_single {
            return s[..i].trim_end();
        }
    }
    s
}

// ============================================================================
// Keyword Parsing
// ============================================================================

/// Check if s is a YAY keyword and return its value.
fn parse_keyword(s: &str) -> Option<Value> {
    match s {
        "null" => Some(Value::Null),
        "true" => Some(Value::Bool(true)),
        "false" => Some(Value::Bool(false)),
        "nan" => Some(Value::Float(f64::NAN)),
        "infinity" => Some(Value::Float(f64::INFINITY)),
        "-infinity" => Some(Value::Float(f64::NEG_INFINITY)),
        _ => None,
    }
}

// ============================================================================
// Number Parsing
// ============================================================================

/// Attempt to parse s as a number.
/// Returns None if the string is not a valid number or uses uppercase E.
fn parse_number(s: &str) -> Option<Value> {
    // Reject uppercase E in exponent
    if s.contains('E') {
        return None;
    }

    // Remove spaces (allowed as digit grouping)
    let trimmed: String = s.chars().filter(|c| *c != ' ').collect();

    // Try integer: optional minus followed by digits
    if is_integer_pattern(&trimmed) {
        if let Ok(n) = trimmed.parse::<BigInt>() {
            return Some(Value::Integer(n));
        }
    }

    // Try float (must have decimal point)
    if is_float_pattern(&trimmed) && trimmed != "." && trimmed != "-." {
        if let Ok(f) = trimmed.parse::<f64>() {
            return Some(Value::Float(f));
        }
    }

    None
}

/// Validate spaces in a potential number string.
/// Spaces are only allowed between two digits.
/// Returns the column of an invalid space if found.
fn validate_number_spaces(s: &str) -> Option<usize> {
    let chars: Vec<char> = s.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        if ch == ' ' {
            let prev = if i > 0 { chars[i - 1] } else { '\0' };
            let next = if i + 1 < chars.len() {
                chars[i + 1]
            } else {
                '\0'
            };
            let is_digit_prev = prev.is_ascii_digit();
            let is_digit_next = next.is_ascii_digit();
            if !(is_digit_prev && is_digit_next) {
                return Some(i);
            }
        }
    }
    None
}

/// Check if a string looks like a number (for validation purposes).
fn looks_like_number(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    let mut has_digit = false;
    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_ascii_digit() {
            has_digit = true;
            continue;
        }
        if ch == ' ' {
            continue;
        }
        if ch == '.' {
            continue;
        }
        if ch == '-' && i == 0 {
            continue;
        }
        return false;
    }
    has_digit
}

/// Check if string matches integer pattern: -?\d+
fn is_integer_pattern(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// Check if string matches float pattern: -?\d*\.\d*([eE][+-]?\d+)?
/// Also matches exponent-only notation: -?\d+[eE][+-]?\d+
fn is_float_pattern(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);

    // Split off exponent part if present
    let (mantissa, exponent) = if let Some(e_pos) = s.to_lowercase().find('e') {
        (&s[..e_pos], Some(&s[e_pos + 1..]))
    } else {
        (s, None)
    };

    // Validate exponent if present
    if let Some(exp) = exponent {
        let exp = exp
            .strip_prefix('+')
            .or_else(|| exp.strip_prefix('-'))
            .unwrap_or(exp);
        if exp.is_empty() || !exp.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }

    // If there's an exponent but no decimal point, it's still a float (e.g., 1e10)
    if exponent.is_some() && !mantissa.contains('.') {
        return !mantissa.is_empty() && mantissa.chars().all(|c| c.is_ascii_digit());
    }

    // Must have decimal point for non-exponent floats
    if !mantissa.contains('.') {
        return false;
    }

    let parts: Vec<&str> = mantissa.splitn(2, '.').collect();
    if parts.len() != 2 {
        return false;
    }
    let before = parts[0];
    let after = parts[1];
    (before.is_empty() || before.chars().all(|c| c.is_ascii_digit()))
        && (after.is_empty() || after.chars().all(|c| c.is_ascii_digit()))
}

// ============================================================================
// String Parsing
// ============================================================================

/// Check if s starts a block string (backtick).
fn is_block_string_start(s: &str) -> bool {
    s == "`" || (s.starts_with('`') && s.len() >= 2 && s.as_bytes()[1] == b' ')
}

/// Extract the first line content from a block string start.
fn extract_block_string_first_line(s: &str) -> &str {
    if s.len() > 2 {
        &s[2..] // Content after "` "
    } else {
        "" // Backtick alone on line
    }
}

/// Check if s is a quoted string (double or single).
fn is_quoted_string(s: &str) -> bool {
    (s.starts_with('"') && s.len() > 1) || (s.starts_with('\'') && s.len() > 1)
}

/// Parse a quoted string value.
fn parse_quoted_string(s: &str, ctx: &ParseContext, line_num: usize, col: usize) -> Result<String> {
    if s.starts_with('"') {
        parse_double_quoted_string(s, ctx, line_num, col)
    } else if s.starts_with('\'') {
        // Single-quoted strings are literal (no escapes except \' and \\)
        if !s.ends_with('\'') || s.len() < 2 {
            return Err(
                ParseError::UnterminatedString(String::new()).with_location(ctx, line_num, col)
            );
        }
        Ok(parse_single_quoted_content(&s[1..s.len() - 1]))
    } else {
        Ok(s.to_string())
    }
}

/// Parse content of a single-quoted string, handling \' and \\ escapes.
fn parse_single_quoted_content(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('\'') => {
                    result.push('\'');
                    chars.next();
                }
                Some('\\') => {
                    result.push('\\');
                    chars.next();
                }
                _ => {
                    result.push('\\');
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Parse a JSON-style double-quoted string.
fn parse_double_quoted_string(
    s: &str,
    ctx: &ParseContext,
    line_num: usize,
    col: usize,
) -> Result<String> {
    if s.len() < 2 || !s.starts_with('"') {
        return Ok(s.to_string());
    }
    if !s.ends_with('"') {
        return Err(ParseError::UnterminatedString(String::new()).with_location(
            ctx,
            line_num,
            col + s.len() - 1,
        ));
    }

    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 1; // Skip opening quote

    while i < chars.len() - 1 {
        let ch = chars[i];

        if ch == '\\' {
            let (escaped, advance) = parse_escape_sequence(&chars, i, ctx, line_num, col)?;
            out.push_str(&escaped);
            i += advance + 1;
        } else if (ch as u32) < 0x20 {
            return Err(ParseError::BadCharInString(String::new()).with_location(
                ctx,
                line_num,
                col + i,
            ));
        } else {
            out.push(ch);
            i += 1;
        }
    }

    Ok(out)
}

/// Parse a backslash escape sequence.
fn parse_escape_sequence(
    chars: &[char],
    i: usize,
    ctx: &ParseContext,
    line_num: usize,
    col: usize,
) -> Result<(String, usize)> {
    if i + 1 >= chars.len() - 1 {
        return Err(ParseError::BadEscapedChar(String::new()).with_location(
            ctx,
            line_num,
            col + i + 1,
        ));
    }

    let esc = chars[i + 1];
    match esc {
        '"' => Ok(("\"".to_string(), 1)),
        '\\' => Ok(("\\".to_string(), 1)),
        '/' => Ok(("/".to_string(), 1)),
        'b' => Ok(("\x08".to_string(), 1)),
        'f' => Ok(("\x0C".to_string(), 1)),
        'n' => Ok(("\n".to_string(), 1)),
        'r' => Ok(("\r".to_string(), 1)),
        't' => Ok(("\t".to_string(), 1)),
        'u' => parse_unicode_escape(chars, i, ctx, line_num, col),
        _ => {
            Err(ParseError::BadEscapedChar(String::new()).with_location(ctx, line_num, col + i + 1))
        }
    }
}

/// Parse a \u{XXXXXX} escape sequence (variable-length with braces).
fn parse_unicode_escape(
    chars: &[char],
    i: usize,
    ctx: &ParseContext,
    line_num: usize,
    col: usize,
) -> Result<(String, usize)> {
    // Expect opening brace after \u
    let brace_start = i + 2;
    // Column of 'u' for "Bad escaped character" (old syntax)
    let u_col = col + i + 1;
    // Column of '{' for other unicode errors
    let brace_col = col + brace_start;

    if brace_start >= chars.len() - 1 || chars[brace_start] != '{' {
        // Old syntax like \u0041 - report as bad escaped character at 'u' position
        return Err(ParseError::BadEscapedChar(String::new()).with_location(ctx, line_num, u_col));
    }

    // Find closing brace
    let mut brace_end = brace_start + 1;
    while brace_end < chars.len() - 1 && chars[brace_end] != '}' {
        brace_end += 1;
    }

    if brace_end >= chars.len() - 1 || chars[brace_end] != '}' {
        return Err(
            ParseError::BadUnicodeEscape(String::new()).with_location(ctx, line_num, brace_col)
        );
    }

    let hex_start = brace_start + 1;
    let hex_len = brace_end - hex_start;

    // Empty braces \u{}
    if hex_len == 0 {
        return Err(
            ParseError::BadUnicodeEscape(String::new()).with_location(ctx, line_num, brace_col)
        );
    }

    // Too many hex digits (> 6)
    if hex_len > 6 {
        return Err(
            ParseError::BadUnicodeEscape(String::new()).with_location(ctx, line_num, brace_col)
        );
    }

    // Validate hex digits
    for &c in &chars[hex_start..brace_end] {
        if !c.is_ascii_hexdigit() {
            return Err(
                ParseError::BadUnicodeEscape(String::new()).with_location(ctx, line_num, brace_col)
            );
        }
    }

    // Parse code point
    let hex_str: String = chars[hex_start..brace_end].iter().collect();
    let code = u32::from_str_radix(&hex_str, 16).unwrap();

    // Reject surrogates
    if (0xD800..=0xDFFF).contains(&code) {
        return Err(
            ParseError::IllegalSurrogate(String::new()).with_location(ctx, line_num, brace_col)
        );
    }

    // Reject out of range
    if code > 0x10FFFF {
        return Err(
            ParseError::UnicodeOutOfRange(String::new()).with_location(ctx, line_num, brace_col)
        );
    }

    let ch = char::from_u32(code).unwrap();
    // advance = length of "u{...}" = brace_end - i
    Ok((ch.to_string(), brace_end - i))
}

// ============================================================================
// Block String Parsing
// ============================================================================

/// A line in a block string with its indent.
struct BlockLine {
    indent: usize,
    text: String,
    is_break: bool,
}

/// Parse a multiline block string.
fn parse_block_string(tokens: &[Token], mut i: usize, first_line: &str) -> Result<(Value, usize)> {
    let mut lines: Vec<String> = Vec::new();
    if !first_line.is_empty() {
        lines.push(first_line.to_string());
    }
    i += 1;

    // Collect continuation lines with their indentation
    let (continuation_lines, new_i) = collect_block_string_lines(tokens, i);
    i = new_i;

    // Normalize indentation
    lines.extend(normalize_block_indent(&continuation_lines));

    // Trim empty lines from start and end
    let trimmed = trim_empty_lines(&lines);

    // Build result with appropriate leading newline
    let result = build_block_string_result(first_line, &trimmed);
    if result.is_empty() {
        return Err(ParseError::Generic(
            "Empty block string not allowed (use \"\" or \"\\n\" explicitly)".to_string(),
        ));
    }
    Ok((Value::String(result), i))
}

/// Gather continuation lines for a block string.
fn collect_block_string_lines(tokens: &[Token], mut i: usize) -> (Vec<BlockLine>, usize) {
    let mut lines = Vec::new();

    while i < tokens.len()
        && (tokens[i].typ == TokenType::Text || tokens[i].typ == TokenType::Break)
    {
        if tokens[i].typ == TokenType::Break {
            lines.push(BlockLine {
                indent: 0,
                text: String::new(),
                is_break: true,
            });
        } else {
            lines.push(BlockLine {
                indent: tokens[i].indent,
                text: tokens[i].text.clone(),
                is_break: false,
            });
        }
        i += 1;
    }

    (lines, i)
}

/// Strip the minimum indentation from block lines.
fn normalize_block_indent(cont_lines: &[BlockLine]) -> Vec<String> {
    // Find minimum indent among non-break lines
    let min_indent = cont_lines
        .iter()
        .filter(|cl| !cl.is_break)
        .map(|cl| cl.indent)
        .min()
        .unwrap_or(0);

    // Build lines with relative indentation
    cont_lines
        .iter()
        .map(|cl| {
            if cl.is_break {
                String::new()
            } else {
                let extra = cl.indent.saturating_sub(min_indent);
                format!("{}{}", " ".repeat(extra), cl.text)
            }
        })
        .collect()
}

/// Remove leading and trailing empty lines.
fn trim_empty_lines(lines: &[String]) -> Vec<String> {
    let start = lines
        .iter()
        .position(|l| !l.is_empty())
        .unwrap_or(lines.len());
    let end = lines
        .iter()
        .rposition(|l| !l.is_empty())
        .map(|i| i + 1)
        .unwrap_or(start);
    lines[start..end].to_vec()
}

/// Construct the final block string.
fn build_block_string_result(first_line: &str, trimmed: &[String]) -> String {
    let leading_newline = first_line.is_empty() && !trimmed.is_empty();

    let mut body = String::new();
    if leading_newline {
        body.push('\n');
    }
    body.push_str(&trimmed.join("\n"));
    if !trimmed.is_empty() {
        body.push('\n');
    }

    body
}

// ============================================================================
// Inline Array Parsing
// ============================================================================

/// Parse an inline array from a text token.
fn parse_inline_array_value(
    s: &str,
    t: &Token,
    i: usize,
    ctx: &ParseContext,
) -> Result<(Value, usize)> {
    if !s.contains(']') {
        return Err(
            ParseError::UnexpectedNewline("array".to_string(), String::new())
                .with_location(ctx, t.line_num, t.col),
        );
    }
    let arr = parse_inline_array(s, ctx, t.line_num, t.col)?;
    Ok((Value::Array(arr), i + 1))
}

/// Validate inline array/object whitespace rules.
fn validate_inline_syntax(
    s: &str,
    ctx: &ParseContext,
    line_num: usize,
    col: usize,
    open_char: char,
    close_char: char,
) -> Result<()> {
    let chars: Vec<char> = s.chars().collect();
    let mut in_single = false;
    let mut in_double = false;
    let mut escape = false;
    let mut depth = 0;

    // First pass: check for tabs (highest priority)
    for (i, &ch) in chars.iter().enumerate() {
        if ch == '\t' {
            return Err(ParseError::TabNotAllowed(String::new()).with_location(
                ctx,
                line_num,
                col + i,
            ));
        }
    }

    for (i, &ch) in chars.iter().enumerate() {
        if escape {
            escape = false;
            continue;
        }
        if in_single {
            if ch == '\\' {
                escape = true;
            } else if ch == '\'' {
                in_single = false;
            }
            continue;
        }
        if in_double {
            if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_double = false;
            }
            continue;
        }
        if ch == '\'' {
            in_single = true;
            continue;
        }
        if ch == '"' {
            in_double = true;
            continue;
        }
        if ch == open_char {
            depth += 1;
            if i + 1 < chars.len() && chars[i + 1] == ' ' {
                return Err(
                    ParseError::UnexpectedSpaceAfter(open_char.to_string(), String::new())
                        .with_location(ctx, line_num, col + i + 1),
                );
            }
            continue;
        }
        if ch == close_char {
            if i > 0 && chars[i - 1] == ' ' {
                return Err(ParseError::UnexpectedSpaceBefore(
                    close_char.to_string(),
                    String::new(),
                )
                .with_location(ctx, line_num, col + i - 1));
            }
            if depth > 0 {
                depth -= 1;
            }
            continue;
        }
        if ch == ',' {
            if i > 0 && chars[i - 1] == ' ' {
                return Err(
                    ParseError::UnexpectedSpaceBefore(",".to_string(), String::new())
                        .with_location(ctx, line_num, col + i - 1),
                );
            }
            if i + 1 < chars.len() && chars[i + 1] != ' ' && chars[i + 1] != close_char {
                // Lookahead to check if next closing bracket has space before it
                let mut lookahead_depth = depth;
                let mut in_s = false;
                let mut in_d = false;
                let mut esc = false;
                let mut next_is_closing_with_space = false;
                for j in (i + 1)..chars.len() {
                    let cj = chars[j];
                    if esc {
                        esc = false;
                        continue;
                    }
                    if in_s {
                        if cj == '\\' {
                            esc = true;
                        } else if cj == '\'' {
                            in_s = false;
                        }
                        continue;
                    }
                    if in_d {
                        if cj == '\\' {
                            esc = true;
                        } else if cj == '"' {
                            in_d = false;
                        }
                        continue;
                    }
                    if cj == '\'' {
                        in_s = true;
                        continue;
                    }
                    if cj == '"' {
                        in_d = true;
                        continue;
                    }
                    if cj == open_char {
                        lookahead_depth += 1;
                        continue;
                    }
                    if cj == close_char {
                        if lookahead_depth == depth {
                            next_is_closing_with_space = j > 0 && chars[j - 1] == ' ';
                            break;
                        }
                        if lookahead_depth > 0 {
                            lookahead_depth -= 1;
                        }
                        continue;
                    }
                    if cj == ',' && lookahead_depth == depth {
                        break;
                    }
                }
                if !next_is_closing_with_space {
                    return Err(
                        ParseError::ExpectedSpaceAfter(",".to_string(), String::new())
                            .with_location(ctx, line_num, col + i),
                    );
                }
            }
            if i + 2 < chars.len() && chars[i + 1] == ' ' && chars[i + 2] == ' ' {
                return Err(
                    ParseError::UnexpectedSpaceAfter(",".to_string(), String::new()).with_location(
                        ctx,
                        line_num,
                        col + i + 2,
                    ),
                );
            }
            continue;
        }
    }
    Ok(())
}

/// Parse an inline array in bracket notation.
fn parse_inline_array(
    s: &str,
    ctx: &ParseContext,
    line_num: usize,
    col: usize,
) -> Result<Vec<Value>> {
    let s = s.trim();
    if !s.starts_with('[') || !s.ends_with(']') {
        return Err(ParseError::UnmatchedBracket(String::new()).with_location(ctx, line_num, col));
    }

    // Validate whitespace
    validate_inline_syntax(s, ctx, line_num, col, '[', ']')?;

    let inner = s[1..s.len() - 1].trim();

    if inner.is_empty() {
        return Ok(Vec::new());
    }

    let mut result = Vec::new();
    let mut remaining = inner;

    while !remaining.is_empty() {
        remaining = remaining.trim_start();

        let (value, consumed) = parse_inline_value(remaining, ctx, line_num, col)?;
        result.push(value);
        remaining = &remaining[consumed..];
        remaining = remaining.trim_start();

        // Skip comma
        if remaining.starts_with(',') {
            remaining = &remaining[1..];
        }
    }

    Ok(result)
}

/// Parse an inline object from a text token.
fn parse_inline_object_value(
    s: &str,
    t: &Token,
    i: usize,
    ctx: &ParseContext,
) -> Result<(Value, usize)> {
    if !s.contains('}') {
        return Err(
            ParseError::UnexpectedNewline("object".to_string(), String::new())
                .with_location(ctx, t.line_num, t.col),
        );
    }
    let obj = parse_inline_object(s, ctx, t.line_num, t.col)?;
    Ok((Value::Object(obj), i + 1))
}

/// Parse an inline object in brace notation.
fn parse_inline_object(
    s: &str,
    ctx: &ParseContext,
    line_num: usize,
    col: usize,
) -> Result<HashMap<String, Value>> {
    let s = s.trim();
    if !s.starts_with('{') || !s.ends_with('}') {
        return Err(ParseError::UnmatchedBrace(String::new()).with_location(ctx, line_num, col));
    }

    // Validate whitespace
    validate_inline_syntax(s, ctx, line_num, col, '{', '}')?;

    let inner = s[1..s.len() - 1].trim();

    if inner.is_empty() {
        return Ok(HashMap::new());
    }

    let mut result = HashMap::new();
    let mut remaining = inner;

    while !remaining.is_empty() {
        remaining = remaining.trim_start();

        // Parse key
        let (key, key_len) = parse_inline_key(remaining, ctx, line_num, col)?;
        remaining = &remaining[key_len..];
        remaining = remaining.trim_start();

        // Expect colon
        if !remaining.starts_with(':') {
            return Err(ParseError::ExpectedColon(String::new()).with_location(ctx, line_num, col));
        }
        remaining = &remaining[1..];
        remaining = remaining.trim_start();

        // Parse value
        let (value, consumed) = parse_inline_value(remaining, ctx, line_num, col)?;
        result.insert(key, value);
        remaining = &remaining[consumed..];
        remaining = remaining.trim_start();

        // Skip comma
        if remaining.starts_with(',') {
            remaining = &remaining[1..];
        }
    }

    Ok(result)
}

/// Parse an object key (unquoted identifier or quoted string).
fn parse_inline_key(
    s: &str,
    ctx: &ParseContext,
    line_num: usize,
    col: usize,
) -> Result<(String, usize)> {
    if s.starts_with('"') {
        let (str_val, consumed) = parse_inline_string(s, ctx, line_num, col)?;
        return Ok((str_val, consumed));
    }
    if s.starts_with('\'') {
        let (str_val, consumed) = parse_inline_single_quoted_string(s)?;
        return Ok((str_val, consumed));
    }

    // Unquoted key: alphanumeric characters, underscores, and hyphens
    let i = s
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .count();
    if i == 0 {
        return Err(ParseError::InvalidKey(String::new()).with_location(ctx, line_num, col));
    }
    Ok((s[..i].to_string(), i))
}

/// Parse a single value from the start of an inline expression.
fn parse_inline_value(
    s: &str,
    ctx: &ParseContext,
    line_num: usize,
    col: usize,
) -> Result<(Value, usize)> {
    if s.starts_with('[') {
        let end = find_matching_bracket(s).ok_or_else(|| {
            ParseError::UnmatchedBracket(String::new()).with_location(ctx, line_num, col)
        })?;
        let arr = parse_inline_array(&s[..=end], ctx, line_num, col)?;
        return Ok((Value::Array(arr), end + 1));
    }

    if s.starts_with('{') {
        let end = find_matching_brace(s).ok_or_else(|| {
            ParseError::UnmatchedBrace(String::new()).with_location(ctx, line_num, col)
        })?;
        let obj = parse_inline_object(&s[..=end], ctx, line_num, col)?;
        return Ok((Value::Object(obj), end + 1));
    }

    if s.starts_with('<') {
        let end = s.find('>').ok_or_else(|| {
            ParseError::UnmatchedAngle(String::new()).with_location(ctx, line_num, col)
        })?;
        let bytes = parse_inline_byte_array(&s[1..end])?;
        return Ok((Value::Bytes(bytes), end + 1));
    }

    if s.starts_with('"') {
        let (str_val, consumed) = parse_inline_string(s, ctx, line_num, col)?;
        return Ok((Value::String(str_val), consumed));
    }

    if s.starts_with('\'') {
        let (str_val, consumed) = parse_inline_single_quoted_string(s)?;
        return Ok((Value::String(str_val), consumed));
    }

    if s.starts_with("true") {
        return Ok((Value::Bool(true), 4));
    }

    if s.starts_with("false") {
        return Ok((Value::Bool(false), 5));
    }

    if s.starts_with("null") {
        return Ok((Value::Null, 4));
    }

    if s.starts_with("nan") {
        return Ok((Value::Float(f64::NAN), 3));
    }

    if s.starts_with("infinity") {
        return Ok((Value::Float(f64::INFINITY), 8));
    }

    if s.starts_with("-infinity") {
        return Ok((Value::Float(f64::NEG_INFINITY), 9));
    }

    parse_inline_number(s)
}

/// Parse hex content from inside angle brackets.
fn parse_inline_byte_array(s: &str) -> Result<Vec<u8>> {
    // Check for uppercase hex digits before filtering whitespace
    if s.chars()
        .any(|c| c.is_ascii_uppercase() && c.is_ascii_hexdigit())
    {
        return Err(ParseError::UppercaseHex(String::new()));
    }

    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();

    if s.is_empty() {
        return Ok(Vec::new());
    }

    if !s.len().is_multiple_of(2) {
        return Err(ParseError::OddHexDigits(String::new()));
    }

    hex::decode(&s).map_err(|_| ParseError::InvalidHexDigit(String::new()))
}

/// Find the index of the closing bracket.
fn find_matching_bracket(s: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut string_char = '\0';
    let mut escape = false;

    for (i, c) in s.chars().enumerate() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' && in_string {
            escape = true;
            continue;
        }
        if (c == '"' || c == '\'') && (!in_string || c == string_char) {
            if in_string {
                in_string = false;
                string_char = '\0';
            } else {
                in_string = true;
                string_char = c;
            }
            continue;
        }
        if in_string {
            continue;
        }
        if c == '[' {
            depth += 1;
        } else if c == ']' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// Find the index of the closing brace.
fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut string_char = '\0';
    let mut escape = false;

    for (i, c) in s.chars().enumerate() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' && in_string {
            escape = true;
            continue;
        }
        if (c == '"' || c == '\'') && (!in_string || c == string_char) {
            if in_string {
                in_string = false;
                string_char = '\0';
            } else {
                in_string = true;
                string_char = c;
            }
            continue;
        }
        if in_string {
            continue;
        }
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// Parse a single-quoted string in inline notation.
fn parse_inline_single_quoted_string(s: &str) -> Result<(String, usize)> {
    if !s.starts_with('\'') {
        return Err(ParseError::UnterminatedString(String::new()));
    }

    let mut out = String::new();
    let mut escape = false;

    for (i, c) in s.chars().enumerate().skip(1) {
        if escape {
            match c {
                '\'' | '\\' => out.push(c),
                _ => {
                    out.push('\\');
                    out.push(c);
                }
            }
            escape = false;
            continue;
        }

        if c == '\\' {
            escape = true;
            continue;
        }

        if c == '\'' {
            return Ok((out, i + 1));
        }

        out.push(c);
    }

    Err(ParseError::UnterminatedString(String::new()))
}

/// Parse a double-quoted string in inline notation.
fn parse_inline_string(
    s: &str,
    ctx: &ParseContext,
    line_num: usize,
    col: usize,
) -> Result<(String, usize)> {
    if !s.starts_with('"') {
        return Err(ParseError::UnterminatedString(String::new()).with_location(ctx, line_num, col));
    }

    let mut out = String::new();
    let mut escape = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 1;

    while i < chars.len() {
        let c = chars[i];

        if escape {
            match c {
                '"' | '\\' | '/' => out.push(c),
                'b' => out.push('\x08'),
                'f' => out.push('\x0C'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'u' => {
                    // Expect \u{XXXXXX} format
                    if i + 2 >= chars.len() || chars[i + 1] != '{' {
                        return Err(ParseError::BadUnicodeEscape(String::new()).with_location(
                            ctx,
                            line_num,
                            col + i,
                        ));
                    }
                    // Find closing brace
                    let mut brace_end = i + 2;
                    while brace_end < chars.len() && chars[brace_end] != '}' {
                        brace_end += 1;
                    }
                    if brace_end >= chars.len() {
                        return Err(ParseError::BadUnicodeEscape(String::new()).with_location(
                            ctx,
                            line_num,
                            col + brace_end,
                        ));
                    }
                    let hex_start = i + 2;
                    if hex_start == brace_end {
                        return Err(ParseError::BadUnicodeEscape(String::new()).with_location(
                            ctx,
                            line_num,
                            col + hex_start,
                        ));
                    }
                    // Validate hex digits
                    for (j, &c) in chars.iter().enumerate().take(brace_end).skip(hex_start) {
                        if !c.is_ascii_hexdigit() {
                            return Err(ParseError::BadUnicodeEscape(String::new()).with_location(
                                ctx,
                                line_num,
                                col + j,
                            ));
                        }
                    }
                    let hex_str: String = chars[hex_start..brace_end].iter().collect();
                    let code = u32::from_str_radix(&hex_str, 16).map_err(|_| {
                        ParseError::BadUnicodeEscape(String::new()).with_location(
                            ctx,
                            line_num,
                            col + hex_start,
                        )
                    })?;
                    // Reject surrogates
                    if (0xD800..=0xDFFF).contains(&code) {
                        return Err(ParseError::IllegalSurrogate(String::new()).with_location(
                            ctx,
                            line_num,
                            col + hex_start,
                        ));
                    }
                    // Reject out of range
                    if code > 0x10FFFF {
                        return Err(ParseError::BadUnicodeEscape(String::new()).with_location(
                            ctx,
                            line_num,
                            col + hex_start,
                        ));
                    }
                    if let Some(ch) = char::from_u32(code) {
                        out.push(ch);
                    }
                    i = brace_end; // will be incremented by 1 at end of loop
                }
                _ => out.push(c),
            }
            escape = false;
            i += 1;
            continue;
        }

        if c == '\\' {
            escape = true;
            i += 1;
            continue;
        }

        if c == '"' {
            return Ok((out, i + 1));
        }

        out.push(c);
        i += 1;
    }

    Err(ParseError::UnterminatedString(String::new()).with_location(ctx, line_num, col))
}

/// Parse a number in inline notation.
fn parse_inline_number(s: &str) -> Result<(Value, usize)> {
    let mut i = 0;
    let chars: Vec<char> = s.chars().collect();
    let mut has_decimal = false;
    let mut has_exponent = false;

    // Optional minus
    if i < chars.len() && chars[i] == '-' {
        i += 1;
    }

    // Integer part
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }

    // Fractional part
    if i < chars.len() && chars[i] == '.' {
        has_decimal = true;
        i += 1;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }

    // Exponent
    if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
        // Reject uppercase E
        if chars[i] == 'E' {
            return Err(ParseError::UppercaseExponent(String::new()));
        }
        has_exponent = true;
        i += 1;
        if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
            i += 1;
        }
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }

    if i == 0 {
        return Err(ParseError::InvalidNumber(String::new()));
    }

    let num_str: String = chars[..i].iter().collect();

    // If no decimal point or exponent, return as big.Int
    if !has_decimal && !has_exponent {
        let n: BigInt = num_str
            .parse()
            .map_err(|_| ParseError::InvalidNumber(String::new()))?;
        return Ok((Value::Integer(n), i));
    }

    // Otherwise return as float64
    let f: f64 = num_str
        .parse()
        .map_err(|_| ParseError::InvalidNumber(String::new()))?;
    Ok((Value::Float(f), i))
}

// ============================================================================
// Byte Array Parsing
// ============================================================================

/// Parse an inline byte array: <hexdigits>
fn parse_angle_bytes(s: &str, ctx: &ParseContext, line_num: usize, col: usize) -> Result<Vec<u8>> {
    // Check for unclosed angle bracket
    if !s.ends_with('>') {
        return Err(ParseError::UnmatchedAngle(String::new()).with_location(ctx, line_num, col));
    }

    if s == "<>" {
        return Ok(Vec::new());
    }

    // Check for space before closing >
    let chars: Vec<char> = s.chars().collect();
    if chars.len() >= 2 && chars[chars.len() - 2] == ' ' && chars[chars.len() - 1] == '>' {
        return Err(
            ParseError::UnexpectedSpaceBefore(">".to_string(), String::new()).with_location(
                ctx,
                line_num,
                col + chars.len() - 2,
            ),
        );
    }

    let inner = &s[1..s.len() - 1];
    let hex_str: String = inner.chars().filter(|c| !c.is_whitespace()).collect();

    if !hex_str.len().is_multiple_of(2) {
        return Err(ParseError::OddHexDigits(String::new()).with_location(ctx, line_num, col));
    }

    // Check for uppercase hex digits
    for (i, c) in inner.chars().enumerate() {
        if c.is_ascii_uppercase() && c.is_ascii_hexdigit() {
            return Err(ParseError::UppercaseHex(String::new()).with_location(
                ctx,
                line_num,
                col + 1 + i,
            ));
        }
    }

    hex::decode(&hex_str)
        .map_err(|_| ParseError::InvalidHexDigit(String::new()).with_location(ctx, line_num, col))
}

// Note: parse_multiline_angle_bytes was removed as dead code.
// The "< hex" syntax (without closing ">") is invalid - inline byte arrays must be closed on the same line.

/// Remove a # comment from a line.
fn strip_comment(line: &str) -> &str {
    if let Some(idx) = line.find('#') {
        &line[..idx]
    } else {
        line
    }
}

/// Parse block bytes starting with > (e.g., "> b0b5" or "> " followed by indented hex)
fn parse_block_bytes(tokens: &[Token], mut i: usize, ctx: &ParseContext) -> Result<(Value, usize)> {
    let first = &tokens[i];
    let base_indent = first.indent;

    // Extract hex from first line (after >)
    let hex_part = if first.text.starts_with("> ") {
        &first.text[2..]
    } else {
        &first.text[1..]
    };

    // Check if there's any content (hex or comment) on the first line
    let trimmed = hex_part.trim();
    if trimmed.is_empty() && base_indent == 0 {
        // Lone > at root level with no content is an error
        return Err(ParseError::ExpectedHexInBlock);
    }

    let hex_part = strip_comment(hex_part);

    // Check for uppercase hex digits
    for (j, c) in hex_part.chars().enumerate() {
        if c.is_ascii_uppercase() && c.is_ascii_hexdigit() {
            return Err(ParseError::UppercaseHex(String::new()).with_location(
                ctx,
                first.line_num,
                first.col + 2 + j,
            ));
        }
    }

    let hex_part: String = hex_part.chars().filter(|c| !c.is_whitespace()).collect();

    let mut hex_str = hex_part;
    i += 1;

    // Collect continuation lines
    while i < tokens.len() && tokens[i].typ == TokenType::Text && tokens[i].indent > base_indent {
        let t = &tokens[i];
        let line = strip_comment(&t.text);

        // Check for uppercase hex digits
        for (j, c) in line.chars().enumerate() {
            if c.is_ascii_uppercase() && c.is_ascii_hexdigit() {
                return Err(ParseError::UppercaseHex(String::new()).with_location(
                    ctx,
                    t.line_num,
                    t.col + j,
                ));
            }
        }

        let line: String = line.chars().filter(|c| !c.is_whitespace()).collect();
        hex_str.push_str(&line);
        i += 1;
    }

    if !hex_str.len().is_multiple_of(2) {
        return Err(ParseError::OddHexDigits(String::new()).with_location(
            ctx,
            first.line_num,
            first.col,
        ));
    }

    let result = hex::decode(&hex_str).map_err(|_| {
        ParseError::InvalidHexDigit(String::new()).with_location(ctx, first.line_num, first.col)
    })?;
    Ok((Value::Bytes(result), i))
}

/// Parse block bytes from a property context (key: >)
fn parse_block_bytes_from_property(
    tokens: &[Token],
    mut i: usize,
    key_indent: usize,
    ctx: &ParseContext,
) -> Result<(Value, usize)> {
    let start_token = &tokens[i];
    i += 1;

    // Skip breaks
    while i < tokens.len() && tokens[i].typ == TokenType::Break {
        i += 1;
    }

    let mut hex_str = String::new();
    while i < tokens.len() && tokens[i].typ == TokenType::Text && tokens[i].indent > key_indent {
        let t = &tokens[i];
        let line = strip_comment(&t.text);

        // Check for uppercase hex digits
        for (j, c) in line.chars().enumerate() {
            if c.is_ascii_uppercase() && c.is_ascii_hexdigit() {
                return Err(ParseError::UppercaseHex(String::new()).with_location(
                    ctx,
                    t.line_num,
                    t.col + j,
                ));
            }
        }

        let line: String = line.chars().filter(|c| !c.is_whitespace()).collect();
        hex_str.push_str(&line);
        i += 1;
    }

    if hex_str.is_empty() {
        return Err(ParseError::ExpectedHexInBlock);
    }

    if !hex_str.len().is_multiple_of(2) {
        return Err(ParseError::OddHexDigits(String::new()).with_location(
            ctx,
            start_token.line_num,
            start_token.col,
        ));
    }

    let result = hex::decode(&hex_str).map_err(|_| {
        ParseError::InvalidHexDigit(String::new()).with_location(
            ctx,
            start_token.line_num,
            start_token.col,
        )
    })?;
    Ok((Value::Bytes(result), i))
}

/// Parse block string from a property context (key: `)
fn parse_block_string_from_property(
    tokens: &[Token],
    mut i: usize,
    key_indent: usize,
) -> Result<(Value, usize)> {
    i += 1;

    // Skip breaks
    while i < tokens.len() && tokens[i].typ == TokenType::Break {
        i += 1;
    }

    let mut lines: Vec<String> = Vec::new();
    let mut content_indent: Option<usize> = None;

    while i < tokens.len() {
        let t = &tokens[i];

        if t.typ == TokenType::Break {
            lines.push(String::new());
            i += 1;
            continue;
        }

        if t.typ != TokenType::Text || t.indent <= key_indent {
            break;
        }

        // Set content indent from first content line
        if content_indent.is_none() {
            content_indent = Some(t.indent);
        }

        let base = content_indent.unwrap();
        let extra_indent = t.indent.saturating_sub(base);
        let prefix = " ".repeat(extra_indent);
        lines.push(format!("{}{}", prefix, t.text));
        i += 1;
    }

    // Remove trailing empty lines
    while lines.last().map(|s| s.is_empty()).unwrap_or(false) {
        lines.pop();
    }

    // Join with newlines and add trailing newline
    let mut result = lines.join("\n");
    if !result.is_empty() {
        result.push('\n');
    }

    Ok((Value::String(result), i))
}

/// Parse concatenated quoted strings (multiple quoted strings on consecutive lines).
/// Each line must be a complete quoted string that gets concatenated together.
fn parse_concatenated_strings(
    tokens: &[Token],
    mut i: usize,
    base_indent: usize,
    ctx: &ParseContext,
) -> Result<(Value, usize)> {
    let mut parts: Vec<String> = Vec::new();
    let start_i = i;

    while i < tokens.len() {
        let t = &tokens[i];

        // Stop at breaks or tokens at lower indent
        if t.typ == TokenType::Break {
            i += 1;
            continue;
        }

        if t.typ == TokenType::Stop {
            i += 1;
            continue;
        }

        if t.typ != TokenType::Text || t.indent < base_indent {
            break;
        }

        let trimmed = t.text.trim();

        // Check if this line is a quoted string
        let is_double_quoted =
            trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2;
        let is_single_quoted =
            trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2;

        if !is_double_quoted && !is_single_quoted {
            // Not a quoted string, stop concatenation
            break;
        }

        // Parse the quoted string
        let parsed = parse_quoted_string(trimmed, ctx, t.line_num, t.col)?;
        parts.push(parsed);
        i += 1;
    }

    // Require at least 2 strings for concatenation
    // A single string on a new line is invalid (use inline syntax instead)
    if parts.len() < 2 {
        return Err(ParseError::UnexpectedIndent(String::new()).with_location(
            ctx,
            tokens[start_i].line_num,
            0,
        ));
    }

    // Concatenate all parts
    let result = parts.join("");
    Ok((Value::String(result), i))
}

// ============================================================================
// Multiline Array Parsing
// ============================================================================

/// Parse a multiline array (list items with - prefix).
fn parse_multiline_array(
    tokens: &[Token],
    mut i: usize,
    ctx: &ParseContext,
) -> Result<(Value, usize)> {
    let mut arr = Vec::new();

    // Track the indent of the first list item - all items must be at this indent
    let base_indent = if i < tokens.len() {
        tokens[i].indent
    } else {
        0
    };

    while i < tokens.len()
        && tokens[i].typ == TokenType::Start
        && tokens[i].text == "- "
        && tokens[i].indent == base_indent
    {
        let list_indent = tokens[i].indent;
        i += 1;

        // Skip breaks after list marker
        i = skip_breaks(tokens, i);
        if i >= tokens.len() {
            break;
        }

        // Parse the array item
        let (value, next_i) = parse_array_item(tokens, i, list_indent, ctx)?;
        arr.push(value);
        i = next_i;

        // Skip stops and breaks between items
        i = skip_breaks_and_stops(tokens, i);
    }

    Ok((Value::Array(arr), i))
}

/// Parse a single array item.
fn parse_array_item(
    tokens: &[Token],
    i: usize,
    list_indent: usize,
    ctx: &ParseContext,
) -> Result<(Value, usize)> {
    if i >= tokens.len() {
        return Ok((Value::Null, i));
    }

    let next = &tokens[i];

    // Nested array: empty text followed by list start
    if next.typ == TokenType::Text
        && next.text.is_empty()
        && i + 1 < tokens.len()
        && tokens[i + 1].typ == TokenType::Start
        && tokens[i + 1].text == "- "
    {
        return parse_multiline_array(tokens, i + 1, ctx);
    }

    // Nested array: direct list start
    if next.typ == TokenType::Start && next.text == "- " {
        return parse_multiline_array(tokens, i, ctx);
    }

    // Inline nested list: "- value" as text
    if next.typ == TokenType::Text && is_inline_list_item(&next.text) {
        return parse_inline_nested_list(tokens, i, list_indent, ctx);
    }

    // Regular value (possibly an object with multiple properties)
    if next.typ == TokenType::Text || next.typ == TokenType::Start {
        return parse_array_item_value(tokens, i, list_indent, ctx);
    }

    Ok((Value::Null, i + 1))
}

/// Check if text looks like an inline list item: "- value"
fn is_inline_list_item(text: &str) -> bool {
    text.starts_with("- ") || text == "-"
}

/// Recursively parse nested inline bullets like "- - - 'hello'".
/// Returns the parsed value (could be a nested array or a scalar).
fn parse_nested_inline_bullet(
    text: &str,
    ctx: &ParseContext,
    line_num: usize,
    col: usize,
) -> Result<Value> {
    // Check if the text itself is another inline bullet
    if is_inline_list_item(text) {
        let inner_text = text.strip_prefix("- ").unwrap_or(text).trim();
        let inner_value = parse_nested_inline_bullet(inner_text, ctx, line_num, col + 2)?;
        return Ok(Value::Array(vec![inner_value]));
    }
    // Otherwise, parse as a scalar (strip inline comments first)
    let text_no_comment = strip_inline_comment(text);
    parse_scalar(text_no_comment, ctx, line_num, col)
}

/// Parse inline nested list items like "- a" as text.
fn parse_inline_nested_list(
    tokens: &[Token],
    mut i: usize,
    list_indent: usize,
    ctx: &ParseContext,
) -> Result<(Value, usize)> {
    let mut group = Vec::new();

    // Collect inline items
    while i < tokens.len()
        && tokens[i].typ == TokenType::Text
        && is_inline_list_item(&tokens[i].text)
    {
        let t = &tokens[i];

        // Validate: no double space after -
        if t.text.starts_with("-  ") {
            return Err(
                ParseError::UnexpectedSpaceAfter("-".to_string(), String::new()).with_location(
                    ctx,
                    t.line_num,
                    t.col + 2,
                ),
            );
        }

        let val_str = t.text.strip_prefix("- ").unwrap_or(&t.text).trim();
        // Use parse_nested_inline_bullet to handle nested "- - value" patterns
        let value = parse_nested_inline_bullet(val_str, ctx, t.line_num, t.col + 2)?;
        group.push(value);
        i += 1;
    }

    // Continue with nested start tokens at deeper indent
    while i < tokens.len()
        && tokens[i].typ == TokenType::Start
        && tokens[i].text == "- "
        && tokens[i].indent > list_indent
    {
        i += 1;
        i = skip_breaks(tokens, i);
        if i >= tokens.len() {
            break;
        }

        let (sub_val, next_i) = parse_value(tokens, i, ctx)?;
        group.push(sub_val);
        i = next_i;
        i = skip_stops(tokens, i);
    }

    Ok((Value::Array(group), i))
}

/// Parse a regular array item value.
fn parse_array_item_value(
    tokens: &[Token],
    i: usize,
    list_indent: usize,
    ctx: &ParseContext,
) -> Result<(Value, usize)> {
    let (mut value, mut j) = parse_value(tokens, i, ctx)?;

    // If value is an object, check for additional properties at the same level
    if let Value::Object(ref mut obj) = value {
        j = merge_additional_object_properties(tokens, j, list_indent, obj, ctx)?;
    }

    // Check for nested list items after this value
    let k = skip_breaks(tokens, j);
    if k < tokens.len() {
        let after_break = &tokens[k];
        if after_break.typ == TokenType::Start
            && after_break.text == "- "
            && after_break.indent > list_indent
        {
            return collect_nested_list_group(tokens, k, list_indent, value, ctx);
        }
    }

    Ok((value, j))
}

/// Merge additional properties into an object.
fn merge_additional_object_properties(
    tokens: &[Token],
    mut j: usize,
    list_indent: usize,
    obj: &mut HashMap<String, Value>,
    ctx: &ParseContext,
) -> Result<usize> {
    loop {
        j = skip_breaks(tokens, j);
        if j >= tokens.len() {
            break;
        }

        let t = &tokens[j];
        if t.typ == TokenType::Text
            && t.indent > list_indent
            && find_colon_outside_quotes(&t.text).is_some()
        {
            let (prop_val, next_j) = parse_value(tokens, j, ctx)?;
            if let Value::Object(prop_obj) = prop_val {
                for (k, v) in prop_obj {
                    obj.insert(k, v);
                }
            }
            j = next_j;
        } else {
            break;
        }
    }
    Ok(j)
}

/// Collect nested list items into a group.
fn collect_nested_list_group(
    tokens: &[Token],
    mut i: usize,
    list_indent: usize,
    first_value: Value,
    ctx: &ParseContext,
) -> Result<(Value, usize)> {
    let mut group = vec![first_value];

    while i < tokens.len()
        && tokens[i].typ == TokenType::Start
        && tokens[i].text == "- "
        && tokens[i].indent > list_indent
    {
        i += 1;
        i = skip_breaks(tokens, i);
        if i >= tokens.len() {
            break;
        }

        let (sub_val, next_i) = parse_value(tokens, i, ctx)?;
        group.push(sub_val);
        i = next_i;
        i = skip_stops(tokens, i);
    }

    Ok((Value::Array(group), i))
}

// ============================================================================
// Object Parsing
// ============================================================================

/// Parse a key:value pair from a text token.
fn parse_key_value_pair(
    tokens: &[Token],
    i: usize,
    colon_idx: usize,
    ctx: &ParseContext,
) -> Result<(Value, usize)> {
    let t = &tokens[i];
    let s = &t.text;

    let key_raw = s[..colon_idx].trim();
    let key = parse_key_name(key_raw);

    // Calculate the column where the value part starts
    let after_colon = &s[colon_idx + 1..];
    let leading_spaces = after_colon.len() - after_colon.trim_start().len();
    let value_col = t.col + colon_idx + 1 + leading_spaces;
    let value_part = after_colon.trim();

    // Empty value part means nested content follows
    if value_part.is_empty() && !key.is_empty() {
        return parse_object_or_named_array(tokens, i, &key, ctx);
    }

    // Block bytes: "key: >" followed by indented hex lines
    if value_part == ">" && !key.is_empty() {
        let (bytes, next) = parse_block_bytes_from_property(tokens, i, t.indent, ctx)?;
        let mut obj = HashMap::new();
        obj.insert(key, bytes);
        return Ok((Value::Object(obj), next));
    }

    // Block string: "key: `" followed by indented content
    if value_part == "`" && !key.is_empty() {
        let (body, next) = parse_block_string_from_property(tokens, i, t.indent)?;
        let mut obj = HashMap::new();
        obj.insert(key, body);
        return Ok((Value::Object(obj), next));
    }

    // Note: "key: <" without closing ">" is invalid - inline byte arrays must be closed on the same line

    // Inline value (strip inline comments first)
    let value_part_no_comment = strip_inline_comment(value_part);
    if !key.is_empty() {
        let value = if !value_part_no_comment.is_empty() {
            parse_scalar(value_part_no_comment, ctx, t.line_num, value_col)?
        } else {
            Value::Null
        };
        let mut obj = HashMap::new();
        obj.insert(key, value);
        return Ok((Value::Object(obj), i + 1));
    }

    Ok((Value::Null, i + 1))
}

/// Find the first colon not inside quotes.
fn find_colon_outside_quotes(s: &str) -> Option<usize> {
    let mut in_double = false;
    let mut in_single = false;

    for (i, c) in s.chars().enumerate() {
        if c == '"' && !in_single {
            in_double = !in_double;
        } else if c == '\'' && !in_double {
            in_single = !in_single;
        } else if c == ':' && !in_double && !in_single {
            return Some(i);
        }
    }
    None
}

/// Extract the key name, handling quoted keys.
fn parse_key_name(s: &str) -> String {
    let s = s.trim();

    // Double-quoted key
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        let inner = &s[1..s.len() - 1];
        return inner.replace("\\\"", "\"").replace("\\\\", "\\");
    }

    // Single-quoted key
    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 {
        return s[1..s.len() - 1].to_string();
    }

    s.to_string()
}

/// Parse content after "key:" (no inline value).
fn parse_object_or_named_array(
    tokens: &[Token],
    mut i: usize,
    key: &str,
    ctx: &ParseContext,
) -> Result<(Value, usize)> {
    i += 1;

    // Skip to next content
    i = skip_breaks_and_stops(tokens, i);

    let base_indent = if i < tokens.len() {
        tokens[i].indent
    } else {
        0
    };

    if i >= tokens.len() {
        // Empty property with no nested content is invalid
        return Err(
            ParseError::ExpectedValueAfterProperty(String::new()).with_location(
                ctx,
                0,
                key.len() + 1,
            ),
        );
    }

    let first = &tokens[i];

    // Named array
    if first.typ == TokenType::Start && first.text == "- " {
        let (arr, next) = parse_multiline_array(tokens, i, ctx)?;
        let mut obj = HashMap::new();
        obj.insert(key.to_string(), arr);
        return Ok((Value::Object(obj), next));
    }

    // Note: "key: <" without closing ">" is invalid - inline byte arrays must be closed on the same line

    // Block string
    if first.typ == TokenType::Text && first.text.trim() == "`" {
        let (body, next) = parse_block_string(tokens, i, "")?;
        let mut obj = HashMap::new();
        obj.insert(key.to_string(), body);
        return Ok((Value::Object(obj), next));
    }

    // Nested object
    let (nested_obj, next) = parse_nested_object_content(tokens, i, base_indent, ctx)?;

    let mut obj = HashMap::new();
    if !nested_obj.is_empty() {
        obj.insert(key.to_string(), Value::Object(nested_obj));
    } else {
        // Empty property with no nested content is invalid
        return Err(
            ParseError::ExpectedValueAfterProperty(String::new()).with_location(
                ctx,
                0,
                key.len() + 1,
            ),
        );
    }
    Ok((Value::Object(obj), next))
}

/// Parse the content of a nested object.
fn parse_nested_object_content(
    tokens: &[Token],
    mut i: usize,
    base_indent: usize,
    ctx: &ParseContext,
) -> Result<(HashMap<String, Value>, usize)> {
    let mut obj = HashMap::new();

    while i < tokens.len() {
        let t = &tokens[i];

        if t.typ == TokenType::Stop || t.typ == TokenType::Break {
            i += 1;
            continue;
        }

        if t.typ == TokenType::Text {
            // Reject inline values on separate line (they look like keys starting with special chars)
            let first_char = t.text.chars().next();
            if matches!(first_char, Some('{') | Some('[') | Some('<')) {
                return Err(
                    ParseError::UnexpectedIndent(String::new()).with_location(ctx, t.line_num, 0)
                );
            }

            let colon_idx = match find_colon_outside_quotes(&t.text) {
                Some(idx) => idx,
                None => break,
            };
            if t.indent < base_indent {
                break;
            }

            let k_raw = t.text[..colon_idx].trim();
            let k = parse_key_name(k_raw);
            let v_part = t.text[colon_idx + 1..].trim();

            if k.is_empty() {
                i += 1;
                continue;
            }

            let (value, next_i) = parse_object_property_value(tokens, i, t, v_part, ctx)?;
            obj.insert(k, value);
            i = next_i;
        } else {
            i += 1;
        }
    }

    Ok((obj, i))
}

/// Parse the value of an object property.
fn parse_object_property_value(
    tokens: &[Token],
    i: usize,
    t: &Token,
    v_part: &str,
    ctx: &ParseContext,
) -> Result<(Value, usize)> {
    // Empty object
    if v_part == "{}" {
        return Ok((Value::Object(HashMap::new()), i + 1));
    }

    // Block bytes - either just ">" or "> # comment"
    if v_part == ">" || (v_part.starts_with("> ") && v_part[2..].trim().starts_with('#')) {
        let (bytes, next) = parse_block_bytes_from_property(tokens, i, t.indent, ctx)?;
        return Ok((bytes, next));
    }

    // Block string
    if v_part == "`" {
        let (body, next) = parse_block_string_from_property(tokens, i, t.indent)?;
        return Ok((body, next));
    }

    // Inline value (strip inline comments first)
    let v_part_no_comment = strip_inline_comment(v_part);
    if !v_part_no_comment.is_empty() {
        let scalar = parse_scalar(v_part_no_comment, ctx, t.line_num, t.col)?;
        return Ok((scalar, i + 1));
    }

    // Nested content
    let mut j = i + 1;
    j = skip_breaks_and_stops(tokens, j);

    if j >= tokens.len() {
        return Ok((Value::Null, i + 1));
    }

    let next_t = &tokens[j];

    // Named array
    if next_t.typ == TokenType::Start && next_t.text == "- " {
        let (arr, next) = parse_multiline_array(tokens, j, ctx)?;
        return Ok((arr, next));
    }

    // Block string
    if next_t.typ == TokenType::Text && next_t.text.trim() == "`" {
        let (body, next) = parse_block_string(tokens, j, "")?;
        return Ok((body, next));
    }

    // Concatenated quoted strings (multiple quoted strings on consecutive lines)
    if next_t.typ == TokenType::Text && next_t.indent > t.indent {
        let trimmed = next_t.text.trim();
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            let (concat_str, next) = parse_concatenated_strings(tokens, j, next_t.indent, ctx)?;
            return Ok((concat_str, next));
        }
    }

    // Nested object
    if next_t.typ == TokenType::Text && next_t.indent > t.indent {
        let (nested_obj, next) = parse_nested_object_content(tokens, j, next_t.indent, ctx)?;
        return Ok((Value::Object(nested_obj), next));
    }

    Ok((Value::Null, j))
}

// ============================================================================
// Root Object Parsing
// ============================================================================

/// Parse an object at the document root level.
fn parse_root_object(tokens: &[Token], mut i: usize, ctx: &ParseContext) -> Result<(Value, usize)> {
    let mut obj = HashMap::new();

    while i < tokens.len() {
        let t = &tokens[i];

        if t.typ == TokenType::Stop || t.typ == TokenType::Break {
            i += 1;
            continue;
        }

        if t.typ != TokenType::Text {
            i += 1;
            continue;
        }

        // Reject unexpected indented content at root level
        if t.indent != 0 {
            return Err(
                ParseError::UnexpectedIndent(String::new()).with_location(ctx, t.line_num, 0)
            );
        }

        let colon_idx = match find_colon_outside_quotes(&t.text) {
            Some(idx) => idx,
            None => {
                i += 1;
                continue;
            }
        };

        let k_raw = &t.text[..colon_idx];

        // Validate: no space before colon (for unquoted keys)
        if !k_raw.is_empty() && k_raw.ends_with(' ') {
            return Err(
                ParseError::UnexpectedSpaceBefore(":".to_string(), String::new()).with_location(
                    ctx,
                    t.line_num,
                    t.col + colon_idx - 1,
                ),
            );
        }

        // Validate key characters for unquoted keys
        let k_trimmed = k_raw.trim();
        if !k_trimmed.is_empty() && !k_trimmed.starts_with('"') && !k_trimmed.starts_with('\'') {
            // Check for invalid characters in unquoted key
            // Valid characters: alphanumeric, underscore, hyphen
            for (j, ch) in k_trimmed.chars().enumerate() {
                if ch == ' ' {
                    return Err(ParseError::InvalidKeyChar(String::new()).with_location(
                        ctx,
                        t.line_num,
                        t.col + j,
                    ));
                }
            }
        }

        let k = parse_key_name(k_trimmed);
        let after_colon = &t.text[colon_idx + 1..];

        // Validate: must have exactly one space after colon (if there's a value)
        if !after_colon.is_empty() {
            if !after_colon.starts_with(' ') {
                return Err(
                    ParseError::ExpectedSpaceAfter(":".to_string(), String::new()).with_location(
                        ctx,
                        t.line_num,
                        t.col + colon_idx,
                    ),
                );
            }
            if after_colon.len() > 1 && after_colon.starts_with("  ") {
                return Err(
                    ParseError::UnexpectedSpaceAfter(":".to_string(), String::new()).with_location(
                        ctx,
                        t.line_num,
                        t.col + colon_idx + 2,
                    ),
                );
            }
        }

        let leading_spaces = after_colon.len() - after_colon.trim_start().len();
        let value_col = t.col + colon_idx + 1 + leading_spaces;
        let v_part = after_colon.trim();

        let (value, next_i) = parse_root_object_property(tokens, i, t, v_part, value_col, ctx)?;
        obj.insert(k, value);
        i = next_i;
    }

    Ok((Value::Object(obj), i))
}

/// Parse a single property in a root object.
fn parse_root_object_property(
    tokens: &[Token],
    i: usize,
    t: &Token,
    v_part: &str,
    value_col: usize,
    ctx: &ParseContext,
) -> Result<(Value, usize)> {
    // Check for block leader followed by non-comment content on same line (invalid in property context)
    // Block string: ` followed by content (not a comment)
    if v_part.starts_with('`') && v_part.len() > 1 && v_part.chars().nth(1) == Some(' ') {
        let after_backtick = v_part[2..].trim();
        if !after_backtick.is_empty() && !after_backtick.starts_with('#') {
            return Err(ParseError::ExpectedNewlineAfterBlockLeader);
        }
    }
    // Block bytes: > followed by any non-comment content on same line (in property context)
    if v_part.starts_with('>') && v_part.len() > 1 && v_part.chars().nth(1) == Some(' ') {
        let after_angle = v_part[2..].trim();
        // Any content after > (except comments) is invalid in property context
        if !after_angle.is_empty() && !after_angle.starts_with('#') {
            return Err(ParseError::ExpectedNewlineAfterBlockLeader);
        }
    }

    // Note: "key: <" without closing ">" is invalid - inline byte arrays must be closed on the same line

    // Empty object
    if v_part == "{}" {
        return Ok((Value::Object(HashMap::new()), i + 1));
    }

    // Block string
    if v_part.trim() == "`" {
        let (body, next) = parse_root_block_string(tokens, i + 1)?;
        return Ok((body, next));
    }

    // Block bytes - either just ">" or "> # comment"
    if v_part == ">" || (v_part.starts_with("> ") && v_part[2..].trim().starts_with('#')) {
        let (bytes, next) = parse_block_bytes_from_property(tokens, i, t.indent, ctx)?;
        return Ok((bytes, next));
    }

    // Strip inline comments
    let v_part_no_comment = strip_inline_comment(v_part);

    // Nested content
    if v_part_no_comment.is_empty() {
        return parse_root_nested_content(tokens, i, ctx);
    }

    // Inline scalar
    let scalar = parse_scalar(v_part_no_comment, ctx, t.line_num, value_col)?;
    Ok((scalar, i + 1))
}

/// Parse a block string in a root object property.
fn parse_root_block_string(tokens: &[Token], mut i: usize) -> Result<(Value, usize)> {
    i = skip_breaks_and_stops(tokens, i);

    // Collect indented lines
    let mut lines: Vec<BlockLine> = Vec::new();
    while i < tokens.len()
        && ((tokens[i].typ == TokenType::Text && tokens[i].indent > 0)
            || tokens[i].typ == TokenType::Break)
    {
        if tokens[i].typ == TokenType::Break {
            lines.push(BlockLine {
                indent: 0,
                text: String::new(),
                is_break: true,
            });
        } else {
            lines.push(BlockLine {
                indent: tokens[i].indent,
                text: tokens[i].text.clone(),
                is_break: false,
            });
        }
        i += 1;
    }

    // Normalize and build result
    let normalized = normalize_block_indent(&lines);
    let trimmed = trim_trailing_empty(&normalized);

    let mut body = trimmed.join("\n");
    if !trimmed.is_empty() {
        body.push('\n');
    }

    if body.is_empty() {
        return Err(ParseError::Generic(
            "Empty block string not allowed (use \"\" or \"\\n\" explicitly)".to_string(),
        ));
    }

    Ok((Value::String(body), i))
}

/// Remove trailing empty lines.
fn trim_trailing_empty(lines: &[String]) -> Vec<String> {
    let end = lines
        .iter()
        .rposition(|l| !l.is_empty())
        .map(|i| i + 1)
        .unwrap_or(0);
    lines[..end].to_vec()
}

/// Parse nested content after "key:" at root level.
fn parse_root_nested_content(
    tokens: &[Token],
    i: usize,
    ctx: &ParseContext,
) -> Result<(Value, usize)> {
    let t = &tokens[i];
    let colon_idx = find_colon_outside_quotes(&t.text).unwrap_or(0);

    let mut j = i + 1;
    j = skip_breaks_and_stops(tokens, j);

    if j >= tokens.len() {
        // Empty property with no nested content is invalid
        return Err(
            ParseError::ExpectedValueAfterProperty(String::new()).with_location(
                ctx,
                t.line_num,
                t.col + colon_idx + 1,
            ),
        );
    }

    let next_t = &tokens[j];

    // Named array
    if next_t.typ == TokenType::Start && next_t.text == "- " {
        let (arr, next) = parse_multiline_array(tokens, j, ctx)?;
        return Ok((arr, next));
    }

    // Concatenated quoted strings (multiple quoted strings on consecutive lines)
    if next_t.typ == TokenType::Text && next_t.indent > 0 {
        let trimmed = next_t.text.trim();
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            let (concat_str, next) = parse_concatenated_strings(tokens, j, next_t.indent, ctx)?;
            return Ok((concat_str, next));
        }
    }

    // Nested object
    if next_t.typ == TokenType::Text && next_t.indent > 0 {
        let (nested_obj, next) = parse_nested_object_content(tokens, j, next_t.indent, ctx)?;
        return Ok((Value::Object(nested_obj), next));
    }

    // Empty property with no nested content is invalid
    Err(
        ParseError::ExpectedValueAfterProperty(String::new()).with_location(
            ctx,
            t.line_num,
            t.col + colon_idx + 1,
        ),
    )
}

// ============================================================================
// Scalar Parsing
// ============================================================================

/// Parse a scalar value from a string.
fn parse_scalar(s: &str, ctx: &ParseContext, line_num: usize, col: usize) -> Result<Value> {
    // Keywords
    if let Some(v) = parse_keyword(s) {
        return Ok(v);
    }

    // Check for uppercase E in exponent (must be lowercase)
    if let Some(e_pos) = s.find('E') {
        let before_e = &s[..e_pos];
        let trimmed: String = before_e.chars().filter(|c| *c != ' ').collect();
        if !trimmed.is_empty()
            && (trimmed
                .chars()
                .all(|c| c.is_ascii_digit() || c == '.' || c == '-'))
        {
            return Err(ParseError::UppercaseExponent(String::new()).with_location(
                ctx,
                line_num,
                col + e_pos,
            ));
        }
    }

    // Numbers
    if let Some(num) = parse_number(s) {
        return Ok(num);
    }

    // Double-quoted string
    if s.starts_with('"') && s.ends_with('"') {
        return Ok(Value::String(parse_quoted_string(s, ctx, line_num, col)?));
    }

    // Single-quoted string
    if s.starts_with('\'') && s.ends_with('\'') {
        return Ok(Value::String(parse_single_quoted_content(
            &s[1..s.len() - 1],
        )));
    }

    // Inline array
    if s.starts_with('[') {
        return Ok(Value::Array(parse_inline_array(s, ctx, line_num, col)?));
    }

    // Inline object
    if s.starts_with('{') {
        return Ok(Value::Object(parse_inline_object(s, ctx, line_num, col)?));
    }

    // Inline bytes
    if s.starts_with('<') {
        return Ok(Value::Bytes(parse_angle_bytes(s, ctx, line_num, col)?));
    }

    // Bare words are not valid - strings must be quoted
    let first_char = s.chars().next().unwrap_or('?');
    Err(ParseError::UnexpectedChar(first_char, String::new()).with_location(ctx, line_num, col))
}

// Add hex crate functionality inline since we can't add it as a dependency easily
mod hex {
    pub fn decode(s: &str) -> Result<Vec<u8>, ()> {
        if !s.len().is_multiple_of(2) {
            return Err(());
        }

        let mut result = Vec::with_capacity(s.len() / 2);
        let chars: Vec<char> = s.chars().collect();

        for i in (0..chars.len()).step_by(2) {
            let high = hex_digit(chars[i]).ok_or(())?;
            let low = hex_digit(chars[i + 1]).ok_or(())?;
            result.push((high << 4) | low);
        }

        Ok(result)
    }

    fn hex_digit(c: char) -> Option<u8> {
        match c {
            '0'..='9' => Some(c as u8 - b'0'),
            'a'..='f' => Some(c as u8 - b'a' + 10),
            'A'..='F' => Some(c as u8 - b'A' + 10),
            _ => None,
        }
    }
}

// Most parser functionality is tested via fixtures
// These unit tests cover internal helper functions
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_keyword() {
        assert_eq!(parse_keyword("null"), Some(Value::Null));
        assert_eq!(parse_keyword("true"), Some(Value::Bool(true)));
        assert_eq!(parse_keyword("false"), Some(Value::Bool(false)));
        assert!(parse_keyword("nan").unwrap().as_float().unwrap().is_nan());
    }

    #[test]
    fn test_parse_number() {
        assert_eq!(parse_number("42"), Some(Value::Integer(42.into())));
        assert_eq!(parse_number("-10"), Some(Value::Integer((-10).into())));
        assert_eq!(parse_number("1.5"), Some(Value::Float(1.5)));
        assert_eq!(parse_number(".5"), Some(Value::Float(0.5)));
        assert_eq!(parse_number("1."), Some(Value::Float(1.0)));
        // Exponent notation (lowercase only)
        assert_eq!(parse_number("1e10"), Some(Value::Float(1e10)));
        assert_eq!(parse_number("1.5e10"), Some(Value::Float(1.5e10)));
        assert_eq!(parse_number("-3e5"), Some(Value::Float(-3e5)));
        assert_eq!(parse_number("1e+5"), Some(Value::Float(1e5)));
        assert_eq!(parse_number(".5e2"), Some(Value::Float(0.5e2)));
        // Uppercase E is rejected
        assert_eq!(parse_number("1E10"), None);
        assert_eq!(parse_number("1.5E-10"), None);
    }

    #[test]
    fn test_find_colon_outside_quotes() {
        assert_eq!(find_colon_outside_quotes("a: 1"), Some(1));
        assert_eq!(find_colon_outside_quotes("\"a:b\": 1"), Some(5));
        assert_eq!(find_colon_outside_quotes("'a:b': 1"), Some(5));
    }
}
