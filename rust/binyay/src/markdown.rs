//! Markdown with YAY front matter.
//!
//! A markdown document is an object, `{...head, content}`: its front matter
//! holds every property but `content`, and its body is `content`, byte for
//! byte. One blank line separates the front matter from the body, and is
//! syntax, not content:
//!
//! ```text
//! ---
//! created: "2026-10-10"
//! author: "Ada Lovelace"
//! ---
//!
//! # Title
//! ```
//!
//! reads as `{created: "2026-10-10", author: "Ada Lovelace", content: "# Title\n"}`.
//!
//! The mapping is a bijection between objects and canonical documents:
//!
//! - **Writing** needs an object. When `content` is a non-empty string, the
//!   front matter is the object without `content` and the body is `content`,
//!   unless the body would follow front matter and begins with a blank line,
//!   which the separator cannot carry. Otherwise (`content` absent, empty, not
//!   a string, or one of those) the front matter is the whole object and the
//!   body is empty, so `{content: ""}` writes `content: ""` in the front
//!   matter. Front matter is written only when it has a property, or when the
//!   body would otherwise read as front matter (it starts with a `---` line),
//!   in which case the fences are empty. A body after front matter always
//!   follows exactly one blank line.
//! - **Reading** splits the front matter from the body. The front matter, if
//!   any, must be an object, and its properties become the object's. A
//!   non-empty body becomes `content`, and front matter that also has a
//!   `content` property, of any type, is refused. An empty body adds nothing.
//!
//! A document opens front matter with a `---` line at its very start, and the
//! front matter ends at the next line that is exactly `---`. After front
//! matter, the body begins at the first line that is not blank (empty, or
//! only spaces and tabs, as CommonMark has it). Strict reading (Yaydown)
//! requires exactly one empty line there before a body, and none when there
//! is no body; lenient reading (Mehdown) skips any number. A document without
//! front matter is all body, byte for byte.

use libyay::{encode, format_yay, parse, parse_with_filename, Format, Value};
use std::collections::HashMap;

/// The property that holds the body.
pub const CONTENT: &str = "content";

/// A markdown document split into its front matter text and its body.
pub struct Doc<'a> {
    /// The front matter text, without its `---` fences, if the document has any.
    pub head: Option<&'a str>,
    /// Everything after the closing fence (or the whole document).
    pub body: &'a str,
}

/// Split a markdown document at its front matter fences.
pub fn split(input: &str) -> Result<Doc<'_>, String> {
    let rest = match input.strip_prefix("---\n") {
        Some(rest) => rest,
        None if input == "---" => return Err("unterminated front matter".to_string()),
        None => {
            return Ok(Doc {
                head: None,
                body: input,
            })
        }
    };
    if let Some(body) = rest.strip_prefix("---\n") {
        return Ok(Doc {
            head: Some(""),
            body,
        });
    }
    if rest == "---" {
        return Ok(Doc {
            head: Some(""),
            body: "",
        });
    }
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line == "---\n" || line == "---" {
            return Ok(Doc {
                head: Some(&rest[..offset]),
                body: &rest[offset + line.len()..],
            });
        }
        offset += line.len();
    }
    Err("unterminated front matter: no closing `---` line".to_string())
}

/// Whether a line, with or without its newline, is blank: empty, or only
/// spaces and tabs.
fn is_blank(line: &str) -> bool {
    line.trim_end_matches('\n')
        .chars()
        .all(|c| c == ' ' || c == '\t')
}

/// The body after front matter: `rest` (everything after the closing fence)
/// without the blank lines that separate it from the front matter. Strict
/// reading requires exactly one empty line before a body and none without
/// one. `line` is the line of the file `rest` begins on, for messages.
fn after_front_matter(rest: &str, line: usize, strict: bool) -> Result<&str, String> {
    let mut body = rest;
    let mut blanks = 0;
    while !body.is_empty() {
        let end = body.find('\n').map_or(body.len(), |i| i + 1);
        if !is_blank(&body[..end]) {
            break;
        }
        if strict && &body[..end] != "\n" {
            return Err(format!(
                "line {}: the blank line after front matter must be empty",
                line + blanks
            ));
        }
        body = &body[end..];
        blanks += 1;
    }
    if strict {
        if body.is_empty() && blanks > 0 {
            return Err(format!(
                "line {}: blank line after front matter, with no content after it",
                line
            ));
        }
        if !body.is_empty() && blanks == 0 {
            return Err(format!(
                "line {}: expected a blank line between the front matter and the content",
                line
            ));
        }
        if blanks > 1 {
            return Err(format!(
                "line {}: expected one blank line between the front matter and the \
                 content, found {}",
                line, blanks
            ));
        }
    }
    Ok(body)
}

/// Whether a document that is only this body would read as front matter.
fn opens_front_matter(body: &str) -> bool {
    body.starts_with("---\n") || body == "---"
}

/// Render a document from canonical head text (YAY, ending in a newline, or
/// empty for none) and a body, which must not begin with a blank line when
/// front matter is written. Empty fences are written only when the body alone
/// would open front matter. A body after front matter follows one blank line.
pub fn render(head: &str, body: &str) -> String {
    if head.trim().is_empty() {
        if opens_front_matter(body) {
            format!("---\n---\n\n{}", body)
        } else {
            body.to_string()
        }
    } else {
        let mut out = String::with_capacity(head.len() + body.len() + 8);
        out.push_str("---\n");
        out.push_str(head);
        if !head.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("---\n");
        if !body.is_empty() {
            out.push('\n');
        }
        out.push_str(body);
        out
    }
}

/// A document as read: its front matter normalized as text, the object it
/// reads as, and its body.
pub struct Read<'a> {
    /// The front matter as canonical text, keeping key order and comments,
    /// or empty for none.
    pub head_text: String,
    /// The object the document reads as.
    pub value: Value,
    /// The body: everything after the front matter and the blank line that
    /// follows it, byte for byte.
    pub body: &'a str,
}

/// Read a markdown document: front matter as YAY (strict, or lenient MEH)
/// and the rest, after the blank line, as its body. Errors name lines of the
/// file.
pub fn read<'a>(input: &'a str, filename: Option<&str>, strict: bool) -> Result<Read<'a>, String> {
    let doc = split(input)?;
    let mut head_text = String::new();
    let mut head_value = None;
    if let Some(h) = doc.head.filter(|h| !h.trim().is_empty()) {
        // Parse with one leading blank line standing in for the opening fence,
        // so error locations are lines of the file, not of the head.
        let padded = format!("\n{}", h);
        let in_head = |e: &dyn std::fmt::Display| format!("in front matter: {}", e);
        if strict {
            parse_with_filename(&padded, filename).map_err(|e| in_head(&e))?;
        }
        // The MEH formatter keeps key order and comments.
        head_text = format_yay(&padded).map_err(|e| in_head(&e))?;
        head_value = Some(parse(&head_text).map_err(|e| in_head(&e))?);
    }
    let body = if doc.head.is_some() {
        let line = input[..input.len() - doc.body.len()].matches('\n').count() + 1;
        after_front_matter(doc.body, line, strict)?
    } else {
        doc.body
    };
    let value = to_value(head_value, body)?;
    Ok(Read {
        head_text,
        value,
        body,
    })
}

/// Read a document as the object it denotes, with strict front matter.
#[cfg(test)]
pub fn decode(input: &str) -> Result<Value, String> {
    read(input, None, true).map(|r| r.value)
}

/// The canonical form of a document as read, for `-f md -t md`: the front
/// matter reformatted, keeping key order and comments, and the body as is.
/// Front matter that holds a `content` a body can carry (with an empty body)
/// is rewritten from its value, so that `content` moves to the body.
pub fn normalize(read: &Read) -> String {
    if let Value::Object(map) = &read.value {
        if read.body.is_empty() && content_body(map).is_some() {
            return render_object(map);
        }
    }
    render(&read.head_text, read.body)
}

/// A value's kind, for messages: "a string", "null".
fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Integer(_) | Value::Float(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
        Value::Bytes(_) => "a byte array",
    }
}

/// The object a document reads as, from its front matter value, if any, and
/// its body.
pub fn to_value(head: Option<Value>, body: &str) -> Result<Value, String> {
    let mut map = match head {
        None => HashMap::new(),
        Some(Value::Object(m)) => m,
        Some(other) => {
            return Err(format!(
                "in front matter: expected an object (key: value lines), found {}",
                kind(&other)
            ))
        }
    };
    if !body.is_empty() {
        if map.contains_key(CONTENT) {
            return Err(format!(
                "{:?} is both in the front matter and the body",
                CONTENT
            ));
        }
        map.insert(CONTENT.to_string(), Value::String(body.to_string()));
    }
    Ok(Value::Object(map))
}

/// The body an object writes, if `content` is a non-empty string that a body
/// can carry: after front matter, a body cannot begin with a blank line,
/// since the blank line there is the separator, so such a `content` stays in
/// the front matter.
fn content_body(map: &HashMap<String, Value>) -> Option<&str> {
    match map.get(CONTENT) {
        Some(Value::String(s)) if !s.is_empty() => {
            let first = s.split_inclusive('\n').next().unwrap_or("");
            if map.len() > 1 && is_blank(first) {
                None
            } else {
                Some(s.as_str())
            }
        }
        _ => None,
    }
}

fn render_object(map: &HashMap<String, Value>) -> String {
    let (head, body) = match content_body(map) {
        Some(body) => {
            let mut head = map.clone();
            head.remove(CONTENT);
            (head, body)
        }
        None => (map.clone(), ""),
    };
    render(&head_text(&head), body)
}

/// Front matter for an object: one property per line, as front matter is
/// written, though canonical YAY would put a small object on one line. Each
/// property is encoded alone and the whole is then formatted, so that the
/// text is what reading the document back would normalize it to.
fn head_text(head: &HashMap<String, Value>) -> String {
    let mut keys: Vec<&String> = head.keys().collect();
    keys.sort();
    let mut text = String::new();
    for key in keys {
        let mut one = HashMap::new();
        one.insert(key.clone(), head[key].clone());
        let line = encode(&Value::Object(one), Format::Yay);
        let line = line.trim_end_matches('\n');
        // A property with a simple value encodes inline, as `{key: value}`.
        let line = match line.strip_prefix('{').and_then(|l| l.strip_suffix('}')) {
            Some(inner) => inner,
            None => line,
        };
        text.push_str(line);
        text.push('\n');
    }
    if text.is_empty() {
        return text;
    }
    format_yay(&text).unwrap_or(text)
}

/// Write an object `{...head, content}` as markdown.
pub fn from_value(value: &Value) -> Result<String, String> {
    match value {
        Value::Object(map) => Ok(render_object(map)),
        other => Err(format!(
            "markdown needs an object at the top level (front matter properties \
             and content), not {}",
            kind(other)
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;

    fn obj(pairs: &[(&str, Value)]) -> Value {
        Value::Object(
            pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        )
    }

    fn s(text: &str) -> Value {
        Value::String(text.to_string())
    }

    #[test]
    fn no_front_matter() {
        let d = split("# Title\n").unwrap();
        assert!(d.head.is_none());
        assert_eq!(d.body, "# Title\n");
    }

    #[test]
    fn front_matter() {
        let d = split("---\na: 1\n---\n\n# T\n").unwrap();
        assert_eq!(d.head, Some("a: 1\n"));
        assert_eq!(d.body, "\n# T\n");
    }

    #[test]
    fn rule_in_body_is_not_a_fence() {
        let d = split("---\na: 1\n---\nx\n---\ny\n").unwrap();
        assert_eq!(d.head, Some("a: 1\n"));
        assert_eq!(d.body, "x\n---\ny\n");
    }

    #[test]
    fn unterminated() {
        assert!(split("---\na: 1\n").is_err());
    }

    #[test]
    fn empty_head_renders_none() {
        assert_eq!(render("", "body\n"), "body\n");
        assert_eq!(render("a: 1\n", "b\n"), "---\na: 1\n---\n\nb\n");
        assert_eq!(render("a: 1\n", ""), "---\na: 1\n---\n");
    }

    #[test]
    fn empty_head_fences_a_body_that_opens_front_matter() {
        assert_eq!(render("", "---\nx\n"), "---\n---\n\n---\nx\n");
        assert_eq!(render("", "---"), "---\n---\n\n---");
        assert_eq!(render("", "----\n"), "----\n");
    }

    /// Each edge, both ways: the object writes exactly this document, and the
    /// document reads as exactly this object.
    #[test]
    fn edges() {
        let cases: Vec<(&str, Value, &str)> = vec![
            ("empty object", obj(&[]), ""),
            ("content only", obj(&[("content", s("# T\n"))]), "# T\n"),
            (
                "content absent",
                obj(&[("title", s("t"))]),
                "---\ntitle: \"t\"\n---\n",
            ),
            (
                "content empty",
                obj(&[("content", s(""))]),
                "---\ncontent: \"\"\n---\n",
            ),
            (
                "content empty beside others",
                obj(&[("content", s("")), ("title", s("t"))]),
                "---\ncontent: \"\"\ntitle: \"t\"\n---\n",
            ),
            (
                "no trailing newline",
                obj(&[("a", Value::Integer(BigInt::from(1))), ("content", s("x"))]),
                "---\na: 1\n---\n\nx",
            ),
            (
                "trailing blank lines",
                obj(&[
                    ("a", Value::Integer(BigInt::from(1))),
                    ("content", s("x\n\n")),
                ]),
                "---\na: 1\n---\n\nx\n\n",
            ),
            (
                "a leading blank line stays in the front matter",
                obj(&[
                    ("a", Value::Integer(BigInt::from(1))),
                    ("content", s("\nx\n")),
                ]),
                "---\na: 1\ncontent: \"\\nx\\n\"\n---\n",
            ),
            (
                "a leading blank line, no head",
                obj(&[("content", s("\n\nx\n"))]),
                "\n\nx\n",
            ),
            (
                "body opens with a rule, no head",
                obj(&[("content", s("---\nx\n"))]),
                "---\n---\n\n---\nx\n",
            ),
            (
                "body is a rule, no head",
                obj(&[("content", s("---"))]),
                "---\n---\n\n---",
            ),
            (
                "body opens with a rule, after a head",
                obj(&[
                    ("a", Value::Integer(BigInt::from(1))),
                    ("content", s("---\nx\n")),
                ]),
                "---\na: 1\n---\n\n---\nx\n",
            ),
            (
                "content a number",
                obj(&[("content", Value::Integer(BigInt::from(3)))]),
                "---\ncontent: 3\n---\n",
            ),
            (
                "content null",
                obj(&[("content", Value::Null)]),
                "---\ncontent: null\n---\n",
            ),
            (
                "content nested",
                obj(&[
                    ("content", s("Body.\n")),
                    ("meta", obj(&[("content", s("deep"))])),
                ]),
                "---\nmeta: {content: \"deep\"}\n---\n\nBody.\n",
            ),
        ];
        for (name, value, doc) in cases {
            assert_eq!(from_value(&value).unwrap(), doc, "write: {}", name);
            assert!(decode(doc).unwrap() == value, "read: {}", name);
            let r = read(doc, None, false).unwrap();
            assert_eq!(normalize(&r), doc, "normalize: {}", name);
        }
    }

    #[test]
    fn reading_refuses() {
        for (name, doc) in [
            ("content and body", "---\ncontent: \"x\"\n---\n\nBody.\n"),
            (
                "null content and body",
                "---\ncontent: null\n---\n\nBody.\n",
            ),
            ("list head", "---\n- \"a\"\n---\n\nBody.\n"),
            ("scalar head", "---\n\"a\"\n---\n\nBody.\n"),
        ] {
            assert!(decode(doc).is_err(), "{}", name);
        }
    }

    /// Strict reading wants exactly one empty line between front matter and a
    /// body, and none after front matter alone; lenient reading takes any
    /// number of blank lines, and normalizing writes one.
    #[test]
    fn the_blank_line_after_front_matter() {
        let one = obj(&[
            ("a", Value::Integer(BigInt::from(1))),
            ("content", s("x\n")),
        ]);
        let canonical = "---\na: 1\n---\n\nx\n";
        assert!(decode(canonical).unwrap() == one);
        for (name, doc, error) in [
            (
                "none",
                "---\na: 1\n---\nx\n",
                "line 4: expected a blank line",
            ),
            (
                "two",
                "---\na: 1\n---\n\n\nx\n",
                "line 4: expected one blank line",
            ),
            (
                "spaces",
                "---\na: 1\n---\n  \nx\n",
                "line 4: the blank line",
            ),
            (
                "spaces then empty",
                "---\na: 1\n---\n\n \nx\n",
                "line 5: the blank line",
            ),
            (
                "empty fences, none",
                "---\n---\nx\n",
                "line 3: expected a blank line",
            ),
        ] {
            let e = decode(doc)
                .err()
                .unwrap_or_else(|| panic!("{}: read", name));
            assert!(e.starts_with(error), "{}: {}", name, e);
            let r = read(doc, None, false).unwrap();
            let want = if name.starts_with("empty fences") {
                obj(&[("content", s("x\n"))])
            } else {
                one.clone()
            };
            assert!(r.value == want, "{}: lenient read", name);
            let normal = normalize(&r);
            assert!(decode(&normal).unwrap() == want, "{}: {:?}", name, normal);
        }
        assert_eq!(
            normalize(&read("---\na: 1\n---\nx\n", None, false).unwrap()),
            canonical
        );
        let head_only = obj(&[("a", Value::Integer(BigInt::from(1)))]);
        for doc in [
            "---\na: 1\n---\n\n",
            "---\na: 1\n---\n\n\n",
            "---\na: 1\n---\n ",
        ] {
            assert!(decode(doc).is_err(), "{:?}", doc);
            let r = read(doc, None, false).unwrap();
            assert!(r.value == head_only, "{:?}", doc);
            assert_eq!(normalize(&r), "---\na: 1\n---\n");
        }
        // A body that itself begins with a blank line keeps it.
        let doc = "---\na: 1\ncontent: \"\\nx\\n\"\n---\n";
        let r = read(doc, None, true).unwrap();
        assert!(
            r.value
                == obj(&[
                    ("a", Value::Integer(BigInt::from(1))),
                    ("content", s("\nx\n"))
                ])
        );
        assert_eq!(normalize(&r), doc);
    }

    #[test]
    fn reading_accepts_noncanonical() {
        // A string content in front matter, with no body, reads as content and
        // normalizes into the body.
        let doc = "---\ncontent: \"x\"\n---\n";
        assert!(decode(doc).unwrap() == obj(&[("content", s("x"))]));
        assert_eq!(normalize(&read(doc, None, false).unwrap()), "x");
        // Empty fences and a closing fence without a newline.
        assert!(decode("---\n---\n").unwrap() == obj(&[]));
        let one = obj(&[("a", Value::Integer(BigInt::from(1)))]);
        assert!(decode("---\na: 1\n---").unwrap() == one);
    }

    #[test]
    fn writing_refuses_non_objects() {
        for value in [
            Value::Array(vec![]),
            s("x"),
            Value::Null,
            Value::Integer(BigInt::from(1)),
        ] {
            assert!(from_value(&value).is_err());
        }
    }

    /// Parts of the space the generator covers that libyay could not yet
    /// carry through a document. Each is switched on by the commit that fixes
    /// what stood in its way.
    const UNICODE: bool = true;
    const WIDE_KEYS: bool = true;
    const NESTED_BLOCKS: bool = true;
    /// Still left out: a list directly in a list holds only scalars, since
    /// libyay's Rust parser reads `- - p: 1` as a scalar, not an object.
    const LISTS_IN_LISTS_OF_BLOCKS: bool = false;

    /// A small seeded generator (xorshift64*), so that failures reproduce.
    struct Gen(u64);

    impl Gen {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }

        fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
            items[self.below(items.len())]
        }

        fn text(&mut self) -> String {
            const PIECES: &[&str] = &[
                "", "a", "Prose", " ", "  ", "\t", "\n", "\n\n", "---", "---\n", "\n---\n", "----",
                "# H", "- item", "`", "``", "\"", "'", "\\", "/", ":", ": ", "#", "{", "}", "[",
                "]", "<", ">", "\u{0}", "\r\n", "null", "1",
            ];
            const UNICODE_PIECES: &[&str] = &["é", "日本", "😀", "\u{2028}"];
            let n = self.below(6);
            (0..n)
                .map(|_| {
                    if UNICODE && self.below(4) == 0 {
                        self.pick(UNICODE_PIECES)
                    } else {
                        self.pick(PIECES)
                    }
                })
                .collect()
        }

        fn key(&mut self) -> String {
            const KEYS: &[&str] = &[
                "a",
                "b",
                "title",
                "content",
                "Content",
                "body",
                "head",
                "key with space",
                "1",
                "a-b",
                "a_b",
            ];
            if WIDE_KEYS && self.below(4) == 0 {
                self.text()
            } else {
                self.pick(KEYS).to_string()
            }
        }

        /// A value; `top` is true for a front matter property's own value, and
        /// `in_list` for an item of a list.
        fn value(&mut self, depth: usize, top: bool, in_list: bool) -> Value {
            let blocks = depth > 0 && (top || NESTED_BLOCKS);
            match self.below(if blocks { 9 } else { 7 }) {
                0 => Value::Null,
                1 => Value::Bool(self.below(2) == 0),
                2 => {
                    let n = self.next() as i64 >> self.below(64);
                    Value::Integer(BigInt::from(n) * BigInt::from(self.next()))
                }
                3 => {
                    const FLOATS: &[f64] = &[0.0, -0.0, 1.5, -2.25, 1e300, 1e-300, 0.1];
                    let f = FLOATS[self.below(FLOATS.len())];
                    Value::Float(if self.below(2) == 0 {
                        f
                    } else {
                        f64::from_bits(self.next() & !(0x7ffu64 << 52) | (0x3ffu64 << 52))
                    })
                }
                4 | 5 => Value::String(self.text()),
                6 => Value::Bytes((0..self.below(4)).map(|_| self.next() as u8).collect()),
                7 => {
                    let depth = if in_list && !LISTS_IN_LISTS_OF_BLOCKS {
                        0
                    } else {
                        depth - 1
                    };
                    Value::Array(
                        (0..self.below(4))
                            .map(|_| self.value(depth, false, true))
                            .collect(),
                    )
                }
                _ => self.object(depth - 1, false),
            }
        }

        fn object(&mut self, depth: usize, top: bool) -> Value {
            let n = self.below(5);
            Value::Object(
                (0..n)
                    .map(|_| (self.key(), self.value(depth, top, false)))
                    .collect(),
            )
        }

        /// A top-level object with content absent, a string (often), or another value.
        fn document_value(&mut self) -> Value {
            let mut value = self.object(3, true);
            if let Value::Object(map) = &mut value {
                match self.below(4) {
                    0 => {
                        map.remove(CONTENT);
                    }
                    1 => {
                        let v = self.value(2, true, false);
                        map.insert(CONTENT.to_string(), v);
                    }
                    _ => {
                        let t = self.text();
                        map.insert(CONTENT.to_string(), Value::String(t));
                    }
                }
            }
            value
        }

        /// Text that may or may not be a document.
        fn document_text(&mut self) -> String {
            const LINES: &[&str] = &[
                "---\n",
                "---",
                "a: 1\n",
                "content: \"x\"\n",
                "content: null\n",
                "content: \"\"\n",
                "b: \"y\"\n",
                "- 1\n",
                "\"s\"\n",
                "\n",
                "Prose\n",
                "Prose",
                "# H\n",
                "x: `\n",
                "  block\n",
                "c: {d: 1}\n",
                "é\n",
            ];
            let n = self.below(8);
            (0..n).map(|_| self.pick(LINES)).collect()
        }
    }

    const SEED: u64 = 0x5eed_cafe_f00d_d00d;
    const RUNS: usize = 20_000;

    /// decode(encode(x)) = x for every object x, and the document it writes
    /// is canonical: it encodes back to itself and normalizes to itself.
    #[test]
    fn property_objects_round_trip() {
        let mut g = Gen(SEED);
        for i in 0..RUNS {
            let value = g.document_value();
            let doc = from_value(&value).unwrap();
            let back = decode(&doc).unwrap_or_else(|e| {
                panic!("run {}: {:?} wrote {:?}, unreadable: {}", i, value, doc, e)
            });
            assert!(
                back == value,
                "run {}: {:?} wrote {:?}, read {:?}",
                i,
                value,
                doc,
                back
            );
            assert_eq!(from_value(&back).unwrap(), doc, "run {}", i);
            assert_eq!(
                normalize(&read(&doc, None, false).unwrap()),
                doc,
                "run {}",
                i
            );
        }
    }

    /// For any text that reads, encode(decode(m)) reads as the same object, and
    /// normalizing it lands on that same canonical document.
    #[test]
    fn property_documents_round_trip() {
        let mut g = Gen(SEED ^ 1);
        let mut read_ok = 0;
        for i in 0..RUNS {
            let text = g.document_text();
            let r = match read(&text, None, false) {
                Ok(r) => r,
                Err(_) => continue,
            };
            read_ok += 1;
            let doc = from_value(&r.value).unwrap();
            assert!(
                decode(&doc).unwrap() == r.value,
                "run {}: {:?} -> {:?}",
                i,
                text,
                doc
            );
            let normal = normalize(&r);
            let again = read(&normal, None, true).unwrap_or_else(|e| {
                panic!("run {}: {:?} normalized to {:?}: {}", i, text, normal, e)
            });
            assert!(
                again.value == r.value,
                "run {}: {:?} normalized to {:?}",
                i,
                text,
                normal
            );
            assert_eq!(normalize(&again), normal, "run {}: not a fixed point", i);
        }
        assert!(
            read_ok > RUNS / 10,
            "only {} of {} generated texts read",
            read_ok,
            RUNS
        );
    }
}
