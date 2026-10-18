//! Candidate span finders. Each rule reads only the pass's input text and
//! proposes byte ranges on character boundaries; the engine clips and merges
//! them. No rule sees another rule's output within a pass.

use std::ops::Range;

use crate::RedactionRules;

/// Pushes every occurrence of `secret`, including overlapping ones.
pub(crate) fn exact(text: &str, secret: &str, out: &mut Vec<Range<usize>>) {
    let mut from = 0;
    while let Some(offset) = text[from..].find(secret) {
        let start = from + offset;
        out.push(start..start + secret.len());
        let step = text[start..].chars().next().map_or(1, char::len_utf8);
        from = start + step;
    }
}

/// Pushes the candidates of every enabled shape rule.
pub(crate) fn shapes(text: &str, rules: &RedactionRules, out: &mut Vec<Range<usize>>) {
    let authorities = url_authorities(text);
    if rules.pem_private_keys {
        pem_private_keys(text, out);
    }
    if rules.url_userinfo {
        url_userinfo(text, &authorities, out);
    }
    if rules.bearer {
        bearer(text, out);
    }
    if !rules.assignment_keys.is_empty() {
        assignments(text, &rules.assignment_keys, &authorities, out);
    }
    if rules.jwt {
        jwt(text, out);
    }
    for prefix in &rules.token_prefixes {
        token_prefix(text, prefix, out);
    }
}

fn is_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

/// End of the run starting at `start` that stops before the first character
/// matching `stop`, or the end of the text.
fn run_end(text: &str, start: usize, stop: impl Fn(char) -> bool) -> usize {
    text[start..]
        .find(stop)
        .map_or(text.len(), |offset| start + offset)
}

fn line_end(text: &str, start: usize) -> usize {
    run_end(text, start, |character| matches!(character, '\n' | '\r'))
}

/// The authority of each `scheme://` occurrence: from after `://` to the next
/// `/`, `?`, `#` or whitespace.
fn url_authorities(text: &str) -> Vec<Range<usize>> {
    text.match_indices("://")
        .map(|(scheme, _)| {
            let start = scheme + 3;
            let end = run_end(text, start, |character| {
                matches!(character, '/' | '?' | '#') || character.is_whitespace()
            });
            start..end
        })
        .collect()
}

/// From the line holding `-----BEGIN … PRIVATE KEY` through the end of its
/// `-----END … PRIVATE KEY` line, line terminators excluded. An unclosed block
/// runs to the end of the text.
fn pem_private_keys(text: &str, out: &mut Vec<Range<usize>>) {
    let mut open: Option<usize> = None;
    let mut line_start = 0;
    while line_start <= text.len() {
        let next = text[line_start..]
            .find('\n')
            .map_or(text.len(), |offset| line_start + offset);
        let line = text[line_start..next]
            .strip_suffix('\r')
            .unwrap_or(&text[line_start..next]);
        let line_end = line_start + line.len();
        match open {
            None => {
                if let Some(after_begin) = pem_marker_end(line, "-----BEGIN ") {
                    if pem_marker_end(&line[after_begin..], "-----END ").is_some() {
                        out.push(line_start..line_end);
                    } else {
                        open = Some(line_start);
                    }
                }
            }
            Some(start) => {
                if pem_marker_end(line, "-----END ").is_some() {
                    out.push(start..line_end);
                    open = None;
                }
            }
        }
        line_start = next + 1;
    }
    if let Some(start) = open {
        out.push(start..text.len());
    }
}

/// Where `PRIVATE KEY` ends after `marker` in `line`, if both appear.
fn pem_marker_end(line: &str, marker: &str) -> Option<usize> {
    let at = line.find(marker)? + marker.len();
    let key = line[at..].find("PRIVATE KEY")?;
    Some(at + key + "PRIVATE KEY".len())
}

/// The userinfo of `scheme://userinfo@host`, only in the password form.
fn url_userinfo(text: &str, authorities: &[Range<usize>], out: &mut Vec<Range<usize>>) {
    for authority in authorities {
        let region = &text[authority.clone()];
        if let Some(at) = region.rfind('@')
            && region[..at].contains(':')
        {
            out.push(authority.start..authority.start + at);
        }
    }
}

/// After a case-insensitive `Bearer` and one space, the credential up to
/// whitespace, a quote or a comma. Credentials under four characters are
/// prose, not credentials.
fn bearer(text: &str, out: &mut Vec<Range<usize>>) {
    const SCHEME: &str = "bearer ";
    let lower = text.to_ascii_lowercase();
    for (index, _) in lower.match_indices(SCHEME) {
        let start = index + SCHEME.len();
        let end = run_end(text, start, |character| {
            character.is_whitespace() || matches!(character, '"' | '\'' | ',')
        });
        if text[start..end].chars().count() >= crate::MIN_SECRET_CHARS {
            out.push(start..end);
        }
    }
}

/// `key=value` and `key: value` for each key, matched case-insensitively
/// where the preceding character is not ASCII alphanumeric. Keys inside a URL
/// authority are left to the userinfo rule.
fn assignments(
    text: &str,
    keys: &[String],
    authorities: &[Range<usize>],
    out: &mut Vec<Range<usize>>,
) {
    let lower = text.to_ascii_lowercase();
    let bytes = text.as_bytes();
    for key in keys {
        let key = key.to_ascii_lowercase();
        if key.is_empty() {
            continue;
        }
        let is_authorization = key == "authorization";
        for (index, _) in lower.match_indices(&key) {
            if index > 0 && bytes[index - 1].is_ascii_alphanumeric() {
                continue;
            }
            if authorities
                .iter()
                .any(|authority| authority.contains(&index))
            {
                continue;
            }
            let mut cursor = skip_blanks(bytes, index + key.len());
            if !matches!(bytes.get(cursor), Some(b'=' | b':')) {
                continue;
            }
            cursor = skip_blanks(bytes, cursor + 1);
            let quote = bytes
                .get(cursor)
                .copied()
                .filter(|byte| matches!(byte, b'"' | b'\''));
            let start = cursor + usize::from(quote.is_some());
            if is_authorization && lower[start..].starts_with("bearer ") {
                continue;
            }
            let end = match quote {
                Some(quote) => run_end(text, start, |character| {
                    character == char::from(quote) || matches!(character, '\n' | '\r')
                }),
                None if is_authorization => line_end(text, start),
                None => run_end(text, start, |character| {
                    character.is_whitespace() || matches!(character, '&' | ',' | ';')
                }),
            };
            if end > start {
                out.push(start..end);
            }
        }
    }
}

fn skip_blanks(bytes: &[u8], mut cursor: usize) -> usize {
    while matches!(bytes.get(cursor), Some(b' ' | b'\t')) {
        cursor += 1;
    }
    cursor
}

/// A run starting `eyJ` that contains at least two dots, up to whitespace.
fn jwt(text: &str, out: &mut Vec<Range<usize>>) {
    let mut from = 0;
    while let Some(offset) = text[from..].find("eyJ") {
        let start = from + offset;
        let end = run_end(text, start, char::is_whitespace);
        if text[start..end].matches('.').count() >= 2 {
            out.push(start..end);
        }
        // A later `eyJ` inside this run ends at the same place with no more
        // dots, so it cannot add a span.
        from = end;
    }
}

/// A prefix at a token boundary followed by at least one body character.
fn token_prefix(text: &str, prefix: &str, out: &mut Vec<Range<usize>>) {
    if prefix.is_empty() {
        return;
    }
    let bytes = text.as_bytes();
    for (index, _) in text.match_indices(prefix) {
        if index > 0 && is_token_byte(bytes[index - 1]) {
            continue;
        }
        let body_start = index + prefix.len();
        let body_end = bytes[body_start..]
            .iter()
            .position(|byte| !is_token_byte(*byte))
            .map_or(text.len(), |offset| body_start + offset);
        if body_end > body_start {
            out.push(index..body_end);
        }
    }
}
