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
