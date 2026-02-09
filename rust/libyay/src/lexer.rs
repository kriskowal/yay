//! Phase 2: Outline Lexer
//!
//! The outline lexer converts scan lines into a token stream. It tracks
//! indentation levels using a stack and emits:
//! - `Start`: When a list item begins or indent increases
//! - `Stop`: When indent decreases (block ends)
//! - `Text`: Line content
//! - `Break`: Blank lines (coalesced)

use crate::scanner::ScanLine;

/// Token type in the outline lexer output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    /// Block start (list item or multiline construct).
    Start,
    /// Block end (dedent).
    Stop,
    /// Text content.
    Text,
    /// Blank line.
    Break,
}

/// A single token in the token stream.
#[derive(Debug, Clone)]
pub struct Token {
    pub typ: TokenType,
    pub text: String,
    pub indent: usize,
    pub line_num: usize,
    pub col: usize,
}

impl Token {
    fn new(typ: TokenType, text: &str, indent: usize, line_num: usize, col: usize) -> Self {
        Self {
            typ,
            text: text.to_string(),
            indent,
            line_num,
            col,
        }
    }

    fn stop() -> Self {
        Self {
            typ: TokenType::Stop,
            text: String::new(),
            indent: 0,
            line_num: 0,
            col: 0,
        }
    }
}

/// Convert scan lines to a token stream with block markers.
pub fn outline_lex(lines: &[ScanLine]) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut stack = vec![0usize]; // Indent level stack, starts at 0
    let mut top = 0; // Current indent level
    let mut broken = false; // Whether we just emitted a break

    for sl in lines {
        // Emit stops for each level we dedent past
        while sl.indent < top {
            tokens.push(Token::stop());
            stack.pop();
            top = *stack.last().unwrap_or(&0);
        }

        // Emit start for list items
        if !sl.leader.is_empty() {
            if sl.indent > top {
                // New nested block
                tokens.push(Token::new(
                    TokenType::Start,
                    &sl.leader,
                    sl.indent,
                    sl.line_num,
                    sl.indent,
                ));
                stack.push(sl.indent);
                top = sl.indent;
                broken = false;
            } else if sl.indent == top {
                // Sibling item - close previous, start new
                tokens.push(Token::stop());
                tokens.push(Token::new(
                    TokenType::Start,
                    &sl.leader,
                    sl.indent,
                    sl.line_num,
                    sl.indent,
                ));
                broken = false;
            }
        }

        // Emit text or break
        if !sl.line.is_empty() {
            tokens.push(Token::new(
                TokenType::Text,
                &sl.line,
                sl.indent,
                sl.line_num,
                sl.indent,
            ));
            broken = false;
        } else if !broken {
            // Empty line - emit break if not already broken
            tokens.push(Token::new(
                TokenType::Break,
                "",
                sl.indent,
                sl.line_num,
                sl.indent,
            ));
            broken = true;
        }
    }

    // Close any remaining open blocks
    while stack.len() > 1 {
        tokens.push(Token::stop());
        stack.pop();
    }

    tokens
}

// Unit tests removed - coverage comes from fixtures
