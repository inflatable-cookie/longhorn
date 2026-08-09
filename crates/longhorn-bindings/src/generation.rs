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
