//! References: a URI or IRI in angle brackets, `<scheme:...>`.
//!
//! After `<`, a run of scheme characters followed by `:` makes a reference;
//! anything else is bytes. The text between the brackets is the URI,
//! verbatim: nothing is escaped, and `>` always closes it.

/// Returns the length of the scheme, colon included, if `s` (the text after
/// `<`) begins `[A-Za-z][A-Za-z0-9+.-]*:`. Uppercase letters are matched so
/// that a reader can name an uppercase scheme rather than call it bad hex.
pub(crate) fn scheme_len(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    if !bytes.first()?.is_ascii_alphabetic() {
        return None;
    }
    let mut i = 1;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b':' {
            return Some(i + 1);
        }
        if !(b.is_ascii_alphanumeric() || b == b'+' || b == b'-' || b == b'.') {
            return None;
        }
        i += 1;
    }
    None
}

/// Returns true if `s` begins with `<` and a scheme, so is a reference.
pub(crate) fn is_reference_start(s: &str) -> bool {
    s.starts_with('<') && scheme_len(&s[1..]).is_some()
}

/// Returns true if `c` may appear in a reference (RFC 3986 and RFC 3987),
/// `%` aside, which must begin a percent-encoding.
fn is_uri_char(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_alphanumeric() || "-._~:/?#[]@!$&'()*+,;=".contains(c);
    }
    let cp = c as u32;
    cp >= 0xA0 && !c.is_control() && !c.is_whitespace() && cp != 0xFFFE && cp != 0xFFFF
}

/// Why a reference was refused, and the byte offset of the fault in the
/// text given to [`scan_reference`].
#[derive(Debug)]
pub(crate) enum ReferenceError {
    /// No `>` closes it on its line.
    Unclosed,
    /// The scheme has an uppercase letter (YAY only).
    UppercaseScheme(String),
    /// A character a URI cannot hold.
    BadChar(char, usize),
    /// A `%` not followed by two hex digits.
    BadPercent(usize),
}

impl ReferenceError {
    /// The error's message, without a location.
    pub(crate) fn message(&self) -> String {
        match self {
            ReferenceError::Unclosed => "Unmatched angle bracket".to_string(),
            ReferenceError::UppercaseScheme(s) => {
                format!("Uppercase scheme \"{}\" in reference (use lowercase)", s)
            }
            ReferenceError::BadChar(' ', _) => "Unexpected space in reference".to_string(),
            ReferenceError::BadChar('\t', _) => "Tab not allowed in reference".to_string(),
            ReferenceError::BadChar(c, _) => {
                format!("Unexpected character \"{}\" in reference", c)
            }
            ReferenceError::BadPercent(_) => {
                "Bad percent-encoding in reference (use % and two hex digits)".to_string()
            }
        }
    }

    /// The byte offset of the fault, from the `<`.
    pub(crate) fn offset(&self) -> usize {
        match self {
            ReferenceError::Unclosed | ReferenceError::UppercaseScheme(_) => 0,
            ReferenceError::BadChar(_, i) | ReferenceError::BadPercent(i) => *i,
        }
    }
}

/// Reads a reference at the start of `s`, which [`is_reference_start`]
/// accepted. Returns the URI and the bytes consumed, the `>` included. With
/// `lenient` (MEH) an uppercase scheme is lowercased; otherwise refused.
pub(crate) fn scan_reference(
    s: &str,
    lenient: bool,
) -> std::result::Result<(String, usize), ReferenceError> {
    let scheme = scheme_len(&s[1..]).expect("reference start") - 1;
    let scheme_text = &s[1..1 + scheme];
    let mut uri = String::new();
    if scheme_text.bytes().any(|b| b.is_ascii_uppercase()) {
        if !lenient {
            return Err(ReferenceError::UppercaseScheme(scheme_text.to_string()));
        }
        uri.push_str(&scheme_text.to_ascii_lowercase());
    } else {
        uri.push_str(scheme_text);
    }
    for (j, c) in s[1 + scheme..].char_indices() {
        let at = 1 + scheme + j;
        if c == '>' {
            return Ok((uri, at + 1));
        }
        if c == '%' {
            let rest = &s[at + 1..];
            let ok = rest.len() >= 2 && rest.as_bytes()[..2].iter().all(u8::is_ascii_hexdigit);
            if !ok {
                return Err(ReferenceError::BadPercent(at));
            }
        } else if !is_uri_char(c) {
            if c == '\n' || c == '\r' {
                return Err(ReferenceError::Unclosed);
            }
            return Err(ReferenceError::BadChar(c, at));
        }
        uri.push(c);
    }
    Err(ReferenceError::Unclosed)
}

/// Returns the byte offset just past a reference at the start of `s`, if
/// there is one, so that comment and syntax scanners can skip its text.
pub(crate) fn skip_reference(s: &str) -> Option<usize> {
    if !is_reference_start(s) {
        return None;
    }
    s.find('>').map(|i| i + 1)
}

/// A reference: a URI or IRI that YAY can read, held as written. It
/// designates, and is never dereferenced. Two references are equal when
/// their texts are, character for character.
///
/// The only way to make one is [`Reference::new`] (or `TryFrom`), which
/// refuses text that is not a reference's by the reader's own rule, so
/// every reference can be written as YAY and YSON and read back.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Reference(String);

/// Why a string is not a reference's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidReference {
    text: String,
    reason: String,
}

impl InvalidReference {
    /// The string that was refused.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The rule it breaks.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl std::fmt::Display for InvalidReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Not a reference: {:?} ({})", self.text, self.reason)
    }
}

impl std::error::Error for InvalidReference {}

/// Why `uri` cannot be a reference's text, or `Ok` if it can: a lowercase
/// scheme, a colon, URI or IRI characters and well-formed percent-encodings,
/// with nothing after.
pub(crate) fn check_reference_text(uri: &str) -> std::result::Result<(), String> {
    let bracketed = format!("<{}>", uri);
    if !is_reference_start(&bracketed) {
        return Err("A reference needs a scheme".to_string());
    }
    match scan_reference(&bracketed, false) {
        Ok((_, n)) if n == bracketed.len() => Ok(()),
        Ok(_) => Err("Unexpected character \">\" in reference".to_string()),
        Err(ReferenceError::Unclosed) => Err("Unexpected line break in reference".to_string()),
        Err(e) => Err(e.message()),
    }
}

impl Reference {
    /// Makes a reference, refusing text that is not a reference's.
    pub fn new(uri: impl Into<String>) -> std::result::Result<Self, InvalidReference> {
        let text = uri.into();
        match check_reference_text(&text) {
            Ok(()) => Ok(Reference(text)),
            Err(reason) => Err(InvalidReference { text, reason }),
        }
    }

    /// A reference whose text a reader has already checked.
    pub(crate) fn checked(uri: String) -> Self {
        Reference(uri)
    }

    /// The reference's text, the URI or IRI.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The reference's text, as an owned string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::ops::Deref for Reference {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Reference {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Debug for Reference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{}>", self.0)
    }
}

impl TryFrom<&str> for Reference {
    type Error = InvalidReference;
    fn try_from(uri: &str) -> std::result::Result<Self, InvalidReference> {
        Reference::new(uri)
    }
}

impl TryFrom<String> for Reference {
    type Error = InvalidReference;
    fn try_from(uri: String) -> std::result::Result<Self, InvalidReference> {
        Reference::new(uri)
    }
}
