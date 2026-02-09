//! Error types for YAY parsing.

use thiserror::Error;

/// Result type for YAY parsing operations.
pub type Result<T> = std::result::Result<T, ParseError>;

/// Parse context carrying filename for error reporting.
#[derive(Clone, Debug)]
pub struct ParseContext {
    pub filename: Option<String>,
}

impl ParseContext {
    /// Create a new parse context.
    pub fn new(filename: Option<&str>) -> Self {
        Self {
            filename: filename.map(String::from),
        }
    }

    /// Format a location suffix for error messages.
    pub fn loc_suffix(&self, line: usize, col: usize) -> String {
        match &self.filename {
            Some(name) => format!(" at {}:{} of <{}>", line + 1, col + 1, name),
            None => String::new(),
        }
    }
}

/// Error type for YAY parsing.
#[derive(Error, Debug)]
pub enum ParseError {
    /// Illegal BOM at start of file.
    #[error("Illegal BOM{0}")]
    IllegalBom(String),

    /// Illegal surrogate code point.
    #[error("Illegal surrogate{0}")]
    IllegalSurrogate(String),

    /// Forbidden code point.
    #[error("Forbidden code point U+{0:04X}{1}")]
    ForbiddenCodePoint(u32, String),

    /// Tab character found where spaces expected.
    #[error("Tab not allowed (use spaces){0}")]
    TabNotAllowed(String),

    /// Trailing space on a line.
    #[error("Unexpected trailing space{0}")]
    TrailingSpace(String),

    /// Unexpected leading space.
    #[error("Unexpected leading space{0}")]
    LeadingSpace(String),

    /// Unexpected indent.
    #[error("Unexpected indent{0}")]
    UnexpectedIndent(String),

    /// Unexpected character.
    #[error("Unexpected character \"{0}\"{1}")]
    UnexpectedChar(char, String),

    /// Unterminated string.
    #[error("Unterminated string{0}")]
    UnterminatedString(String),

    /// Bad character in string.
    #[error("Bad character in string{0}")]
    BadCharInString(String),

    /// Bad escaped character.
    #[error("Bad escaped character{0}")]
    BadEscapedChar(String),

    /// Bad Unicode escape.
    #[error("Bad Unicode escape{0}")]
    BadUnicodeEscape(String),

    /// Unicode code point out of range.
    #[error("Unicode code point out of range{0}")]
    UnicodeOutOfRange(String),

    /// Odd number of hex digits in byte literal.
    #[error("Odd number of hex digits in byte literal{0}")]
    OddHexDigits(String),

    /// Invalid hex digit.
    #[error("Invalid hex digit{0}")]
    InvalidHexDigit(String),

    /// Uppercase hex digit (must be lowercase).
    #[error("Uppercase hex digit (use lowercase){0}")]
    UppercaseHex(String),

    /// Uppercase exponent (must be lowercase).
    #[error("Uppercase exponent (use lowercase 'e'){0}")]
    UppercaseExponent(String),

    /// Unexpected newline in inline construct.
    #[error("Unexpected newline in inline {0}{1}")]
    UnexpectedNewline(String, String),

    /// Unexpected extra content after value.
    #[error("Unexpected extra content{0}")]
    ExtraContent(String),

    /// Invalid number format.
    #[error("Invalid number{0}")]
    InvalidNumber(String),

    /// Expected colon after key.
    #[error("Expected colon after key{0}")]
    ExpectedColon(String),

    /// Invalid key.
    #[error("Invalid key{0}")]
    InvalidKey(String),

    /// Unmatched bracket.
    #[error("Unmatched bracket{0}")]
    UnmatchedBracket(String),

    /// Unmatched brace.
    #[error("Unmatched brace{0}")]
    UnmatchedBrace(String),

    /// Unmatched angle bracket.
    #[error("Unmatched angle bracket{0}")]
    UnmatchedAngle(String),

    /// Unexpected space after character.
    #[error("Unexpected space after \"{0}\"{1}")]
    UnexpectedSpaceAfter(String, String),

    /// Unexpected space before character.
    #[error("Unexpected space before \"{0}\"{1}")]
    UnexpectedSpaceBefore(String, String),

    /// Expected space after character.
    #[error("Expected space after \"{0}\"{1}")]
    ExpectedSpaceAfter(String, String),

    /// No value found in document.
    #[error("No value found in document{0}")]
    NoValueFound(String),

    /// Unexpected space in number.
    #[error("Unexpected space in number{0}")]
    UnexpectedSpaceInNumber(String),

    /// Invalid key character.
    #[error("Invalid key character{0}")]
    InvalidKeyChar(String),

    /// Expected newline after block leader in property.
    #[error("Expected newline after block leader in property")]
    ExpectedNewlineAfterBlockLeader,

    /// Expected hex or comment in hex block.
    #[error("Expected hex or comment in hex block")]
    ExpectedHexInBlock,

    /// Expected value after property.
    #[error("Expected value after property{0}")]
    ExpectedValueAfterProperty(String),

    /// Generic parse error.
    #[error("{0}")]
    Generic(String),
}

impl ParseError {
    /// Create an error with location information.
    pub fn with_location(self, ctx: &ParseContext, line: usize, col: usize) -> Self {
        let suffix = ctx.loc_suffix(line, col);
        match self {
            ParseError::IllegalBom(_) => ParseError::IllegalBom(suffix),
            ParseError::IllegalSurrogate(_) => ParseError::IllegalSurrogate(suffix),
            ParseError::ForbiddenCodePoint(cp, _) => ParseError::ForbiddenCodePoint(cp, suffix),
            ParseError::TabNotAllowed(_) => ParseError::TabNotAllowed(suffix),
            ParseError::TrailingSpace(_) => ParseError::TrailingSpace(suffix),
            ParseError::LeadingSpace(_) => ParseError::LeadingSpace(suffix),
            ParseError::UnexpectedIndent(_) => ParseError::UnexpectedIndent(suffix),
            ParseError::UnexpectedChar(c, _) => ParseError::UnexpectedChar(c, suffix),
            ParseError::UnterminatedString(_) => ParseError::UnterminatedString(suffix),
            ParseError::BadCharInString(_) => ParseError::BadCharInString(suffix),
            ParseError::BadEscapedChar(_) => ParseError::BadEscapedChar(suffix),
            ParseError::BadUnicodeEscape(_) => ParseError::BadUnicodeEscape(suffix),
            ParseError::UnicodeOutOfRange(_) => ParseError::UnicodeOutOfRange(suffix),
            ParseError::OddHexDigits(_) => ParseError::OddHexDigits(suffix),
            ParseError::InvalidHexDigit(_) => ParseError::InvalidHexDigit(suffix),
            ParseError::UppercaseHex(_) => ParseError::UppercaseHex(suffix),
            ParseError::UppercaseExponent(_) => ParseError::UppercaseExponent(suffix),
            ParseError::UnexpectedNewline(kind, _) => ParseError::UnexpectedNewline(kind, suffix),
            ParseError::ExtraContent(_) => ParseError::ExtraContent(suffix),
            ParseError::InvalidNumber(_) => ParseError::InvalidNumber(suffix),
            ParseError::ExpectedColon(_) => ParseError::ExpectedColon(suffix),
            ParseError::InvalidKey(_) => ParseError::InvalidKey(suffix),
            ParseError::UnmatchedBracket(_) => ParseError::UnmatchedBracket(suffix),
            ParseError::UnmatchedBrace(_) => ParseError::UnmatchedBrace(suffix),
            ParseError::UnmatchedAngle(_) => ParseError::UnmatchedAngle(suffix),
            ParseError::UnexpectedSpaceAfter(c, _) => ParseError::UnexpectedSpaceAfter(c, suffix),
            ParseError::UnexpectedSpaceBefore(c, _) => ParseError::UnexpectedSpaceBefore(c, suffix),
            ParseError::ExpectedSpaceAfter(c, _) => ParseError::ExpectedSpaceAfter(c, suffix),
            ParseError::NoValueFound(_) => ParseError::NoValueFound(suffix),
            ParseError::UnexpectedSpaceInNumber(_) => ParseError::UnexpectedSpaceInNumber(suffix),
            ParseError::InvalidKeyChar(_) => ParseError::InvalidKeyChar(suffix),
            ParseError::ExpectedNewlineAfterBlockLeader => {
                ParseError::ExpectedNewlineAfterBlockLeader
            }
            ParseError::ExpectedHexInBlock => ParseError::ExpectedHexInBlock,
            ParseError::ExpectedValueAfterProperty(_) => {
                ParseError::ExpectedValueAfterProperty(suffix)
            }
            ParseError::Generic(msg) => ParseError::Generic(format!("{}{}", msg, suffix)),
        }
    }
}
