//! Read-only human-written TOML domain documents (contract 004, phase 1).
//!
//! The file holds the domain value only — no envelope. An optional top-level
//! `schema_version` integer names the document version; when it is missing the
//! document is at the domain's current version. The key is reserved and never
//! reaches the domain's value model. Longhorn maps the document to the JSON
//! value model, validates and migrates it in memory, and never writes the
//! file. Parse, schema-version, and type-mapping refusals carry a one-based
//! line and column.

use toml::Spanned;
use toml::de::{DeInteger, DeTable, DeValue};

use super::store::load::{migrate_from, read_source, validated_default};
use crate::store::{LoadOutcome, RecoveryKind, RecoveryState, SourceDocument, SourcePosition};
use crate::{ConfigDomain, ResolvedFile};
use longhorn_core::SchemaVersion;
use serde_json::Value;

/// The reserved top-level key naming the document schema version.
const SCHEMA_VERSION_KEY: &str = "schema_version";

/// A refused document location: detail plus the exact byte span.
struct SourceRefusal {
    detail: String,
    span: std::ops::Range<usize>,
}

pub(crate) fn load_toml_file<D: ConfigDomain>(
    domain: &D,
    file: &ResolvedFile,
) -> LoadOutcome<D::Value> {
    let source = match read_source(file) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return validated_default(domain, Some(file.full_path().to_path_buf()), true);
        }
        Err(error) => {
            return LoadOutcome::Recovery(RecoveryState {
                kind: RecoveryKind::ReadFailed,
                path: Some(file.full_path().to_path_buf()),
                source: None,
                detail: error.to_string(),
                position: None,
            });
        }
    };

    load_toml_source(domain, source)
}

fn load_toml_source<D: ConfigDomain>(domain: &D, source: SourceDocument) -> LoadOutcome<D::Value> {
    // Own the text so recovery states can keep the source bytes.
    let text = match std::str::from_utf8(&source.bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => {
            return LoadOutcome::Recovery(RecoveryState {
                kind: RecoveryKind::CorruptDocument,
                path: Some(source.path.clone()),
                source: Some(source),
                detail: "domain file is not valid UTF-8".to_owned(),
                position: None,
            });
        }
    };

    let root = match DeTable::parse(&text) {
        Ok(root) => root,
        Err(error) => {
            return LoadOutcome::Recovery(RecoveryState {
                kind: RecoveryKind::CorruptDocument,
                path: Some(source.path.clone()),
                source: Some(source),
                detail: format!("TOML parse error: {}", error.message()),
                position: error.span().map(|span| position_of(&text, span.start)),
            });
        }
    };

    let current_version = domain.descriptor().schema_version();
    let version = match declared_version(root.get_ref()) {
        Ok(Some(version)) => version,
        Ok(None) => current_version,
        Err(refusal) => {
            return refused::<D>(source, refusal, &text, RecoveryKind::CorruptDocument);
        }
    };

    if version > current_version {
        return LoadOutcome::Recovery(RecoveryState {
            kind: RecoveryKind::FutureSchema,
            path: Some(source.path.clone()),
            source: Some(source),
            detail: format!("schema {version} is newer than supported schema {current_version}"),
            position: declared_version_span(root.get_ref())
                .map(|span| position_of(&text, span.start)),
        });
    }

    let value = match map_document(root.get_ref()) {
        Ok(value) => value,
        Err(refusal) => {
            return refused::<D>(source, refusal, &text, RecoveryKind::CorruptDocument);
        }
    };

    // Domain issues on the file's own content report the document start;
    // only parse, schema-version, and type-mapping refusals pinpoint a span.
    migrate_from(
        domain,
        source,
        version,
        value,
        Some(SourcePosition { line: 1, column: 1 }),
    )
}

/// Converts one refused span into a typed recovery state that keeps the
/// source bytes and names the offending line and column.
fn refused<D: ConfigDomain>(
    source: SourceDocument,
    refusal: SourceRefusal,
    text: &str,
    kind: RecoveryKind,
) -> LoadOutcome<D::Value> {
    LoadOutcome::Recovery(RecoveryState {
        kind,
        path: Some(source.path.clone()),
        source: Some(source),
        detail: refusal.detail,
        position: Some(position_of(text, refusal.span.start)),
    })
}

/// Reads the reserved `schema_version` key. `Ok(None)` means the key is
/// absent and the document is at the domain's current version.
fn declared_version(root: &DeTable<'_>) -> Result<Option<SchemaVersion>, SourceRefusal> {
    let Some((_, declared)) = root
        .iter()
        .find(|(key, _)| key.get_ref().as_ref() == SCHEMA_VERSION_KEY)
    else {
        return Ok(None);
    };
    let span = declared.span();
    let refusal = |detail: String| SourceRefusal {
        detail,
        span: span.clone(),
    };

    let DeValue::Integer(raw) = declared.get_ref() else {
        return Err(refusal("schema_version must be a TOML integer".to_owned()));
    };
    let parsed = parse_integer(raw).ok_or_else(|| {
        refusal("schema_version is outside the supported version range".to_owned())
    })?;
    let version = SchemaVersion::new(u32::try_from(parsed).map_err(|_| {
        refusal("schema_version is outside the supported version range".to_owned())
    })?)
    .map_err(|_| refusal("schema_version must be a positive integer".to_owned()))?;
    Ok(Some(version))
}

fn declared_version_span(root: &DeTable<'_>) -> Option<std::ops::Range<usize>> {
    root.iter()
        .find(|(key, _)| key.get_ref().as_ref() == SCHEMA_VERSION_KEY)
        .map(|(_, value)| value.span())
}

/// Maps every entry except the reserved version key to the JSON value model.
fn map_document(root: &DeTable<'_>) -> Result<Value, SourceRefusal> {
    let mut mapped = serde_json::Map::new();
    for (key, value) in root.iter() {
        if key.get_ref().as_ref() == SCHEMA_VERSION_KEY {
            continue;
        }
        mapped.insert(key.get_ref().as_ref().to_owned(), map_value(value)?);
    }
    Ok(Value::Object(mapped))
}

/// Maps one spanned TOML value to the JSON value model.
fn map_value(value: &Spanned<DeValue<'_>>) -> Result<Value, SourceRefusal> {
    let span = value.span();
    let refusal = |detail: String| SourceRefusal {
        detail,
        span: span.clone(),
    };

    match value.get_ref() {
        DeValue::String(raw) => Ok(Value::String(raw.as_ref().to_owned())),
        DeValue::Boolean(raw) => Ok(Value::Bool(*raw)),
        DeValue::Integer(raw) => match parse_integer(raw) {
            Some(parsed) => Ok(Value::Number(serde_json::Number::from(parsed))),
            None => Err(refusal(format!(
                "integer {} is outside the 64-bit signed range TOML and JSON share",
                raw.as_str()
            ))),
        },
        DeValue::Float(raw) => {
            let normalized = raw.as_str().replace('_', "");
            match normalized.parse::<f64>() {
                Ok(parsed) if parsed.is_finite() => Ok(serde_json::Number::from_f64(parsed)
                    .map_or_else(
                        || Err(refusal("float is not representable in JSON".to_owned())),
                        |number| Ok(Value::Number(number)),
                    )?),
                Ok(_) => Err(refusal(format!(
                    "float {normalized} is not finite; TOML nan and inf have no JSON number"
                ))),
                Err(_) => Err(refusal(format!("float {normalized} is not representable"))),
            }
        }
        DeValue::Datetime(raw) => {
            if raw.offset.is_some() {
                Ok(Value::String(raw.to_string()))
            } else {
                let kind = match (raw.date.is_some(), raw.time.is_some()) {
                    (true, true) => "local date-time",
                    (true, false) => "local date",
                    (false, _) => "local time",
                };
                Err(refusal(format!(
                    "{kind} has no UTC offset; use an offset date-time, which maps to an RFC 3339 string"
                )))
            }
        }
        DeValue::Array(items) => {
            let mut mapped = Vec::with_capacity(items.len());
            for item in items.iter() {
                mapped.push(map_value(item)?);
            }
            Ok(Value::Array(mapped))
        }
        DeValue::Table(table) => {
            let mut mapped = serde_json::Map::new();
            for (key, item) in table.iter() {
                mapped.insert(key.get_ref().as_ref().to_owned(), map_value(item)?);
            }
            Ok(Value::Object(mapped))
        }
    }
}

/// Parses a TOML integer literal in any radix within the 64-bit signed range
/// both formats share.
fn parse_integer(raw: &DeInteger<'_>) -> Option<i64> {
    let literal = raw.as_str();
    let (negative, digits) = match literal.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, literal.strip_prefix('+').unwrap_or(literal)),
    };
    let magnitude = i128::from_str_radix(&digits.replace('_', ""), raw.radix()).ok()?;
    let signed = if negative { -magnitude } else { magnitude };
    i64::try_from(signed).ok()
}

/// Converts a zero-based byte offset to a one-based line and character column.
fn position_of(text: &str, offset: usize) -> SourcePosition {
    let offset = offset.min(text.len());
    let prefix = &text[..offset];
    let line = prefix.matches('\n').count() + 1;
    let column = match prefix.rfind('\n') {
        Some(line_start) => prefix[line_start + 1..].chars().count() + 1,
        None => prefix.chars().count() + 1,
    };
    SourcePosition {
        line: line as u32,
        column: column as u32,
    }
}
