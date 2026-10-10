//! Markdown with YAY front matter.
//!
//! A markdown document is read as a record with an optional `head` (the front
//! matter, a YAY value) and a `body` (everything after it, byte for byte):
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
//! reads as `{head: {created: "2026-10-10", author: "Ada Lovelace"}, body: "\n# Title\n"}`.
//! A document that does not open with a `---` line has no head. The head ends
//! at the next line that is exactly `---`. Writing emits front matter only
//! when there is a head (an absent, null or empty head writes none), so a
//! normalization pass reformats the head and leaves the body alone.

use libyay::{encode, Format, Value};
use std::collections::HashMap;

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

/// Render a document from canonical head text (YAY, ending in a newline, or
/// empty for none) and a body.
pub fn render(head: &str, body: &str) -> String {
    if head.trim().is_empty() {
        body.to_string()
    } else {
        let mut out = String::with_capacity(head.len() + body.len() + 8);
        out.push_str("---\n");
        out.push_str(head);
        if !head.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("---\n");
        out.push_str(body);
        out
    }
}

/// The record a document reads as.
pub fn to_value(head: Option<Value>, body: &str) -> Value {
    let mut map = HashMap::new();
    if let Some(h) = head {
        map.insert("head".to_string(), h);
    }
    map.insert("body".to_string(), Value::String(body.to_string()));
    Value::Object(map)
}

/// Render a record `{head?, body?}` back to markdown.
pub fn from_value(value: &Value) -> Result<String, String> {
    let map = match value {
        Value::Object(m) => m,
        _ => return Err("markdown output needs an object with head and body".to_string()),
    };
    if let Some(k) = map.keys().find(|k| *k != "head" && *k != "body") {
        return Err(format!(
            "markdown output takes only head and body, not {:?}",
            k
        ));
    }
    let body = match map.get("body") {
        None | Some(Value::Null) => "",
        Some(Value::String(s)) => s.as_str(),
        Some(_) => return Err("markdown body must be a string".to_string()),
    };
    let head = match map.get("head") {
        None | Some(Value::Null) => String::new(),
        Some(Value::Object(m)) if m.is_empty() => String::new(),
        Some(h) => encode(h, Format::Yay),
    };
    Ok(render(&head, body))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(render("a: 1\n", "b\n"), "---\na: 1\n---\nb\n");
    }
}
