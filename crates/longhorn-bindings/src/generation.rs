use std::{
    env,
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};

/// Requested checked-artifact operation.
pub enum GenerationMode {
    /// Replace changed generated artifacts.
    Write,
    /// Fail when any checked artifact differs.
    Check,
}

/// One repository-relative generated artifact.
pub struct Artifact {
    pub relative_path: &'static str,
    pub contents: String,
}

pub fn apply(
    domain: &str,
    generate_task: &str,
    mode: GenerationMode,
    artifacts: &[Artifact],
) -> Result<(), Box<dyn Error>> {
    let root = repository_root();
    match mode {
        GenerationMode::Write => write_artifacts(&root, artifacts),
        GenerationMode::Check => check_artifacts(&root, domain, generate_task, artifacts),
    }
}

pub fn exported_declaration(declaration: String) -> String {
    declaration
        .strip_prefix("type ")
        .map_or(declaration.clone(), |body| format!("export type {body}"))
}

pub fn tagged_variants(declaration: &str, tag: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let marker = format!("\"{tag}\": \"");
    let values: Vec<_> = declaration
        .split(&marker)
        .skip(1)
        .filter_map(|suffix| suffix.split('"').next())
        .map(str::to_owned)
        .collect();
    if values.is_empty() {
        Err(io::Error::other(format!("generated declaration has no `{tag}` variants")).into())
    } else {
        Ok(values)
    }
}

pub fn string_union_variants(declaration: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let body = declaration
        .split_once('=')
        .map(|(_, body)| body)
        .ok_or_else(|| io::Error::other("generated string union has no assignment"))?;
    let values: Vec<_> = body
        .trim()
        .trim_end_matches(';')
        .split('|')
        .map(str::trim)
        .filter_map(|value| value.strip_prefix('"')?.strip_suffix('"'))
        .map(str::to_owned)
        .collect();
    if values.is_empty() {
        Err(io::Error::other("generated string union has no variants").into())
    } else {
        Ok(values)
    }
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("binding crate must remain under crates/")
        .to_path_buf()
}

fn write_artifacts(root: &Path, artifacts: &[Artifact]) -> Result<(), Box<dyn Error>> {
    for artifact in artifacts {
        let path = root.join(artifact.relative_path);
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("generated artifact has no parent"))?;
        fs::create_dir_all(parent)?;
        if fs::read_to_string(&path).ok().as_deref() != Some(artifact.contents.as_str()) {
            fs::write(&path, &artifact.contents)?;
            println!("wrote {}", artifact.relative_path);
        }
    }
    Ok(())
}

fn check_artifacts(
    root: &Path,
    domain: &str,
    generate_task: &str,
    artifacts: &[Artifact],
) -> Result<(), Box<dyn Error>> {
    let drifted: Vec<_> = artifacts
        .iter()
        .filter_map(|artifact| {
            let path = root.join(artifact.relative_path);
            (fs::read_to_string(path).ok().as_deref() != Some(artifact.contents.as_str()))
                .then_some(artifact.relative_path)
        })
        .collect();

    if drifted.is_empty() {
        println!("{domain} bindings and fixtures are current");
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "generated {domain} artifacts drifted: {}; run `effigy {generate_task}`",
            drifted.join(", ")
        ))
        .into())
    }
}

/// One generated label map: a constant, the type indexing it, and its
/// entries.
pub struct LabelMap<'a> {
    /// The exported constant's name.
    pub constant: &'a str,
    /// The type imported from `protocol.ts` to key the record.
    pub import: &'a str,
    /// The type expression used as the record's key, which may index
    /// [`Self::import`] — a tagged union's discriminant is not itself
    /// importable.
    pub key_type: &'a str,
    /// `(wire name, label or label template)` pairs, from the Rust enum that
    /// owns the wording.
    pub entries: &'a [(&'a str, &'a str)],
}

/// Renders a generated TypeScript module of label maps.
///
/// Emitting these rather than letting the TypeScript tier keep its own is what
/// stops the two backends wording the same fact differently. Previously only
/// the Rust map failed to compile when a variant was added; the TypeScript one
/// silently returned `undefined`, and a webview rendered a blank.
///
/// Each map is typed `Record<key, string>`, so a variant added to the union and
/// missing from the map is a TypeScript error at the point of use rather than
/// a surprise at runtime.
#[must_use]
pub fn label_module(task: &str, maps: &[LabelMap<'_>]) -> String {
    let mut imports: Vec<&str> = maps.iter().map(|map| map.import).collect();
    imports.sort_unstable();
    imports.dedup();

    let mut rendered = format!("// @generated by `effigy {task}`; do not edit.\n");
    rendered
        .push_str("// Wording lives in Rust, on the enum that owns it. A label edited here is\n");
    rendered.push_str("// a label that disagrees with the native surface.\n\n");
    rendered.push_str("import type { ");
    rendered.push_str(&imports.join(", "));
    rendered.push_str(" } from \"./protocol.ts\";\n");

    for map in maps {
        rendered.push_str("\nexport const ");
        rendered.push_str(map.constant);
        rendered.push_str(": Record<");
        rendered.push_str(map.key_type);
        rendered.push_str(", string> = {\n");
        for (name, label) in map.entries {
            rendered.push_str("  ");
            rendered.push_str(&json_key(name));
            rendered.push_str(": ");
            rendered.push_str(&json_string(label));
            rendered.push_str(",\n");
        }
        rendered.push_str("};\n");
    }
    rendered
}

/// Renders the shared `{name}` interpolator used by templated label maps.
///
/// One substitution rule and deliberately not a template language, mirroring
/// `longhorn_config::render_label_template` exactly. An unknown placeholder is
/// left as written, so a mistake shows as `{typo}` on screen rather than a
/// hole that reads like intentional wording.
#[must_use]
pub fn label_template_renderer(task: &str) -> String {
    const BODY: &str = r#"// Mirrors `longhorn_config::render_label_template`. One substitution rule: a
// placeholder is a name in braces, and anything else is literal. An unknown
// placeholder is left as written, so a mistake shows as `{typo}` on screen
// rather than a hole that reads like intentional wording.

export function renderLabelTemplate(
  template: string,
  fields: Readonly<Record<string, string>>,
): string {
  return template.replace(/\{([^{}]*)\}/g, (match, name: string) =>
    Object.hasOwn(fields, name) ? fields[name]! : match,
  );
}
"#;

    format!("// @generated by `effigy {task}`; do not edit.\n{BODY}")
}

fn json_key(name: &str) -> String {
    if name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
        && !name.starts_with(|character: char| character.is_ascii_digit())
    {
        name.to_owned()
    } else {
        json_string(name)
    }
}

fn json_string(value: &str) -> String {
    let mut rendered = String::with_capacity(value.len() + 2);
    rendered.push('"');
    for character in value.chars() {
        match character {
            '"' => rendered.push_str("\\\""),
            '\\' => rendered.push_str("\\\\"),
            '\n' => rendered.push_str("\\n"),
            other => rendered.push(other),
        }
    }
    rendered.push('"');
    rendered
}

/// Emits the allowed field names for every plain object type in a domain.
///
/// Contract 010's boundary target requires rejecting unknown and missing
/// fields, and the nine packages that did not already have the *shape* to do
/// it — an error type, a `record()` helper, kind checks — only lacked the list
/// of keys. This emits that list so `record()` can take it, which is a much
/// smaller step than generating whole parsers and closes the asymmetry on its
/// own.
///
/// Only plain object declarations are emitted. A tagged union's fields depend
/// on its discriminant, so one flat list would be wrong for it; unions are
/// returned as skipped rather than guessed at, and the caller decides what to
/// say about them.
#[must_use]
pub fn field_map(task: &str, constant: &str, declarations: &[String]) -> (String, Vec<String>) {
    let mut rendered = format!("// @generated by `effigy {task}`; do not edit.\n");
    rendered.push_str("// Allowed field names per protocol type, from the Rust structs.\n");
    rendered.push_str("// A `record()` given one of these rejects unknown and missing keys —\n");
    rendered.push_str("// contract 010's Boundary Validation Target.\n\n");
    rendered.push_str("export const ");
    rendered.push_str(constant);
    rendered.push_str(": Record<string, readonly string[]> = {\n");

    let mut skipped = Vec::new();
    for declaration in declarations {
        let Some((name, body)) = plain_object(declaration) else {
            if let Some(name) = tagged_union_name(declaration) {
                skipped.push(name);
            }
            continue;
        };
        let fields: Vec<String> = field_names(&body)
            .into_iter()
            .map(|f| json_string(&f))
            .collect();
        rendered.push_str("  ");
        rendered.push_str(&json_string(&name));
        rendered.push_str(": [");
        rendered.push_str(&fields.join(", "));
        rendered.push_str("],\n");
    }
    rendered.push_str("};\n");
    (rendered, skipped)
}

/// Splits `export type Name = { .. };` into its name and body.
///
/// Returns `None` for anything whose fields are not one flat set: aliases,
/// string unions, and tagged unions. The last of those is the one that has to
/// be caught deliberately — `ts-rs` renders a tagged union as several brace
/// groups joined by `|`, its braces balance, and a naive balance check lets it
/// through and produces nonsense field names.
fn plain_object(declaration: &str) -> Option<(String, String)> {
    let declaration = strip_doc_comments(declaration);
    let rest = declaration.trim().strip_prefix("export type ")?;
    let (name, remainder) = rest.split_once(" = ")?;
    let body = remainder
        .trim()
        .strip_suffix(';')?
        .trim_end()
        .strip_prefix('{')?
        .strip_suffix('}')?;

    // One flat set means one brace group. Any brace inside the body is either
    // a further object or the next arm of a union, and neither has a single
    // answer to "which keys are allowed".
    if body.contains('{') || body.contains('}') {
        return None;
    }
    Some((name.trim().to_owned(), body.to_owned()))
}

/// Removes `/** .. */` blocks, which `ts-rs` emits from Rust doc comments.
///
/// They sit between fields and contain colons and commas, so a parser that
/// does not remove them reads prose as field names.
fn strip_doc_comments(declaration: &str) -> String {
    let mut rendered = String::with_capacity(declaration.len());
    let mut rest = declaration;
    while let Some(start) = rest.find("/**") {
        rendered.push_str(&rest[..start]);
        let Some(end) = rest[start..].find("*/") else {
            break;
        };
        rest = &rest[start + end + 2..];
    }
    rendered.push_str(rest);
    rendered
}

fn tagged_union_name(declaration: &str) -> Option<String> {
    let declaration = strip_doc_comments(declaration);
    let rest = declaration.trim().strip_prefix("export type ")?;
    let (name, remainder) = rest.split_once(" = ")?;
    remainder.contains('|').then(|| name.trim().to_owned())
}

/// Field names at brace depth zero, before the first colon of each.
///
/// Depth-aware because a field's own type can be an inline object or a
/// generic, and a comma inside either is not a field separator.
fn field_names(body: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut depth = 0_i32;
    let mut current = String::new();

    for character in body.chars() {
        match character {
            '{' | '<' | '(' => {
                depth += 1;
                current.push(character);
            }
            '}' | '>' | ')' => {
                depth -= 1;
                current.push(character);
            }
            // `ts-rs` separates fields with a comma when it renders a type
            // across lines and a semicolon when it renders one inline. Both
            // appear in the same generated module, and splitting on only one
            // silently yields a short field list — which then rejects valid
            // payloads at the boundary rather than failing loudly here.
            ',' | ';' if depth == 0 => {
                if let Some(name) = leading_name(&current) {
                    fields.push(name);
                }
                current.clear();
            }
            other => current.push(other),
        }
    }
    if let Some(name) = leading_name(&current) {
        fields.push(name);
    }
    fields
}

fn leading_name(part: &str) -> Option<String> {
    let trimmed = part.trim();
    if trimmed.is_empty() {
        return None;
    }
    let name = trimmed.split_once(':')?.0.trim();
    Some(name.trim_matches('"').to_owned())
}
