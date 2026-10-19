//! Redact-then-bound text helpers.

use longhorn_redact::Redactor;

/// The marker that opens a tail whose earlier output was cut.
pub const OMITTED_MARKER: &str = "[earlier output omitted]\n";

/// The bounds [`bounded_tail`] keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TailBounds {
    /// Most lines kept, counted before the byte cut.
    pub lines: usize,
    /// Most bytes returned, the marker included.
    pub bytes: usize,
}

/// Redacts `text`, then keeps its first `max_bytes` bytes, cut back to a
/// character boundary.
#[must_use]
pub fn bound_field(redactor: &Redactor, text: &str, max_bytes: usize) -> String {
    let redacted = redactor.redact(text).text;
    clip(&redacted, max_bytes).to_owned()
}

/// Redacts `text`, then keeps its newest output: the last `bounds.lines`
/// lines joined by `\n`, then, when that is over `bounds.bytes`, the newest
/// bytes after [`OMITTED_MARKER`]. The cut moves forward to a character
/// boundary. A byte bound smaller than the marker returns an empty string.
#[must_use]
pub fn bounded_tail(redactor: &Redactor, text: &str, bounds: TailBounds) -> String {
    let redacted = redactor.redact(text).text;
    let lines = redacted.lines().collect::<Vec<_>>();
    let tail = lines[lines.len().saturating_sub(bounds.lines)..].join("\n");
    if tail.len() <= bounds.bytes {
        return tail;
    }
    let Some(budget) = bounds.bytes.checked_sub(OMITTED_MARKER.len()) else {
        return String::new();
    };
    let mut start = tail.len() - budget;
    while !tail.is_char_boundary(start) {
        start += 1;
    }
    format!("{OMITTED_MARKER}{}", &tail[start..])
}

/// The longest prefix of `text` within `max_bytes` that ends on a character
/// boundary.
pub(crate) fn clip(text: &str, max_bytes: usize) -> &str {
    let mut end = text.len().min(max_bytes);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Writes interior CR and LF as the two characters `\r` and `\n`, so a
/// field never breaks its line.
pub(crate) fn single_line(text: &str) -> String {
    text.replace('\r', "\\r").replace('\n', "\\n")
}

/// Appends `text` as a JSON string literal.
pub(crate) fn push_json_string(out: &mut String, text: &str) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if u32::from(control) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", u32::from(control)));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}
