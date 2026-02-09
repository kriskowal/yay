//! YAY (Yet Another YAML) parser implementation.
//!
//! YAY is a data serialization format that is more expressive than JSON
//! (supporting big integers and byte arrays) while having fewer surprising
//! behaviors than YAML.
//!
//! # Parsing Pipeline
//!
//! The parser operates in three phases:
//!
//! 1. **Scanner**: Converts source text into scan lines, validating encoding
//!    and extracting indentation and list markers.
//!
//! 2. **Outline Lexer**: Converts scan lines into a token stream with explicit
//!    block start/stop markers based on indentation changes.
//!
//! 3. **Value Parser**: Recursively parses the token stream into Rust values.

mod encode;
mod error;
mod lexer;
mod meh;
mod parser;
mod scanner;
pub mod shon;
mod value;
mod yson;

pub use encode::{encode, Format};
pub use error::{ParseError, Result};
pub use meh::format_yay;
pub use shon::{
    parse_shon_bracket, parse_shon_file_bytes, parse_shon_file_string, parse_shon_hex, ShonError,
};
pub use value::Value;
pub use yson::parse_yson;

/// Parse a YAY document from a string.
///
/// # Example
///
/// ```
/// use libyay::parse;
///
/// let value = parse("42").unwrap();
/// ```
pub fn parse(input: &str) -> Result<Value> {
    parse_with_filename(input, None)
}

/// Parse a YAY document from a string with a filename for error messages.
pub fn parse_with_filename(input: &str, filename: Option<&str>) -> Result<Value> {
    let ctx = error::ParseContext::new(filename);

    // Phase 1: Scan source into lines
    let scan_result = scanner::scan(input, &ctx)?;

    // Phase 2: Convert lines to token stream
    let tokens = lexer::outline_lex(&scan_result.lines);

    // Phase 3: Parse tokens into value
    parser::parse_root(&tokens, &ctx, scan_result.had_comments)
}

// Unit tests removed - coverage should come from fixtures
// #[cfg(test)]
// mod tests { ... }
