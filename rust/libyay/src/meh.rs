//! MEH Parser and Formatter
//!
//! MEH is a loose CST (Concrete Syntax Tree) parser for YAY that:
//! - Tolerates extra whitespace where strict YAY forbids it
//! - Preserves comments and their line associations
//! - Preserves key order (not sorted alphabetically)
//! - Preserves blank lines between sections
//!
//! The MEH pipeline consists of:
//! 1. MEH Parser - Parses loose YAY into a CST
//! 2. MEH-to-YAY Transform - Normalizes the CST to canonical form
//! 3. MEH Formatter - Serializes the CST back to text

use std::env;

/// Default line wrap length
const DEFAULT_WRAP: usize = 80;

/// Get the line wrap length from YAY_WRAP env var or default
fn get_wrap_length() -> usize {
    env::var("YAY_WRAP")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_WRAP)
}

// =============================================================================
// CST Types
// =============================================================================

/// A CST node representing a YAY document
#[derive(Debug, Clone)]
pub struct Document {
    pub items: Vec<Item>,
    pub trailing_comments: Vec<Comment>,
}

/// An item in a document or block
#[derive(Debug, Clone)]
pub enum Item {
    /// A blank line
    BlankLine,
    /// A comment line
    Comment(Comment),
    /// A value (scalar, array, object, etc.)
    Value(CstValue),
    /// A key-value property
    Property(Property),
    /// An array item (- prefix)
    ArrayItem(ArrayItem),
}

/// A comment with its content
#[derive(Debug, Clone)]
pub struct Comment {
    pub text: String,                // Content after #
    pub align_column: Option<usize>, // Column to align to (for inline comments in blocks)
}

/// A key-value property
#[derive(Debug, Clone)]
pub struct Property {
    pub key: Key,
    pub value: Option<PropertyValue>,
    pub inline_comment: Option<Comment>,
}

/// A property key
#[derive(Debug, Clone)]
pub enum Key {
    Bare(String),
    SingleQuoted(String),
    DoubleQuoted(String),
}

/// The value part of a property
#[derive(Debug, Clone)]
pub enum PropertyValue {
    Inline(CstValue),
    Block(Block),
}

/// A block (indented content after a property)
#[derive(Debug, Clone)]
pub struct Block {
    pub items: Vec<Item>,
}

/// An array item (- prefix)
#[derive(Debug, Clone)]
pub struct ArrayItem {
    pub value: Option<ArrayItemValue>,
    pub inline_comment: Option<Comment>,
}

/// The value part of an array item
#[derive(Debug, Clone)]
pub enum ArrayItemValue {
    Inline(CstValue),
    Block(Block),
}

/// A CST value node
#[derive(Debug, Clone)]
pub enum CstValue {
    Null,
    Bool(bool),
    Integer(String), // Preserve original formatting (spaces)
    Float(String),   // Preserve original formatting
    String(CstString),
    Bytes(CstBytes),
    Array(CstArray),
    Object(CstObject),
}

/// A string value
#[derive(Debug, Clone)]
pub enum CstString {
    SingleQuoted(String),
    DoubleQuoted(String),
    Block(BlockString),
}

/// A block string (backtick)
#[derive(Debug, Clone)]
pub struct BlockString {
    pub first_line: Option<String>, // Content after ` on first line
    pub lines: Vec<BlockStringLine>,
}

#[derive(Debug, Clone)]
pub struct BlockStringLine {
    pub indent: usize,
    pub content: String,
}

/// A bytes value
#[derive(Debug, Clone)]
pub enum CstBytes {
    Inline(InlineBytes),
    Block(BlockBytes),
}

/// Inline bytes <hex>
#[derive(Debug, Clone)]
pub struct InlineBytes {
    pub content: String, // Content between < and >, preserving spaces
}

/// Block bytes (> prefix)
#[derive(Debug, Clone)]
pub struct BlockBytes {
    pub first_line_comment: Option<Comment>,
    pub lines: Vec<BlockBytesLine>,
}

#[derive(Debug, Clone)]
pub struct BlockBytesLine {
    pub indent: usize,
    pub hex: String,
    pub comment: Option<Comment>,
}

/// Inline array [...]
#[derive(Debug, Clone)]
pub struct CstArray {
    pub items: Vec<CstArrayItem>,
}

#[derive(Debug, Clone)]
pub struct CstArrayItem {
    pub value: CstValue,
}

/// Inline object {...}
#[derive(Debug, Clone)]
pub struct CstObject {
    pub entries: Vec<CstObjectEntry>,
}

#[derive(Debug, Clone)]
pub struct CstObjectEntry {
    pub key: Key,
    pub value: CstValue,
}

// =============================================================================
// MEH Parser
// =============================================================================

pub struct MehParser<'a> {
    lines: Vec<&'a str>,
    line_idx: usize,
    col: usize,
}

impl<'a> MehParser<'a> {
    pub fn new(input: &'a str) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        Self {
            lines,
            line_idx: 0,
            col: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Document, String> {
        let mut items = Vec::new();

        while self.line_idx < self.lines.len() {
            if let Some(item) = self.parse_item(0)? {
                items.push(item);
            }
        }

        Ok(Document {
            items,
            trailing_comments: Vec::new(),
        })
    }

    fn current_line(&self) -> Option<&'a str> {
        self.lines.get(self.line_idx).copied()
    }

    fn advance_line(&mut self) {
        self.line_idx += 1;
        self.col = 0;
    }

    fn parse_item(&mut self, min_indent: usize) -> Result<Option<Item>, String> {
        let line = match self.current_line() {
            Some(l) => l,
            None => return Ok(None),
        };

        // Blank line
        if line.trim().is_empty() {
            self.advance_line();
            return Ok(Some(Item::BlankLine));
        }

        let indent = count_indent(line);
        let content = &line[indent..];

        // Check indent
        if indent < min_indent {
            return Ok(None);
        }

        // Comment line
        if content.starts_with('#') {
            let comment = Comment {
                text: content[1..].to_string(),
                align_column: None,
            };
            self.advance_line();
            return Ok(Some(Item::Comment(comment)));
        }

        // Array item
        if content.starts_with("- ") || content == "-" {
            return self.parse_array_item(indent);
        }

        // Property (key: value)
        if let Some(colon_idx) = find_colon_outside_quotes(content) {
            return self.parse_property(indent, colon_idx);
        }

        // Block string at root level
        if content == "`" || content.starts_with("` ") {
            let first_line = if content.starts_with("` ") {
                Some(content[2..].to_string())
            } else {
                None
            };
            self.advance_line();
            let block_str = self.parse_block_string_lines(indent, first_line)?;
            return Ok(Some(Item::Value(CstValue::String(CstString::Block(
                block_str,
            )))));
        }

        // Block bytes at root level
        if content == ">" || content.starts_with("> ") {
            let (first_hex, first_comment) = if content.starts_with("> ") {
                let rest = &content[2..];
                if rest.trim_start().starts_with('#') {
                    // Just a comment on the first line
                    (
                        None,
                        Some(Comment {
                            text: rest.trim_start()[1..].to_string(),
                            align_column: None,
                        }),
                    )
                } else if !rest.trim().is_empty() {
                    // Hex data on the first line (possibly with inline comment)
                    let (hex_part, comment) = split_inline_comment(rest);
                    (Some(hex_part.trim().to_string()), comment)
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };
            self.advance_line();
            let mut block_bytes = self.parse_block_bytes(indent, first_comment.clone())?;
            // If there was hex on the first line, prepend it
            if let Some(hex) = first_hex {
                let first_line = BlockBytesLine {
                    indent,
                    hex,
                    comment: first_comment,
                };
                block_bytes.lines.insert(0, first_line);
                block_bytes.first_line_comment = None;
            }
            return Ok(Some(Item::Value(CstValue::Bytes(CstBytes::Block(
                block_bytes,
            )))));
        }

        // Standalone value
        let value = self.parse_inline_value(content)?;
        self.advance_line();
        Ok(Some(Item::Value(value)))
    }

    fn parse_array_item(&mut self, indent: usize) -> Result<Option<Item>, String> {
        let line = self.current_line().unwrap();
        let content = &line[indent..];

        // Extract content after "- "
        let after_dash = if content.starts_with("- ") {
            &content[2..]
        } else {
            "" // Just "-"
        };

        // Check for inline comment
        let (value_part, inline_comment) = split_inline_comment(after_dash);

        self.advance_line();

        // Parse the value part
        let value = if value_part.is_empty() {
            // Check for block content
            let block = self.parse_block(indent)?;
            if block.items.is_empty() {
                None
            } else {
                Some(ArrayItemValue::Block(block))
            }
        } else if value_part == "`" || value_part.starts_with("` ") {
            // Block string in array item
            let first_line = if value_part.starts_with("` ") {
                Some(value_part[2..].to_string())
            } else {
                None
            };
            let block_str = self.parse_block_string_lines(indent, first_line)?;
            Some(ArrayItemValue::Inline(CstValue::String(CstString::Block(
                block_str,
            ))))
        } else if value_part.starts_with("- ") || value_part == "-" {
            // Nested array item on same line (e.g., "- - a")
            // Parse the rest as a nested array item
            let nested_item = self.parse_inline_array_item(value_part, indent + 2)?;
            // Then check for more items on subsequent lines
            let mut block_items = vec![Item::ArrayItem(nested_item)];
            let block = self.parse_block(indent)?;
            block_items.extend(block.items);
            Some(ArrayItemValue::Block(Block { items: block_items }))
        } else if find_colon_outside_quotes(value_part).is_some() {
            // Nested property on same line (e.g., "- a: 1")
            let mut nested_item = self.parse_inline_property(value_part, indent + 2)?;
            // If the property has no inline value, parse its block content
            if nested_item.value.is_none() {
                let prop_block = self.parse_block(indent + 2)?;
                if !prop_block.items.is_empty() {
                    nested_item.value = Some(PropertyValue::Block(prop_block));
                }
            }
            let mut block_items = vec![Item::Property(nested_item)];
            // Parse sibling items at the same level as the property
            let block = self.parse_block(indent)?;
            block_items.extend(block.items);
            Some(ArrayItemValue::Block(Block { items: block_items }))
        } else {
            Some(ArrayItemValue::Inline(self.parse_inline_value(value_part)?))
        };

        Ok(Some(Item::ArrayItem(ArrayItem {
            value,
            inline_comment,
        })))
    }

    fn parse_property(&mut self, indent: usize, colon_idx: usize) -> Result<Option<Item>, String> {
        let line = self.current_line().unwrap();
        let content = &line[indent..];

        let key_str = &content[..colon_idx];
        let after_colon = &content[colon_idx + 1..];

        // Parse key
        let key = parse_key(key_str.trim());

        let value_part = after_colon.trim_start();

        // Check for inline comment
        let (value_str, inline_comment) = split_inline_comment(value_part);

        self.advance_line();

        // Parse value
        let value = if value_str.is_empty() {
            // Check for block content
            let block = self.parse_block(indent)?;
            if block.items.is_empty() {
                None
            } else {
                Some(PropertyValue::Block(block))
            }
        } else if value_str == "`" {
            // Block string
            let block_str = self.parse_block_string(indent)?;
            Some(PropertyValue::Inline(CstValue::String(CstString::Block(
                block_str,
            ))))
        } else if value_str == ">" || value_str.starts_with("> ") {
            // Block bytes
            let (first_hex, first_comment) = if value_str.starts_with("> ") {
                let rest = &value_str[2..];
                if rest.trim_start().starts_with('#') {
                    // Just a comment on the first line
                    (
                        None,
                        Some(Comment {
                            text: rest.trim_start()[1..].to_string(),
                            align_column: None,
                        }),
                    )
                } else if !rest.trim().is_empty() {
                    // Hex data on the first line (possibly with inline comment)
                    let (hex_part, comment) = split_inline_comment(rest);
                    (Some(hex_part.trim().to_string()), comment)
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };
            let mut block_bytes = self.parse_block_bytes(indent, first_comment.clone())?;
            // If there was hex on the first line, prepend it
            if let Some(hex) = first_hex {
                let first_line = BlockBytesLine {
                    indent,
                    hex,
                    comment: first_comment,
                };
                block_bytes.lines.insert(0, first_line);
                block_bytes.first_line_comment = None;
            }
            Some(PropertyValue::Inline(CstValue::Bytes(CstBytes::Block(
                block_bytes,
            ))))
        } else {
            Some(PropertyValue::Inline(self.parse_inline_value(value_str)?))
        };

        Ok(Some(Item::Property(Property {
            key,
            value,
            inline_comment,
        })))
    }

    fn parse_block(&mut self, parent_indent: usize) -> Result<Block, String> {
        let mut items = Vec::new();

        while let Some(line) = self.current_line() {
            if line.trim().is_empty() {
                // Peek ahead to see if next non-blank line is still in this block
                let mut peek_idx = self.line_idx + 1;
                while peek_idx < self.lines.len() && self.lines[peek_idx].trim().is_empty() {
                    peek_idx += 1;
                }

                // If next non-blank line has indent <= parent, don't consume blank line
                if peek_idx < self.lines.len() {
                    let next_indent = count_indent(self.lines[peek_idx]);
                    if next_indent <= parent_indent {
                        break;
                    }
                }

                self.advance_line();
                items.push(Item::BlankLine);
                continue;
            }

            let indent = count_indent(line);
            if indent <= parent_indent {
                break;
            }

            if let Some(item) = self.parse_item(indent)? {
                items.push(item);
            }
        }

        Ok(Block { items })
    }

    /// Parse an array item from inline content (e.g., "- a" from "- - a")
    fn parse_inline_array_item(
        &mut self,
        content: &str,
        _indent: usize,
    ) -> Result<ArrayItem, String> {
        // content starts with "- " or is just "-"
        let after_dash = if content.starts_with("- ") {
            &content[2..]
        } else {
            ""
        };

        let (value_part, inline_comment) = split_inline_comment(after_dash);

        let value = if value_part.is_empty() {
            None
        } else if value_part.starts_with("- ") || value_part == "-" {
            // Recursively nested array item
            let nested = self.parse_inline_array_item(value_part, _indent + 2)?;
            Some(ArrayItemValue::Block(Block {
                items: vec![Item::ArrayItem(nested)],
            }))
        } else if find_colon_outside_quotes(value_part).is_some() {
            // Nested property
            let nested = self.parse_inline_property(value_part, _indent + 2)?;
            Some(ArrayItemValue::Block(Block {
                items: vec![Item::Property(nested)],
            }))
        } else {
            Some(ArrayItemValue::Inline(self.parse_inline_value(value_part)?))
        };

        Ok(ArrayItem {
            value,
            inline_comment,
        })
    }

    /// Parse a property from inline content (e.g., "a: 1" from "- a: 1")
    fn parse_inline_property(&self, content: &str, _indent: usize) -> Result<Property, String> {
        let colon_idx = find_colon_outside_quotes(content).unwrap();
        let key_str = &content[..colon_idx];
        let after_colon = &content[colon_idx + 1..];

        let key = parse_key(key_str.trim());
        let value_part = after_colon.trim_start();
        let (value_str, inline_comment) = split_inline_comment(value_part);

        let value = if value_str.is_empty() {
            None
        } else {
            Some(PropertyValue::Inline(self.parse_inline_value(value_str)?))
        };

        Ok(Property {
            key,
            value,
            inline_comment,
        })
    }

    fn parse_block_string(&mut self, parent_indent: usize) -> Result<BlockString, String> {
        self.parse_block_string_lines(parent_indent, None)
    }

    fn parse_block_string_lines(
        &mut self,
        parent_indent: usize,
        first_line: Option<String>,
    ) -> Result<BlockString, String> {
        let mut lines = Vec::new();

        while let Some(line) = self.current_line() {
            if line.trim().is_empty() {
                self.advance_line();
                lines.push(BlockStringLine {
                    indent: 0,
                    content: String::new(),
                });
                continue;
            }

            let indent = count_indent(line);
            if indent <= parent_indent {
                break;
            }

            lines.push(BlockStringLine {
                indent,
                content: line[indent..].to_string(),
            });
            self.advance_line();
        }

        Ok(BlockString { first_line, lines })
    }

    fn parse_block_bytes(
        &mut self,
        parent_indent: usize,
        first_comment: Option<Comment>,
    ) -> Result<BlockBytes, String> {
        let mut lines = Vec::new();

        while let Some(line) = self.current_line() {
            if line.trim().is_empty() {
                break;
            }

            let indent = count_indent(line);
            if indent <= parent_indent {
                break;
            }

            let content = &line[indent..];

            // Check for standalone comment line (no hex data)
            if content.starts_with('#') {
                // Standalone comment - store as a hex line with empty hex and the comment
                lines.push(BlockBytesLine {
                    indent,
                    hex: String::new(),
                    comment: Some(Comment {
                        text: content[1..].to_string(),
                        align_column: None,
                    }),
                });
                self.advance_line();
                continue;
            }

            let (hex_part, comment) = split_inline_comment(content);

            lines.push(BlockBytesLine {
                indent,
                hex: hex_part.to_string(),
                comment,
            });
            self.advance_line();
        }

        Ok(BlockBytes {
            first_line_comment: first_comment,
            lines,
        })
    }

    fn parse_inline_value(&self, s: &str) -> Result<CstValue, String> {
        let s = s.trim();

        if s.is_empty() {
            return Ok(CstValue::Null);
        }

        // Keywords
        match s {
            "null" => return Ok(CstValue::Null),
            "true" => return Ok(CstValue::Bool(true)),
            "false" => return Ok(CstValue::Bool(false)),
            _ => {}
        }

        // String
        if s.starts_with('"') {
            return Ok(CstValue::String(CstString::DoubleQuoted(s.to_string())));
        }
        if s.starts_with('\'') {
            return Ok(CstValue::String(CstString::SingleQuoted(s.to_string())));
        }

        // Inline bytes
        if s.starts_with('<') && s.ends_with('>') {
            return Ok(CstValue::Bytes(CstBytes::Inline(InlineBytes {
                content: s[1..s.len() - 1].to_string(),
            })));
        }

        // Inline array
        if s.starts_with('[') && s.ends_with(']') {
            return self.parse_inline_array(s);
        }

        // Inline object
        if s.starts_with('{') && s.ends_with('}') {
            return self.parse_inline_object(s);
        }

        // Number (integer or float)
        if looks_like_number(s) {
            if s.contains('.')
                || s.contains('e')
                || s.contains('E')
                || s == "nan"
                || s == "infinity"
                || s == "-infinity"
            {
                return Ok(CstValue::Float(s.to_string()));
            } else {
                return Ok(CstValue::Integer(s.to_string()));
            }
        }

        // Special float keywords
        match s {
            "nan" => return Ok(CstValue::Float(s.to_string())),
            "infinity" | "-infinity" => return Ok(CstValue::Float(s.to_string())),
            _ => {}
        }

        // Bare identifier - treat as a string value
        // This handles cases like property values that are unquoted identifiers
        Ok(CstValue::String(CstString::DoubleQuoted(format!(
            "\"{}\"",
            s
        ))))
    }

    fn parse_inline_array(&self, s: &str) -> Result<CstValue, String> {
        let inner = &s[1..s.len() - 1];
        let trimmed = inner.trim();

        if trimmed.is_empty() {
            return Ok(CstValue::Array(CstArray { items: Vec::new() }));
        }

        let mut items = Vec::new();
        let mut remaining = trimmed;

        while !remaining.is_empty() {
            let (value_str, rest) = split_array_item(remaining);
            let value = self.parse_inline_value(value_str.trim())?;

            items.push(CstArrayItem { value });

            remaining = rest.trim_start();
            if remaining.starts_with(',') {
                remaining = &remaining[1..];
                remaining = remaining.trim_start();
            }
        }

        Ok(CstValue::Array(CstArray { items }))
    }

    fn parse_inline_object(&self, s: &str) -> Result<CstValue, String> {
        let inner = &s[1..s.len() - 1];
        let trimmed = inner.trim();

        if trimmed.is_empty() {
            return Ok(CstValue::Object(CstObject {
                entries: Vec::new(),
            }));
        }

        let mut entries = Vec::new();
        let mut remaining = trimmed;

        while !remaining.is_empty() {
            let (entry_str, rest) = split_object_entry(remaining);

            if let Some(colon_idx) = find_colon_outside_quotes(entry_str) {
                let key_str = &entry_str[..colon_idx];
                let value_str = &entry_str[colon_idx + 1..];

                let key = parse_key(key_str.trim());
                let value = self.parse_inline_value(value_str.trim())?;

                entries.push(CstObjectEntry { key, value });
            }

            remaining = rest.trim_start();
            if remaining.starts_with(',') {
                remaining = &remaining[1..];
                remaining = remaining.trim_start();
            }
        }

        Ok(CstValue::Object(CstObject { entries }))
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn count_indent(line: &str) -> usize {
    line.bytes().take_while(|&b| b == b' ').count()
}

fn find_colon_outside_quotes(s: &str) -> Option<usize> {
    let mut in_double = false;
    let mut in_single = false;
    let mut escape = false;
    let mut depth: i32 = 0;

    for (i, c) in s.chars().enumerate() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' && (in_double || in_single) {
            escape = true;
            continue;
        }
        if c == '"' && !in_single {
            in_double = !in_double;
        } else if c == '\'' && !in_double {
            in_single = !in_single;
        } else if !in_double && !in_single {
            match c {
                '[' | '{' | '<' => depth += 1,
                ']' | '}' | '>' => depth -= 1,
                ':' if depth == 0 => return Some(i),
                _ => {}
            }
        }
    }
    None
}

fn parse_key(s: &str) -> Key {
    if s.starts_with('"') && s.ends_with('"') {
        Key::DoubleQuoted(s.to_string())
    } else if s.starts_with('\'') && s.ends_with('\'') {
        Key::SingleQuoted(s.to_string())
    } else {
        Key::Bare(s.to_string())
    }
}

fn split_inline_comment(s: &str) -> (&str, Option<Comment>) {
    // Find # not inside quotes
    let mut in_double = false;
    let mut in_single = false;
    let mut escape = false;

    for (i, c) in s.chars().enumerate() {
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
            let value_part = s[..i].trim_end();
            let comment_text = &s[i + 1..];
            return (
                value_part,
                Some(Comment {
                    text: comment_text.to_string(),
                    align_column: None,
                }),
            );
        }
    }
    (s, None)
}

fn split_array_item(s: &str) -> (&str, &str) {
    let mut depth = 0;
    let mut in_double = false;
    let mut in_single = false;
    let mut escape = false;

    for (i, c) in s.chars().enumerate() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' && (in_double || in_single) {
            escape = true;
            continue;
        }
        if c == '"' && !in_single {
            in_double = !in_double;
        } else if c == '\'' && !in_double {
            in_single = !in_single;
        } else if !in_double && !in_single {
            match c {
                '[' | '{' | '<' => depth += 1,
                ']' | '}' | '>' => depth -= 1,
                ',' if depth == 0 => return (&s[..i], &s[i..]),
                _ => {}
            }
        }
    }
    (s, "")
}

fn split_object_entry(s: &str) -> (&str, &str) {
    split_array_item(s) // Same logic
}

fn looks_like_number(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }

    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    // Optional minus
    if i < chars.len() && chars[i] == '-' {
        i += 1;
    }

    // Must have at least one digit or be a special keyword
    if i >= chars.len() {
        return false;
    }

    // Check for digits, dots, spaces (digit grouping), e/E
    let mut has_digit = false;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() {
            has_digit = true;
        } else if c == '.' || c == ' ' || c == 'e' || c == 'E' || c == '+' || c == '-' {
            // Allow these in numbers
        } else {
            return false;
        }
        i += 1;
    }

    has_digit
}

// =============================================================================
// MEH-to-YAY Transform
// =============================================================================

pub fn transform_to_canonical(doc: &Document) -> Document {
    let wrap = get_wrap_length();
    let transformer = Transformer { wrap };
    transformer.transform_document(doc)
}

struct Transformer {
    wrap: usize,
}

impl Transformer {
    fn transform_document(&self, doc: &Document) -> Document {
        let items = self.transform_items_with_alignment(&doc.items, 0);

        Document {
            items,
            trailing_comments: doc.trailing_comments.clone(),
        }
    }

    /// Transform a list of items, handling blank line collapsing and comment alignment
    fn transform_items_with_alignment(&self, items: &[Item], base_indent: usize) -> Vec<Item> {
        let mut result = Vec::new();
        let mut prev_was_blank = false;

        // First pass: transform all items (may produce multiple items from one)
        for item in items {
            match item {
                Item::BlankLine => {
                    if !prev_was_blank && !result.is_empty() {
                        result.push(Item::BlankLine);
                        prev_was_blank = true;
                    }
                }
                _ => {
                    let transformed = self.transform_item_maybe_expand(item, base_indent);
                    result.extend(transformed);
                    prev_was_blank = false;
                }
            }
        }

        // Remove trailing blank lines
        while matches!(result.last(), Some(Item::BlankLine)) {
            result.pop();
        }

        // Second pass: align inline comments within contiguous groups
        self.align_comments_in_items(&mut result, base_indent);

        result
    }

    /// Transform an item, potentially expanding it into multiple items
    /// (e.g., when converting inline array to block form)
    fn transform_item_maybe_expand(&self, item: &Item, indent: usize) -> Vec<Item> {
        match item {
            Item::Property(p) => self.transform_property_maybe_expand(p, indent),
            _ => vec![self.transform_item(item, indent)],
        }
    }

    /// Transform a property, potentially expanding inline arrays/objects to block form
    fn transform_property_maybe_expand(&self, prop: &Property, indent: usize) -> Vec<Item> {
        let key_width = self.key_width(&prop.key);

        // Check if we need to expand an inline array/object to block form
        if let Some(PropertyValue::Inline(value)) = &prop.value {
            let indent_width = indent * 2;
            let prefix_width = indent_width + key_width + 2; // "key: "

            match value {
                CstValue::Array(arr) => {
                    let value_width = self.measure_array(arr);
                    if prefix_width + value_width > self.wrap {
                        // Convert to block form: property with no value, followed by array items
                        // Array items are siblings at the same indent level
                        let mut items = vec![Item::Property(Property {
                            key: prop.key.clone(),
                            value: None,
                            inline_comment: prop.inline_comment.clone(),
                        })];

                        for arr_item in &arr.items {
                            items.push(Item::ArrayItem(ArrayItem {
                                value: Some(ArrayItemValue::Inline(
                                    self.transform_value(&arr_item.value, indent),
                                )),
                                inline_comment: None,
                            }));
                        }

                        return items;
                    }
                }
                CstValue::Object(obj) => {
                    let value_width = self.measure_object(obj);
                    if prefix_width + value_width > self.wrap {
                        // Convert to block form: property with block value containing nested properties
                        // Object properties are children at indent + 1
                        let nested_items: Vec<Item> = obj
                            .entries
                            .iter()
                            .map(|entry| {
                                Item::Property(Property {
                                    key: entry.key.clone(),
                                    value: Some(PropertyValue::Inline(
                                        self.transform_value(&entry.value, indent + 1),
                                    )),
                                    inline_comment: None,
                                })
                            })
                            .collect();

                        return vec![Item::Property(Property {
                            key: prop.key.clone(),
                            value: Some(PropertyValue::Block(Block {
                                items: nested_items,
                            })),
                            inline_comment: prop.inline_comment.clone(),
                        })];
                    }
                }
                _ => {}
            }
        }

        // No expansion needed, just transform normally
        vec![Item::Property(self.transform_property(prop, indent))]
    }

    /// Align inline comments within contiguous groups of items
    fn align_comments_in_items(&self, items: &mut [Item], base_indent: usize) {
        let indent_width = base_indent * 2; // 2 spaces per indent level

        // Find contiguous groups (separated by blank lines or standalone comments)
        let mut group_start = 0;
        while group_start < items.len() {
            // Skip blank lines and standalone comments
            if matches!(items[group_start], Item::BlankLine | Item::Comment(_)) {
                group_start += 1;
                continue;
            }

            // Find end of this group
            let mut group_end = group_start + 1;
            while group_end < items.len() {
                if matches!(items[group_end], Item::BlankLine | Item::Comment(_)) {
                    break;
                }
                group_end += 1;
            }

            // Align comments in this group
            self.align_group(&mut items[group_start..group_end], indent_width);

            group_start = group_end;
        }
    }

    /// Align comments within a contiguous group of items
    fn align_group(&self, items: &mut [Item], indent_width: usize) {
        // Calculate max data width for items with inline comments
        let mut max_data_width = 0usize;
        for item in items.iter() {
            if let Some(width) = self.item_data_width(item) {
                if self.item_has_inline_comment(item) {
                    max_data_width = max_data_width.max(width);
                }
            }
        }

        if max_data_width == 0 {
            return; // No items with comments
        }

        // The alignment column is where # starts: indent + max_data_width + 2 spaces
        let align_col = indent_width + max_data_width + 2;

        // Set alignment on all inline comments in this group
        for item in items.iter_mut() {
            self.set_item_comment_alignment(item, align_col);
        }
    }

    /// Get the data width of an item (excluding comment)
    fn item_data_width(&self, item: &Item) -> Option<usize> {
        match item {
            Item::Property(p) => {
                let key_width = self.key_width(&p.key);
                let value_width = p
                    .value
                    .as_ref()
                    .map(|v| match v {
                        PropertyValue::Inline(val) => self.measure_value(val),
                        PropertyValue::Block(_) => 0, // Block values don't count
                    })
                    .unwrap_or(0);
                // Use saturating_add to avoid overflow when value_width is usize::MAX
                // (which indicates a block value that shouldn't be inlined)
                if value_width > 0 && value_width < usize::MAX {
                    Some(key_width.saturating_add(2).saturating_add(value_width))
                // key: value
                } else if value_width == usize::MAX {
                    None // Block value, can't calculate inline width
                } else {
                    Some(key_width + 1) // key:
                }
            }
            Item::ArrayItem(a) => {
                match &a.value {
                    Some(ArrayItemValue::Inline(val)) => {
                        let val_width = self.measure_value(val);
                        if val_width == usize::MAX {
                            None // Block value, can't calculate inline width
                        } else {
                            Some(2usize.saturating_add(val_width)) // "- " + value
                        }
                    }
                    Some(ArrayItemValue::Block(_)) => Some(1), // Just "-"
                    None => Some(1),
                }
            }
            _ => None,
        }
    }

    /// Check if an item has an inline comment
    fn item_has_inline_comment(&self, item: &Item) -> bool {
        match item {
            Item::Property(p) => p.inline_comment.is_some(),
            Item::ArrayItem(a) => a.inline_comment.is_some(),
            _ => false,
        }
    }

    /// Set the alignment column on an item's inline comment
    fn set_item_comment_alignment(&self, item: &mut Item, align_col: usize) {
        match item {
            Item::Property(p) => {
                if let Some(ref mut comment) = p.inline_comment {
                    comment.align_column = Some(align_col);
                }
            }
            Item::ArrayItem(a) => {
                if let Some(ref mut comment) = a.inline_comment {
                    comment.align_column = Some(align_col);
                }
            }
            _ => {}
        }
    }

    /// Measure the formatted width of a key
    fn key_width(&self, key: &Key) -> usize {
        match key {
            Key::Bare(s) => s.len(),
            Key::SingleQuoted(s) => s.len(),
            Key::DoubleQuoted(s) => s.len(),
        }
    }

    /// Measure the formatted width of a value (for inline representation)
    fn measure_value(&self, value: &CstValue) -> usize {
        match value {
            CstValue::Null => 4,
            CstValue::Bool(true) => 4,
            CstValue::Bool(false) => 5,
            CstValue::Integer(s) => normalize_number_spaces(s).len(),
            CstValue::Float(s) => s.len(),
            CstValue::String(s) => self.measure_string(s),
            CstValue::Bytes(b) => self.measure_bytes(b),
            CstValue::Array(a) => self.measure_array(a),
            CstValue::Object(o) => self.measure_object(o),
        }
    }

    fn measure_string(&self, s: &CstString) -> usize {
        match s {
            CstString::SingleQuoted(v) => v.len(),
            CstString::DoubleQuoted(v) => v.len(),
            CstString::Block(_) => usize::MAX, // Block strings are multiline
        }
    }

    fn measure_bytes(&self, b: &CstBytes) -> usize {
        match b {
            CstBytes::Inline(ib) => {
                let normalized = normalize_hex_spaces(&ib.content);
                normalized.len() + 2 // < and >
            }
            CstBytes::Block(_) => usize::MAX, // Block bytes are multiline
        }
    }

    fn measure_array(&self, arr: &CstArray) -> usize {
        if arr.items.is_empty() {
            return 2; // []
        }
        let mut total = 2; // [ and ]
        for (i, item) in arr.items.iter().enumerate() {
            if i > 0 {
                total += 2; // ", "
            }
            let item_width = self.measure_value(&item.value);
            if item_width == usize::MAX {
                return usize::MAX;
            }
            total += item_width;
        }
        total
    }

    fn measure_object(&self, obj: &CstObject) -> usize {
        if obj.entries.is_empty() {
            return 2; // {}
        }
        let mut total = 2; // { and }
        for (i, entry) in obj.entries.iter().enumerate() {
            if i > 0 {
                total += 2; // ", "
            }
            let key_width = self.key_width(&entry.key);
            let value_width = self.measure_value(&entry.value);
            if value_width == usize::MAX {
                return usize::MAX;
            }
            total += key_width + 2 + value_width; // key: value
        }
        total
    }

    fn transform_item(&self, item: &Item, indent: usize) -> Item {
        match item {
            Item::BlankLine => Item::BlankLine,
            Item::Comment(c) => Item::Comment(c.clone()),
            Item::Value(v) => Item::Value(self.transform_value(v, indent)),
            Item::Property(p) => Item::Property(self.transform_property(p, indent)),
            Item::ArrayItem(a) => Item::ArrayItem(self.transform_array_item(a, indent)),
        }
    }

    fn transform_property(&self, prop: &Property, indent: usize) -> Property {
        Property {
            key: prop.key.clone(),
            value: prop
                .value
                .as_ref()
                .map(|v| self.transform_property_value(v, indent)),
            inline_comment: prop.inline_comment.clone(),
        }
    }

    fn transform_property_value(&self, value: &PropertyValue, indent: usize) -> PropertyValue {
        match value {
            PropertyValue::Inline(v) => PropertyValue::Inline(self.transform_value(v, indent)),
            PropertyValue::Block(b) => {
                // Check if block contains only a single Item::Value - if so, promote to inline
                // This handles cases like:
                //   key:
                //     `
                //       content
                // which should become:
                //   key: `
                //     content
                if b.items.len() == 1 {
                    if let Item::Value(v) = &b.items[0] {
                        return PropertyValue::Inline(self.transform_value(v, indent));
                    }
                }
                PropertyValue::Block(self.transform_block(b, indent + 1))
            }
        }
    }

    fn transform_block(&self, block: &Block, indent: usize) -> Block {
        Block {
            items: self.transform_items_with_alignment(&block.items, indent),
        }
    }

    fn transform_array_item(&self, item: &ArrayItem, indent: usize) -> ArrayItem {
        ArrayItem {
            value: item.value.as_ref().map(|v| match v {
                ArrayItemValue::Inline(val) => {
                    ArrayItemValue::Inline(self.transform_value(val, indent))
                }
                ArrayItemValue::Block(b) => {
                    ArrayItemValue::Block(self.transform_block(b, indent + 1))
                }
            }),
            inline_comment: item.inline_comment.clone(),
        }
    }

    fn transform_value(&self, value: &CstValue, indent: usize) -> CstValue {
        match value {
            CstValue::Null => CstValue::Null,
            CstValue::Bool(b) => CstValue::Bool(*b),
            CstValue::Integer(s) => CstValue::Integer(normalize_number_spaces(s)),
            CstValue::Float(s) => CstValue::Float(canonicalize_float(s)),
            CstValue::String(s) => CstValue::String(s.clone()),
            CstValue::Bytes(b) => self.transform_bytes(b, indent),
            CstValue::Array(a) => self.transform_array(a, indent),
            CstValue::Object(o) => self.transform_object(o, indent),
        }
    }

    fn transform_bytes(&self, bytes: &CstBytes, indent: usize) -> CstValue {
        match bytes {
            CstBytes::Inline(ib) => {
                let normalized = normalize_hex_spaces(&ib.content);
                let inline_width = indent * 2 + normalized.len() + 2; // indent + <hex>

                // If too long, convert to block form
                if inline_width > self.wrap {
                    self.inline_bytes_to_block(&normalized, indent)
                } else {
                    CstValue::Bytes(CstBytes::Inline(InlineBytes {
                        content: normalized,
                    }))
                }
            }
            CstBytes::Block(bb) => {
                // Normalize hex in each line
                let mut lines: Vec<BlockBytesLine> = bb
                    .lines
                    .iter()
                    .map(|line| BlockBytesLine {
                        indent: line.indent,
                        hex: normalize_hex_spaces(&line.hex),
                        comment: line.comment.clone(),
                    })
                    .collect();

                // Phase 1: Join fragmented comments (continuation lines with no hex)
                lines = self.join_block_bytes_comments(lines);

                // Phase 2: Align comments in block bytes
                self.align_block_bytes_comments(&mut lines, indent);

                // Phase 3: Wrap long comments (may re-split them)
                lines = self.wrap_block_bytes_comments(lines, indent);

                CstValue::Bytes(CstBytes::Block(BlockBytes {
                    first_line_comment: bb.first_line_comment.clone(),
                    lines,
                }))
            }
        }
    }

    /// Convert inline bytes to block form when too long
    fn inline_bytes_to_block(&self, hex: &str, indent: usize) -> CstValue {
        // Remove spaces to get raw hex
        let raw: String = hex.chars().filter(|c| !c.is_whitespace()).collect();

        // Calculate how many bytes fit per line
        // Line format: "  " * (indent+1) + hex + possible comment
        // We want roughly 4 words (16 bytes = 32 hex chars + 7 spaces = 39 chars) per line
        let bytes_per_line = 16;
        let chars_per_line = bytes_per_line * 2;

        let mut lines = Vec::new();
        let chars: Vec<char> = raw.chars().collect();

        for chunk in chars.chunks(chars_per_line) {
            let chunk_str: String = chunk.iter().collect();
            let formatted = normalize_hex_spaces(&chunk_str);
            lines.push(BlockBytesLine {
                indent: indent + 1,
                hex: formatted,
                comment: None,
            });
        }

        CstValue::Bytes(CstBytes::Block(BlockBytes {
            first_line_comment: None,
            lines,
        }))
    }

    /// Join fragmented comments: merge continuation lines (empty hex) into preceding lines.
    /// Stop joining when a line ends with sentence-ending punctuation.
    /// A period followed by a capitalized word (like "Mr. Smith") is NOT a sentence end.
    fn join_block_bytes_comments(&self, lines: Vec<BlockBytesLine>) -> Vec<BlockBytesLine> {
        /// Check if a word starts with a capital letter
        fn starts_with_capital(word: &str) -> bool {
            word.chars()
                .next()
                .map_or(false, |c| c.is_ascii_uppercase())
        }

        /// Check if we should join: previous line doesn't end a sentence,
        /// OR it ends with "Abbr." followed by a capitalized continuation word
        fn should_join_comments(prev_text: &str, next_text: &str) -> bool {
            let prev_trimmed = prev_text.trim_end();
            if prev_trimmed.is_empty() {
                return false;
            }

            // If previous doesn't end with sentence punctuation, join
            if !prev_trimmed.ends_with('.')
                && !prev_trimmed.ends_with('!')
                && !prev_trimmed.ends_with('?')
            {
                return true;
            }

            // Previous ends with punctuation - check for abbreviation pattern
            // If prev ends with "Capital." and next starts with "Capital", it's likely
            // an abbreviation like "Mr. Smith", so join
            if prev_trimmed.ends_with('.') {
                let last_word = prev_trimmed.split_whitespace().last().unwrap_or("");
                let next_trimmed = next_text.trim_start();
                let first_word = next_trimmed.split_whitespace().next().unwrap_or("");

                // Pattern: "Mr. Smith" - both capitalized, period between
                if starts_with_capital(last_word) && starts_with_capital(first_word) {
                    return true;
                }
            }

            // Ends with ! or ?, or period not followed by capital - sentence end
            false
        }

        let mut result: Vec<BlockBytesLine> = Vec::new();

        for line in lines {
            if !line.hex.is_empty() {
                // Line with hex data - start fresh
                result.push(line);
            } else if let Some(comment) = &line.comment {
                // Continuation line (no hex, just comment)
                let should_join = result.last().map_or(false, |prev| {
                    if let Some(prev_comment) = &prev.comment {
                        should_join_comments(&prev_comment.text, &comment.text)
                    } else {
                        false
                    }
                });

                if should_join {
                    // Join with previous comment
                    if let Some(prev) = result.last_mut() {
                        if let Some(ref mut prev_comment) = prev.comment {
                            let continuation_text = comment.text.trim();
                            prev_comment.text =
                                format!("{} {}", prev_comment.text.trim_end(), continuation_text);
                        }
                    }
                } else {
                    // Can't join - keep as standalone comment
                    result.push(line);
                }
            } else {
                // Empty line with no comment - keep it
                result.push(line);
            }
        }

        result
    }

    /// Align comments within block bytes lines
    fn align_block_bytes_comments(&self, lines: &mut [BlockBytesLine], indent: usize) {
        // Find max hex width among lines with inline comments (not standalone comments)
        let mut max_hex_width = 0usize;
        for line in lines.iter() {
            // Only consider lines with actual hex data for alignment
            if line.comment.is_some() && !line.hex.is_empty() {
                max_hex_width = max_hex_width.max(line.hex.len());
            }
        }

        if max_hex_width == 0 {
            return;
        }

        // align_col is where # starts: indent + max_hex_width + 2 spaces
        let indent_width = (indent + 1) * 2;
        let align_col = indent_width + max_hex_width + 2;

        // Track the last alignment column seen (for standalone comments that follow)
        let mut last_align_col: Option<usize> = None;

        for line in lines.iter_mut() {
            if !line.hex.is_empty() {
                // Line with hex data - align to the computed column
                if let Some(ref mut comment) = line.comment {
                    comment.align_column = Some(align_col);
                    last_align_col = Some(align_col);
                }
            } else if let Some(ref mut comment) = line.comment {
                // Standalone comment - align to same column as previous comment
                if let Some(prev_col) = last_align_col {
                    comment.align_column = Some(prev_col);
                }
            }
        }
    }

    /// Wrap long comments in block bytes lines
    fn wrap_block_bytes_comments(
        &self,
        lines: Vec<BlockBytesLine>,
        indent: usize,
    ) -> Vec<BlockBytesLine> {
        let indent_width = (indent + 1) * 2;
        let mut result = Vec::new();

        for line in lines {
            if line.hex.is_empty() {
                // Standalone comment - don't wrap
                result.push(line);
                continue;
            }

            let Some(comment) = &line.comment else {
                result.push(line);
                continue;
            };

            // Calculate line length: indent + hex + spaces_to_align + # + comment
            let align_col = comment
                .align_column
                .unwrap_or(indent_width + line.hex.len() + 2);
            let line_len = align_col + 1 + comment.text.len(); // +1 for #

            if line_len <= self.wrap {
                result.push(line);
                continue;
            }

            // Need to wrap the comment
            let available_width = self.wrap.saturating_sub(align_col + 1); // +1 for #
            let wrapped = wrap_comment_text(&comment.text, available_width);

            // First line keeps the hex data
            result.push(BlockBytesLine {
                indent: line.indent,
                hex: line.hex.clone(),
                comment: Some(Comment {
                    text: wrapped[0].clone(),
                    align_column: comment.align_column,
                }),
            });

            // Continuation lines are standalone comments aligned to the same column
            for continuation in &wrapped[1..] {
                result.push(BlockBytesLine {
                    indent: line.indent,
                    hex: String::new(), // Empty hex = standalone comment
                    comment: Some(Comment {
                        text: format!(" {}", continuation), // Space after #
                        align_column: Some(align_col),      // Align with the # above
                    }),
                });
            }
        }

        result
    }

    fn transform_array(&self, arr: &CstArray, indent: usize) -> CstValue {
        let items: Vec<CstArrayItem> = arr
            .items
            .iter()
            .map(|item| CstArrayItem {
                value: self.transform_value(&item.value, indent),
            })
            .collect();

        CstValue::Array(CstArray { items })
    }

    fn transform_object(&self, obj: &CstObject, indent: usize) -> CstValue {
        let entries: Vec<CstObjectEntry> = obj
            .entries
            .iter()
            .map(|entry| CstObjectEntry {
                key: entry.key.clone(),
                value: self.transform_value(&entry.value, indent),
            })
            .collect();

        CstValue::Object(CstObject { entries })
    }
}

/// Wrap comment text at word boundaries, keeping abbreviation pairs together.
/// Handles bullet points (starting with "- ") with hanging indent.
/// An abbreviation pair is detected as "Capital. Capital" (e.g., "Mr. Smith").
fn wrap_comment_text(text: &str, max_width: usize) -> Vec<String> {
    // Preserve leading space if present
    let has_leading_space = text.starts_with(' ');
    let text_trimmed = text.trim();

    if text_trimmed.len() <= max_width {
        return vec![text.to_string()];
    }

    // Check for bullet point
    let (is_bullet, content) = if text_trimmed.starts_with("- ") {
        (true, &text_trimmed[2..])
    } else {
        (false, text_trimmed)
    };

    let words: Vec<&str> = content.split_whitespace().collect();

    if words.is_empty() {
        return vec![String::new()];
    }

    /// Check if a word starts with a capital letter
    fn starts_with_capital(word: &str) -> bool {
        word.chars()
            .next()
            .map_or(false, |c| c.is_ascii_uppercase())
    }

    /// Check if this word and the next form an abbreviation pair like "Mr. Smith"
    fn is_abbreviation_pair(word: &str, next_word: Option<&str>) -> bool {
        if !word.ends_with('.') {
            return false;
        }
        if !starts_with_capital(word) {
            return false;
        }
        match next_word {
            Some(next) => starts_with_capital(next),
            None => false,
        }
    }

    /// Check if a word ends a sentence (ends with . ! ? and is not part of an abbreviation pair)
    fn is_sentence_end(word: &str, next_word: Option<&str>) -> bool {
        let ends_with_punct = word.ends_with('.') || word.ends_with('!') || word.ends_with('?');
        if !ends_with_punct {
            return false;
        }
        // If it's an abbreviation pair, not a sentence end
        !is_abbreviation_pair(word, next_word)
    }

    // For bullets: first line has "- ", continuations have "  " (hanging indent)
    let first_line_prefix = if is_bullet { "- " } else { "" };
    let cont_line_prefix = if is_bullet { "  " } else { "" };

    // Adjust max width for prefixes
    let first_line_width = max_width.saturating_sub(first_line_prefix.len());
    let cont_line_width = max_width.saturating_sub(cont_line_prefix.len());

    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_max_width = first_line_width;
    let mut last_sentence_break: Option<(String, usize)> = None;

    let mut i = 0;
    while i < words.len() {
        let word = words[i];
        let next_word = words.get(i + 1).copied();

        // Check if this word forms an abbreviation pair with the next word
        let is_abbrev_pair = is_abbreviation_pair(word, next_word);
        let combined = if is_abbrev_pair {
            format!("{} {}", word, next_word.unwrap())
        } else {
            word.to_string()
        };

        let addition = if current_line.is_empty() {
            combined.clone()
        } else {
            format!(" {}", combined)
        };

        let words_consumed = if is_abbrev_pair { 2 } else { 1 };

        if current_line.len() + addition.len() <= current_max_width || current_line.is_empty() {
            current_line.push_str(&addition);
            i += words_consumed;

            // Track sentence boundaries for potential break points
            let last_word_added = if is_abbrev_pair {
                next_word.unwrap()
            } else {
                word
            };
            let following_word = words.get(i).copied();
            if is_sentence_end(last_word_added, following_word) {
                last_sentence_break = Some((current_line.clone(), i));
            }
        } else {
            // Line is full - prefer breaking at last sentence boundary if available
            if let Some((line_at_break, next_idx)) = last_sentence_break.take() {
                lines.push(line_at_break);
                current_line = String::new();
                i = next_idx;
            } else {
                lines.push(current_line);
                current_line = String::new();
                // Don't increment i - we'll add this word to the new line
            }
            // After first line, use continuation width
            current_max_width = cont_line_width;
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    // Ensure we have at least one line
    if lines.is_empty() {
        lines.push(content.to_string());
    }

    // Add prefixes
    for (idx, line) in lines.iter_mut().enumerate() {
        let prefix = if idx == 0 {
            first_line_prefix
        } else {
            cont_line_prefix
        };
        *line = format!("{}{}", prefix, line);
    }

    // Add leading space back to first line if it was present
    if has_leading_space && !lines.is_empty() {
        lines[0] = format!(" {}", lines[0]);
    }

    lines
}

fn normalize_number_spaces(s: &str) -> String {
    // Collapse multiple spaces to single space
    let mut result = String::new();
    let mut prev_space = false;

    for c in s.chars() {
        if c == ' ' {
            if !prev_space && !result.is_empty() {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(c);
            prev_space = false;
        }
    }

    result.trim().to_string()
}

/// Canonicalize a float string to its shortest representation that round-trips.
fn canonicalize_float(s: &str) -> String {
    // Handle special values
    match s {
        "nan" => return "nan".to_string(),
        "infinity" => return "infinity".to_string(),
        "-infinity" => return "-infinity".to_string(),
        _ => {}
    }

    // Remove spaces (digit grouping)
    let compact: String = s.chars().filter(|c| *c != ' ').collect();

    // Parse to f64
    let f: f64 = match compact.parse() {
        Ok(v) => v,
        Err(_) => return s.to_string(), // Preserve original if parse fails
    };

    // Handle special float values
    if f.is_nan() {
        return "nan".to_string();
    }
    if f.is_infinite() {
        return if f.is_sign_positive() {
            "infinity".to_string()
        } else {
            "-infinity".to_string()
        };
    }

    // Format to shortest representation
    // Try both regular and exponential notation, pick the shorter one
    let regular = format!("{}", f);
    let exponential = format!("{:e}", f);

    // Choose the shorter representation
    let formatted = if exponential.len() < regular.len() {
        exponential
    } else {
        regular
    };

    // Ensure floats have a decimal point to distinguish from integers
    if !formatted.contains('.') && !formatted.contains('e') && !formatted.contains('E') {
        format!("{}.0", formatted)
    } else {
        formatted
    }
}

fn normalize_hex_spaces(s: &str) -> String {
    // Remove all spaces, lowercase, then re-add with proper grouping
    let hex: String = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(|c| c.to_lowercase())
        .collect();

    if hex.is_empty() {
        return String::new();
    }

    // Group into bytes (2 chars each), then words (4 bytes = 8 chars)
    let mut result = String::new();
    let chars: Vec<char> = hex.chars().collect();

    for (i, chunk) in chars.chunks(2).enumerate() {
        if i > 0 {
            if i % 4 == 0 {
                result.push_str("  "); // Double space between words
            } else {
                result.push(' '); // Single space between bytes
            }
        }
        for &c in chunk {
            result.push(c);
        }
    }

    result
}

// =============================================================================
// MEH Formatter
// =============================================================================

pub fn format_document(doc: &Document) -> String {
    let mut formatter = Formatter::new();
    formatter.format_document(doc)
}

struct Formatter {
    output: String,
    indent: usize,
    current_column: usize,   // Track current column for alignment
    in_property_value: bool, // Whether we're formatting a property value
}

impl Formatter {
    fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
            current_column: 0,
            in_property_value: false,
        }
    }

    fn format_document(&mut self, doc: &Document) -> String {
        for item in &doc.items {
            self.format_item(item);
        }

        // Ensure trailing newline
        if !self.output.is_empty() && !self.output.ends_with('\n') {
            self.output.push('\n');
        }

        self.output.clone()
    }

    fn write_indent(&mut self) {
        let indent_str = "  ".repeat(self.indent);
        self.output.push_str(&indent_str);
        self.current_column = indent_str.len();
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
        // Update column (simplified: assumes no newlines in s)
        self.current_column += s.len();
    }

    fn write_char(&mut self, c: char) {
        self.output.push(c);
        self.current_column += 1;
    }

    fn newline(&mut self) {
        self.output.push('\n');
        self.current_column = 0;
    }

    /// Write an inline comment with optional alignment
    fn write_inline_comment(&mut self, comment: &Comment) {
        if let Some(align_col) = comment.align_column {
            // Pad to alignment column (minimum 2 spaces)
            let padding = if self.current_column < align_col {
                align_col - self.current_column
            } else {
                2
            };
            for _ in 0..padding {
                self.write_char(' ');
            }
        } else {
            // Default: 2 spaces before comment
            self.write("  ");
        }
        self.write_char('#');
        self.write(&comment.text);
    }

    fn format_item(&mut self, item: &Item) {
        match item {
            Item::BlankLine => {
                self.newline();
            }
            Item::Comment(c) => {
                self.write_indent();
                self.write_char('#');
                self.write(&c.text);
                self.newline();
            }
            Item::Value(v) => {
                self.write_indent();
                self.format_value(v);
                // Block values (block strings, block bytes) already end with newline
                let is_block_value = matches!(
                    v,
                    CstValue::String(CstString::Block(_)) | CstValue::Bytes(CstBytes::Block(_))
                );
                if !is_block_value {
                    self.newline();
                }
            }
            Item::Property(p) => {
                self.format_property(p);
            }
            Item::ArrayItem(a) => {
                self.format_array_item(a);
            }
        }
    }

    fn format_property(&mut self, prop: &Property) {
        self.write_indent();
        self.format_key(&prop.key);
        self.write_char(':');

        match &prop.value {
            Some(PropertyValue::Inline(v)) => {
                // Check if this is a block value (bytes or string) that spans multiple lines
                let is_block_value = matches!(
                    v,
                    CstValue::Bytes(CstBytes::Block(_)) | CstValue::String(CstString::Block(_))
                );

                self.write_char(' ');
                self.in_property_value = true;
                self.format_value(v);
                self.in_property_value = false;

                if !is_block_value {
                    if let Some(comment) = &prop.inline_comment {
                        self.write_inline_comment(comment);
                    }
                    self.newline();
                }
            }
            Some(PropertyValue::Block(block)) => {
                if let Some(comment) = &prop.inline_comment {
                    self.write_inline_comment(comment);
                }
                self.newline();
                self.indent += 1;
                for item in &block.items {
                    self.format_item(item);
                }
                self.indent -= 1;
            }
            None => {
                if let Some(comment) = &prop.inline_comment {
                    self.write_inline_comment(comment);
                }
                self.newline();
            }
        }
    }

    fn format_array_item(&mut self, item: &ArrayItem) {
        self.write_indent();

        match &item.value {
            Some(ArrayItemValue::Inline(v)) => {
                self.write("- ");
                self.format_value(v);
                // Block values (block strings, block bytes) already end with newline
                let is_block_value = matches!(
                    v,
                    CstValue::String(CstString::Block(_)) | CstValue::Bytes(CstBytes::Block(_))
                );
                if !is_block_value {
                    if let Some(comment) = &item.inline_comment {
                        self.write_inline_comment(comment);
                    }
                    self.newline();
                }
            }
            Some(ArrayItemValue::Block(block)) => {
                // Check if first item is an array item or property that can go on same line
                let mut items_iter = block.items.iter();
                if let Some(first_item) = items_iter.next() {
                    match first_item {
                        Item::ArrayItem(nested) => {
                            // Write "- " then the nested array item inline
                            self.write("- ");
                            self.format_array_item_inline(nested);
                        }
                        Item::Property(nested) => {
                            // Write "- " then the property inline
                            self.write("- ");
                            self.format_property_inline(nested);
                        }
                        _ => {
                            // Other items: write "-" then newline
                            self.write_char('-');
                            if let Some(comment) = &item.inline_comment {
                                self.write_inline_comment(comment);
                            }
                            self.newline();
                            self.indent += 1;
                            self.format_item(first_item);
                            for item in items_iter {
                                self.format_item(item);
                            }
                            self.indent -= 1;
                            return;
                        }
                    }
                } else {
                    self.write_char('-');
                    if let Some(comment) = &item.inline_comment {
                        self.write_inline_comment(comment);
                    }
                    self.newline();
                    return;
                }
                // Format remaining items
                self.indent += 1;
                for item in items_iter {
                    self.format_item(item);
                }
                self.indent -= 1;
            }
            None => {
                self.write_char('-');
                if let Some(comment) = &item.inline_comment {
                    self.write_inline_comment(comment);
                }
                self.newline();
            }
        }
    }

    /// Format an array item inline (without leading indent, for nested items on same line)
    fn format_array_item_inline(&mut self, item: &ArrayItem) {
        match &item.value {
            Some(ArrayItemValue::Inline(v)) => {
                self.write("- ");
                self.format_value(v);
                if let Some(comment) = &item.inline_comment {
                    self.write_inline_comment(comment);
                }
                self.newline();
            }
            Some(ArrayItemValue::Block(block)) => {
                // Recursively handle nested blocks
                let mut items_iter = block.items.iter();
                if let Some(first_item) = items_iter.next() {
                    match first_item {
                        Item::ArrayItem(nested) => {
                            self.write("- ");
                            self.format_array_item_inline(nested);
                        }
                        Item::Property(nested) => {
                            self.write("- ");
                            self.format_property_inline(nested);
                        }
                        _ => {
                            self.write_char('-');
                            self.newline();
                            self.indent += 1;
                            self.format_item(first_item);
                            for item in items_iter {
                                self.format_item(item);
                            }
                            self.indent -= 1;
                            return;
                        }
                    }
                } else {
                    self.write_char('-');
                    self.newline();
                    return;
                }
                self.indent += 1;
                for item in items_iter {
                    self.format_item(item);
                }
                self.indent -= 1;
            }
            None => {
                self.write_char('-');
                if let Some(comment) = &item.inline_comment {
                    self.write_inline_comment(comment);
                }
                self.newline();
            }
        }
    }

    /// Format a property inline (without leading indent, for nested items on same line)
    fn format_property_inline(&mut self, prop: &Property) {
        self.format_key(&prop.key);
        self.write_char(':');

        match &prop.value {
            Some(PropertyValue::Inline(v)) => {
                let is_block_value = matches!(
                    v,
                    CstValue::Bytes(CstBytes::Block(_)) | CstValue::String(CstString::Block(_))
                );

                self.write_char(' ');
                self.in_property_value = true;
                self.format_value(v);
                self.in_property_value = false;

                if !is_block_value {
                    if let Some(comment) = &prop.inline_comment {
                        self.write_inline_comment(comment);
                    }
                    self.newline();
                }
            }
            Some(PropertyValue::Block(block)) => {
                if let Some(comment) = &prop.inline_comment {
                    self.write_inline_comment(comment);
                }
                self.newline();
                // Use +2 indent for block content to distinguish from siblings
                self.indent += 2;
                for item in &block.items {
                    self.format_item(item);
                }
                self.indent -= 2;
            }
            None => {
                if let Some(comment) = &prop.inline_comment {
                    self.write_inline_comment(comment);
                }
                self.newline();
            }
        }
    }

    fn format_key(&mut self, key: &Key) {
        match key {
            Key::Bare(s) => self.write(s),
            Key::SingleQuoted(s) => self.write(s),
            Key::DoubleQuoted(s) => self.write(s),
        }
    }

    fn format_value(&mut self, value: &CstValue) {
        match value {
            CstValue::Null => self.write("null"),
            CstValue::Bool(true) => self.write("true"),
            CstValue::Bool(false) => self.write("false"),
            CstValue::Integer(s) => self.write(s),
            CstValue::Float(s) => self.write(s),
            CstValue::String(s) => self.format_string(s),
            CstValue::Bytes(b) => self.format_bytes(b),
            CstValue::Array(a) => self.format_array(a),
            CstValue::Object(o) => self.format_object(o),
        }
    }

    fn format_string(&mut self, s: &CstString) {
        match s {
            CstString::SingleQuoted(v) => self.write(v),
            CstString::DoubleQuoted(v) => self.write(v),
            CstString::Block(b) => {
                self.write_char('`');
                if let Some(first) = &b.first_line {
                    self.write_char(' ');
                    self.write(first);
                }
                self.newline();

                // Find minimum indent of non-empty lines to calculate relative indentation
                let min_indent = b
                    .lines
                    .iter()
                    .filter(|l| !l.content.is_empty())
                    .map(|l| l.indent)
                    .min()
                    .unwrap_or(0);

                let base_indent = "  ".repeat(self.indent + 1);

                for line in &b.lines {
                    if line.content.is_empty() {
                        // Empty lines should have no indent (no trailing spaces)
                        self.newline();
                    } else {
                        // Calculate relative indent from minimum
                        let relative_indent = line.indent.saturating_sub(min_indent);
                        let extra_spaces = " ".repeat(relative_indent);
                        self.output.push_str(&base_indent);
                        self.output.push_str(&extra_spaces);
                        self.current_column = base_indent.len() + relative_indent;
                        self.write(&line.content);
                        self.newline();
                    }
                }
            }
        }
    }

    fn format_bytes(&mut self, b: &CstBytes) {
        match b {
            CstBytes::Inline(ib) => {
                self.write_char('<');
                self.write(&ib.content);
                self.write_char('>');
            }
            CstBytes::Block(bb) => {
                self.write_char('>');
                // When in a property value, > must be alone on its line (strict parser rule)
                // At root level, first hex can be on the same line as >
                let mut lines_iter = bb.lines.iter().peekable();
                if !self.in_property_value {
                    // Root level: check if first line has hex data
                    if let Some(first_line) = lines_iter.peek() {
                        if !first_line.hex.is_empty() {
                            // Put first hex on same line as >
                            self.write_char(' ');
                            self.write(&first_line.hex);
                            if let Some(comment) = &first_line.comment {
                                self.write_inline_comment(comment);
                            }
                            self.newline();
                            lines_iter.next(); // consume the first line
                        } else if let Some(comment) = &bb.first_line_comment {
                            self.write_inline_comment(comment);
                            self.newline();
                        } else {
                            self.newline();
                        }
                    } else if let Some(comment) = &bb.first_line_comment {
                        self.write_inline_comment(comment);
                        self.newline();
                    } else {
                        self.newline();
                    }
                } else {
                    // Property value: > must be alone, hex on next lines
                    if let Some(comment) = &bb.first_line_comment {
                        self.write_inline_comment(comment);
                    }
                    self.newline();
                }
                for line in lines_iter {
                    let indent_str = "  ".repeat(self.indent + 1);
                    self.output.push_str(&indent_str);
                    self.current_column = indent_str.len();

                    if line.hex.is_empty() {
                        // Standalone comment line (could be a continuation)
                        if let Some(comment) = &line.comment {
                            if let Some(align_col) = comment.align_column {
                                // This is a continuation line - pad to align with #
                                let padding = align_col.saturating_sub(self.current_column);
                                for _ in 0..padding {
                                    self.write_char(' ');
                                }
                            }
                            self.write_char('#');
                            self.write(&comment.text);
                        }
                    } else {
                        self.write(&line.hex);
                        if let Some(comment) = &line.comment {
                            self.write_inline_comment(comment);
                        }
                    }
                    self.newline();
                }
            }
        }
    }

    fn format_array(&mut self, arr: &CstArray) {
        self.write_char('[');
        for (i, item) in arr.items.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.format_value(&item.value);
        }
        self.write_char(']');
    }

    fn format_object(&mut self, obj: &CstObject) {
        self.write_char('{');
        for (i, entry) in obj.entries.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.format_key(&entry.key);
            self.write(": ");
            self.format_value(&entry.value);
        }
        self.write_char('}');
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Parse loose YAY (MEH) and format to canonical YAY
pub fn format_yay(input: &str) -> Result<String, String> {
    let mut parser = MehParser::new(input);
    let doc = parser.parse()?;
    let canonical = transform_to_canonical(&doc);
    Ok(format_document(&canonical))
}

// Most MEH functionality is tested via fixtures in test/meh/
// These unit tests cover internal helper functions not directly exercised by fixtures
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_hex() {
        assert_eq!(normalize_hex_spaces("cafebabe"), "ca fe ba be");
        assert_eq!(normalize_hex_spaces("ca fe ba be"), "ca fe ba be");
        assert_eq!(normalize_hex_spaces("cafe babe"), "ca fe ba be");
        assert_eq!(normalize_hex_spaces(""), "");
        assert_eq!(
            normalize_hex_spaces("cafebabe deadbeef"),
            "ca fe ba be  de ad be ef"
        );
        // Uppercase is canonicalized to lowercase
        assert_eq!(normalize_hex_spaces("CAFEBABE"), "ca fe ba be");
        assert_eq!(normalize_hex_spaces("DeAdBeEf"), "de ad be ef");
    }

    #[test]
    fn test_normalize_number() {
        assert_eq!(normalize_number_spaces("1000000"), "1000000");
        assert_eq!(normalize_number_spaces("1 000 000"), "1 000 000");
        assert_eq!(normalize_number_spaces("1  000   000"), "1 000 000");
        assert_eq!(normalize_number_spaces("  123  "), "123");
    }

    #[test]
    fn test_wrap_comment_text_short() {
        let result = wrap_comment_text(" short", 80);
        assert_eq!(result, vec![" short"]);
    }

    #[test]
    fn test_wrap_comment_text_long() {
        let long_text = " This is a very long comment that should be wrapped because it exceeds the maximum width";
        let result = wrap_comment_text(long_text, 40);
        assert!(result.len() > 1);
        assert!(result[0].starts_with(' '));
    }

    #[test]
    fn test_wrap_comment_text_bullet() {
        let bullet = " - This is a bullet point that is long enough to wrap to the next line";
        let result = wrap_comment_text(bullet, 40);
        assert!(result.len() > 1);
        assert!(result[0].contains("- "));
        assert!(result[1].starts_with("  "));
    }

    #[test]
    fn test_wrap_comment_text_abbreviation() {
        let text = " Please contact Mr. Smith for more information about this matter";
        let result = wrap_comment_text(text, 30);
        let joined = result.join(" ");
        assert!(joined.contains("Mr. Smith"));
    }

    #[test]
    fn test_wrap_comment_text_empty() {
        let result = wrap_comment_text("   ", 80);
        assert_eq!(result.len(), 1);
    }
}
