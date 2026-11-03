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
    let mut chars = s[1 + scheme..].char_indices().peekable();
    while let Some((j, c)) = chars.next() {
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
