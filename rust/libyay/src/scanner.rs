//! Phase 1: Scanner
//!
//! The scanner converts raw source text into scan lines. It performs:
//! - UTF-8 validation (no BOM, no surrogates)
//! - Whitespace validation (no tabs, no trailing spaces)
//! - Indentation counting
//! - List marker extraction (the "-" prefix)
//! - Comment filtering

use crate::error::{ParseContext, ParseError, Result};

/// A single line after the scanning phase.
#[derive(Debug, Clone)]
pub struct ScanLine {
    /// Content after indent and leader.
    pub line: String,
    /// Number of leading spaces.
    pub indent: usize,
    /// "- " for list items, "" otherwise.
    pub leader: String,
    /// Zero-based line number for error reporting.
    pub line_num: usize,
}

/// Result of scanning including metadata.
pub struct ScanResult {
    pub lines: Vec<ScanLine>,
    pub had_comments: bool,
}

/// Scan source text into scan lines with validation.
pub fn scan(source: &str, ctx: &ParseContext) -> Result<ScanResult> {
    // Validate: No BOM allowed
    validate_no_bom(source, ctx)?;

    // Validate: No forbidden code points
    validate_code_points(source, ctx)?;

    // Process each line
    scan_lines(source, ctx)
}

/// Check that the source doesn't start with a UTF-8 BOM.
fn validate_no_bom(source: &str, ctx: &ParseContext) -> Result<()> {
    if source.starts_with('\u{FEFF}') {
        return Err(ParseError::IllegalBom(String::new()).with_location(ctx, 0, 0));
    }
    Ok(())
}

/// Check whether a code point is allowed in a YAY document.
/// Only U+000A (line feed) and printable characters are permitted.
fn is_allowed_code_point(cp: u32) -> bool {
    cp == 0x000A
        || (0x0020..=0x007E).contains(&cp)
        || (0x00A0..=0xD7FF).contains(&cp)
        || ((0xE000..=0xFFFD).contains(&cp) && !(0xFDD0..=0xFDEF).contains(&cp))
        || ((0x10000..=0x10FFFF).contains(&cp) && (cp & 0xFFFF) < 0xFFFE)
}

/// Validate that the source contains no forbidden code points.
fn validate_code_points(source: &str, ctx: &ParseContext) -> Result<()> {
    let mut line = 0;
    let mut col = 0;
    for ch in source.chars() {
        let cp = ch as u32;
        if !is_allowed_code_point(cp) {
            // Tabs get their own specific error message.
            if cp == 0x0009 {
                return Err(ParseError::TabNotAllowed(String::new()).with_location(ctx, line, col));
            }
            // Surrogates get their own specific error message.
            if (0xD800..=0xDFFF).contains(&cp) {
                return Err(
                    ParseError::IllegalSurrogate(String::new()).with_location(ctx, line, col)
                );
            }
            return Err(
                ParseError::ForbiddenCodePoint(cp, String::new()).with_location(ctx, line, col)
            );
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    Ok(())
}

/// Process each line of source, extracting indent and leader.
fn scan_lines(source: &str, ctx: &ParseContext) -> Result<ScanResult> {
    let mut lines = Vec::new();
    let mut had_comments = false;
    // While inside a block string, the indent of the line that opened it.
    // Its content lines are literal: no list leaders, no `-`/`*` checks, no
    // comments. The block ends at the next non-empty line indented the same or
    // less than its opener.
    let mut block_opener: Option<usize> = None;

    for (line_num, line_str) in source.split('\n').enumerate() {
        // Validate: No trailing spaces
        if !line_str.is_empty() && line_str.ends_with(' ') {
            return Err(ParseError::TrailingSpace(String::new()).with_location(
                ctx,
                line_num,
                line_str.len() - 1,
            ));
        }

        // Count leading spaces (indent)
        let indent = count_indent(line_str);

        let rest = &line_str[indent..];

        if let Some(opener) = block_opener {
            if line_str.is_empty() || indent > opener {
                lines.push(ScanLine {
                    line: rest.to_string(),
                    indent,
                    leader: String::new(),
                    line_num,
                });
                continue;
            }
            block_opener = None;
        }

        // Skip top-level comments but track that we saw them
        if rest.starts_with('#') && indent == 0 {
            had_comments = true;
            continue;
        }

        // Extract leader (list marker) and content
        let (leader, content) = extract_leader(rest, line_num, indent, ctx)?;

        if opens_block_string(content) {
            // A property's block ends at its key's column, which follows the
            // list leader in `- key: \``.
            block_opener = Some(if is_bare_block_opener(content) {
                indent
            } else {
                indent + leader.len()
            });
        }

        lines.push(ScanLine {
            line: content.to_string(),
            indent,
            leader: leader.to_string(),
            line_num,
        });
    }

    Ok(ScanResult {
        lines,
        had_comments,
    })
}

/// Whether a line's content (after any list leader) opens a block string:
/// a property `key: \``, or a bare `\`` / `\` text` at root or as a list item.
fn opens_block_string(content: &str) -> bool {
    is_bare_block_opener(content) || content.ends_with(": `")
}

/// Whether a line's content is a bare `\`` or `\` text` opener rather than a
/// property.
fn is_bare_block_opener(content: &str) -> bool {
    content == "`" || content.starts_with("` ")
}

/// Count the number of leading spaces in a line.
fn count_indent(line: &str) -> usize {
    line.bytes().take_while(|&b| b == b' ').count()
}

/// Separate the list marker from line content.
/// Returns (leader, content) where leader is "- " for list items.
fn extract_leader<'a>(
    rest: &'a str,
    line_num: usize,
    indent: usize,
    ctx: &ParseContext,
) -> Result<(&'static str, &'a str)> {
    // "- " prefix is the list marker (dash + space)
    if let Some(content) = rest.strip_prefix("- ") {
        return Ok(("- ", content));
    }

    // Compact list syntax (-value without space) is not allowed
    // But "-1", "-.5", and "-infinity" are valid numbers/keywords
    if rest.starts_with('-') && rest.len() >= 2 {
        let second = rest.chars().nth(1).unwrap();
        if second != ' ' && second != '.' && !second.is_ascii_digit() && rest != "-infinity" {
            return Err(
                ParseError::ExpectedSpaceAfter("-".to_string(), String::new()).with_location(
                    ctx,
                    line_num,
                    indent + 1,
                ),
            );
        }
    }

    // "*" or "* " at top level is an error
    if rest == "*" || rest.starts_with("* ") {
        return Err(
            ParseError::UnexpectedChar('*', String::new()).with_location(ctx, line_num, indent)
        );
    }

    Ok(("", rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_string_lines_are_literal() {
        let ctx = ParseContext::new(None);
        let result = scan("body: `\n  - item\n  * star\n  ---\n  -x\nnext: 1", &ctx).unwrap();
        assert_eq!(result.lines.len(), 6);
        assert_eq!(result.lines[1].line, "- item");
        assert_eq!(result.lines[1].leader, "");
        assert_eq!(result.lines[3].line, "---");
        assert_eq!(result.lines[5].line, "next: 1");
    }

    #[test]
    fn test_count_indent() {
        assert_eq!(count_indent(""), 0);
        assert_eq!(count_indent("hello"), 0);
        assert_eq!(count_indent("  hello"), 2);
        assert_eq!(count_indent("    hello"), 4);
    }

    #[test]
    fn test_scan_simple() {
        let ctx = ParseContext::new(None);
        let result = scan("hello", &ctx).unwrap();
        assert_eq!(result.lines.len(), 1);
        assert_eq!(result.lines[0].line, "hello");
        assert_eq!(result.lines[0].indent, 0);
    }

    #[test]
    fn test_scan_list() {
        let ctx = ParseContext::new(None);
        let result = scan("- item", &ctx).unwrap();
        assert_eq!(result.lines.len(), 1);
        assert_eq!(result.lines[0].line, "item");
        assert_eq!(result.lines[0].leader, "- ");
    }

    #[test]
    fn test_scan_comment() {
        let ctx = ParseContext::new(None);
        let result = scan("# comment\nvalue", &ctx).unwrap();
        assert_eq!(result.lines.len(), 1);
        assert_eq!(result.lines[0].line, "value");
    }

    #[test]
    fn test_trailing_space_error() {
        let ctx = ParseContext::new(None);
        let result = scan("hello ", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_tab_error() {
        let ctx = ParseContext::new(None);
        let result = scan("\thello", &ctx);
        assert!(result.is_err());
    }
}
