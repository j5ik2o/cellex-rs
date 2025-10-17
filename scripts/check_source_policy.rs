//! ```cargo
//! [dependencies]
//! walkdir = "2"
//! glob = "0.3"
//! anyhow = "1"
//! regex = "1"
//! syn = { version = "2", features = ["full", "parsing"] }
//! toml = "0.8"
//! ```
use anyhow::{anyhow, Context, Result};
use glob::glob;
use regex::Regex;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env,
    fs,
    path::{Path, PathBuf},
};
use toml::Value;
use walkdir::WalkDir;

#[derive(Clone)]
struct TypeInfo {
    kind: &'static str,
    name: String,
}

struct TypeCollection {
    items: Vec<TypeInfo>,
    has_alias_or_union: bool,
}

struct MultiTypeViolation {
    path: PathBuf,
    items: Vec<TypeInfo>,
    has_alias_or_union: bool,
}

const RESERVED_FILE_BASENAMES: &[&str] = &["core", "common"];

fn read_toml(path: &Path) -> Result<Value> {
    let s = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    Ok(s.parse::<Value>()?)
}

fn is_dir_exists(p: &Path) -> bool {
    p.exists() && p.is_dir()
}

fn is_file_exists(p: &Path) -> bool {
    p.exists() && p.is_file()
}

fn collect_source_dirs(root: &Path) -> Result<Vec<PathBuf>> {
    let root_toml = root.join("Cargo.toml");
    let doc = read_toml(&root_toml).with_context(|| "failed to parse top-level Cargo.toml")?;

    let include_tests = env::var("ONE_TYPE_INCLUDE_TESTS").ok().as_deref() == Some("1");

    let mut exclude_patterns: Vec<String> = Vec::new();
    if let Some(ws) = doc.get("workspace") {
        if let Some(ex) = ws.get("exclude") {
            if let Some(arr) = ex.as_array() {
                for v in arr.iter().filter_map(|v| v.as_str()) {
                    exclude_patterns.push(v.to_string());
                }
            }
        }
    }

    let excluded = |p: &Path| -> bool {
        let s = p.to_string_lossy().to_string();
        exclude_patterns.iter().any(|pat| s.contains(pat))
    };

    let mut crate_dirs: HashSet<PathBuf> = HashSet::new();
    if let Some(ws) = doc.get("workspace") {
        if let Some(members) = ws.get("members") {
            if let Some(arr) = members.as_array() {
                for pat in arr.iter().filter_map(|v| v.as_str()) {
                    let pattern = root.join(pat).to_string_lossy().to_string();
                    for entry in glob(&pattern)? {
                        let path = entry?;
                        let dir = if path.is_file() {
                            path.parent().unwrap().to_path_buf()
                        } else {
                            path.clone()
                        };
                        if excluded(&dir) {
                            continue;
                        }
                        if is_file_exists(&dir.join("Cargo.toml")) {
                            crate_dirs.insert(dir);
                        }
                    }
                }
            }
        }
    }

    if doc.get("package").is_some() && !excluded(root) {
        crate_dirs.insert(root.to_path_buf());
    }

    let mut dirs: Vec<PathBuf> = Vec::new();
    for cd in crate_dirs {
        let src = cd.join("src");
        if is_dir_exists(&src) {
            dirs.push(src);
        }
        if include_tests {
            for extra in ["tests", "benches", "examples"] {
                let p = cd.join(extra);
                if is_dir_exists(&p) {
                    dirs.push(p);
                }
            }
        }
    }

    dirs.sort();
    dirs.dedup();
    Ok(dirs)
}

fn has_allow_comment(src: &str) -> bool {
    src.lines().take(80).any(|l| l.contains("allow:multi-types"))
}

fn is_exception_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|s| s.to_str()),
        Some("lib.rs")
            | Some("main.rs")
            | Some("build.rs")
            | Some("tests.rs")
            | Some("core.rs")
            | Some("common.rs")
    )
}

fn collect_top_level_types(src: &str) -> Result<TypeCollection> {
    let file = syn::parse_file(src)?;
    use syn::Item::*;
    let mut items: Vec<TypeInfo> = Vec::new();
    let mut has_alias_or_union = false;
    let mut seen: HashSet<String> = HashSet::new();
    for item in file.items {
        match item {
            Struct(data) => {
                let name = data.ident.to_string();
                let key = format!("struct:{name}");
                if seen.insert(key) {
                    items.push(TypeInfo { kind: "struct", name });
                }
            }
            Enum(data) => {
                let name = data.ident.to_string();
                let key = format!("enum:{name}");
                if seen.insert(key) {
                    items.push(TypeInfo { kind: "enum", name });
                }
            }
            Trait(data) => {
                let name = data.ident.to_string();
                let key = format!("trait:{name}");
                if seen.insert(key) {
                    items.push(TypeInfo { kind: "trait", name });
                }
            }
            Type(_) | Union(_) => has_alias_or_union = true,
            _ => {}
        }
    }
    Ok(TypeCollection {
        items,
        has_alias_or_union,
    })
}

fn main() -> Result<()> {
    let root = PathBuf::from(".");
    let mut dirs = collect_source_dirs(&root)?;

    let filters = parse_filters();
    let mut label_cache: HashMap<PathBuf, Vec<String>> = HashMap::new();
    if !filters.is_empty() {
        let joined = filters.join(", ");
        dirs.retain(|dir| {
            filters
                .iter()
                .any(|filter| matches_filter(dir, filter, &root, &mut label_cache))
        });
        if dirs.is_empty() {
            return Err(anyhow!(
                "指定されたフィルターに一致するモジュールがありません: {}",
                joined
            ));
        }
        println!("[INFO] フィルターを適用: {joined}");
    }

    if dirs.is_empty() {
        println!("[WARN] no source dirs detected from workspace; nothing to check");
        return Ok(());
    }

    let re_modrs = Regex::new(r"(^|/|\\)mod\\.rs$").unwrap();
    let mut errors: Vec<(String, String)> = Vec::new();
    let mut multi_type_violations: Vec<MultiTypeViolation> = Vec::new();

    for dir in &dirs {
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }

            if is_exception_file(path) {
                continue;
            }

            if re_modrs.is_match(&path.to_string_lossy()) {
                let module = path
                    .parent()
                    .unwrap_or(Path::new("."))
                    .display()
                    .to_string();
                errors.push((module, format!("mod.rs is not allowed: {}", path.display())));
                continue;
            }

            let src = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            if has_allow_comment(&src) {
                continue;
            }

            match collect_top_level_types(&src) {
                Ok(collection) => {
                    if collection.items.len() > 1 {
                        multi_type_violations.push(MultiTypeViolation {
                            path: path.to_path_buf(),
                            items: collection.items,
                            has_alias_or_union: collection.has_alias_or_union,
                        });
                    }
                }
                Err(e) => {
                    let module = path
                        .parent()
                        .unwrap_or(Path::new("."))
                        .display()
                        .to_string();
                    errors.push((module, format!("Parse error in {}: {e}", path.display())));
                }
            }
        }
    }

    if errors.is_empty() && multi_type_violations.is_empty() {
        println!("[OK] source policy passed on {} dirs:", dirs.len());
        for d in dirs {
            println!("  - {}", d.display());
        }
        Ok(())
    } else {
        report_violations(errors, multi_type_violations)
    }
}

fn report_violations(
    errors: Vec<(String, String)>,
    multi_type_violations: Vec<MultiTypeViolation>,
) -> Result<()> {
    const MAX_MODULES: usize = 20;
    const MAX_WARNINGS_PER_MODULE: usize = 5;

    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (module, message) in errors {
        grouped.entry(module).or_default().push(message);
    }

    for violation in multi_type_violations {
        let module = violation
            .path
            .parent()
            .unwrap_or(Path::new("."))
            .display()
            .to_string();

        let entry = grouped.entry(module).or_default();
        entry.push(format!(
            "{} has {} top-level type/trait definitions",
            violation.path.display(),
            violation.items.len()
        ));

        let max_items = 3usize.min(violation.items.len());
        for item in violation.items.iter().take(max_items) {
            let suggestion = format_suggestion(&violation.path, item);
            entry.push(format!("    - {} {} -> {}", item.kind, item.name, suggestion));
        }
        if violation.items.len() > max_items {
            entry.push(format!(
                "    ... {} additional type(s) omitted",
                violation.items.len() - max_items
            ));
        }
        let dir = violation.path.parent().unwrap_or(Path::new("."));
        let core_path = dir.join("core.rs");
        if violation
            .path
            .file_name()
            .and_then(|s| s.to_str())
            == Some("core.rs")
        {
            entry.push(format!(
                "    > After extraction: keep {} for module wiring only (`pub mod ...; pub use ...`).",
                violation.path.display()
            ));
        } else if core_path.exists() {
            entry.push(format!(
                "    > After extraction: migrate module wiring from {} into existing {} (use `pub mod ...; pub use ...`).",
                violation.path.display(),
                core_path.display()
            ));
        } else {
            entry.push(format!(
                "    > After extraction: rename {} to {} and keep only module wiring (`pub mod ...; pub use ...`).",
                violation.path.display(),
                core_path.display()
            ));
        }
        if violation.has_alias_or_union {
            let common_path = dir.join("common.rs");
            entry.push(format!(
                "    > Remaining type aliases or unions? move them into {}.",
                common_path.display()
            ));
        }
    }

    eprintln!("source policy violations:");
    for (idx, (module, messages)) in grouped.iter().enumerate() {
        if idx >= MAX_MODULES {
            let remaining = grouped.len().saturating_sub(MAX_MODULES);
            if remaining > 0 {
                eprintln!("  ... {} more module(s) containing violations (output truncated)", remaining);
            }
            break;
        }

        eprintln!("  module: {} ({} violation(s))", module, messages.len());
        for (i, message) in messages.iter().enumerate() {
            if i >= MAX_WARNINGS_PER_MODULE {
                let remaining = messages.len().saturating_sub(MAX_WARNINGS_PER_MODULE);
                if remaining > 0 {
                    eprintln!("    ... {} additional violation(s) in this module", remaining);
                }
                break;
            }
            eprintln!("    - {message}");
        }
    }
    Err(anyhow!("violations found"))
}

fn parse_filters() -> Vec<String> {
    env::var("CARGO_MAKE_TASK_ARGS")
        .ok()
        .map(|args| {
            args.split(|c: char| c.is_whitespace() || c == ';')
                .filter(|s| {
                    let trimmed = s.trim();
                    !trimmed.is_empty() && trimmed != "--"
                })
                .map(|s| s.trim().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn matches_filter(
    dir: &Path,
    filter: &str,
    root: &Path,
    cache: &mut HashMap<PathBuf, Vec<String>>,
) -> bool {
    let normalized = filter.trim().trim_matches('/');
    if normalized.is_empty() {
        return false;
    }

    let labels = cache
        .entry(dir.to_path_buf())
        .or_insert_with(|| collect_labels(dir, root));

    labels.iter().any(|label| {
        let label_norm = label.trim_matches('/');
        label_norm == normalized
            || label_norm.ends_with(&format!("/{normalized}"))
            || label_norm.contains(normalized)
    })
}

fn collect_labels(dir: &Path, root: &Path) -> Vec<String> {
    fn push_unique(buf: &mut Vec<String>, value: String) {
        if !value.is_empty() && !buf.iter().any(|existing| existing == &value) {
            buf.push(value);
        }
    }

    let mut labels: Vec<String> = Vec::new();

    if let Ok(rel_dir) = dir.strip_prefix(root) {
        let rel_str = rel_dir.to_string_lossy().replace('\\', "/");
        push_unique(&mut labels, rel_str.clone());
        if let Some(stripped) = rel_str.strip_suffix("/src") {
            push_unique(&mut labels, stripped.to_string());
        }
    }

    if let Some(parent) = dir.parent() {
        if let Some(name) = parent.file_name().and_then(|s| s.to_str()) {
            push_unique(&mut labels, name.to_string());
        }

        if let Ok(rel_parent) = parent.strip_prefix(root) {
            let rel_parent_str = rel_parent.to_string_lossy().replace('\\', "/");
            push_unique(&mut labels, rel_parent_str.clone());
            if let Some(stripped) = rel_parent_str.strip_suffix("/src") {
                push_unique(&mut labels, stripped.to_string());
            }
        }

        let cargo_path = parent.join("Cargo.toml");
        if cargo_path.exists() {
            if let Ok(doc) = read_toml(&cargo_path) {
                if let Some(pkg) = doc
                    .get("package")
                    .and_then(|pkg| pkg.get("name"))
                    .and_then(|name| name.as_str())
                {
                    push_unique(&mut labels, pkg.to_string());
                }
            }
        }
    }

    labels
}

fn format_suggestion(path: &Path, item: &TypeInfo) -> String {
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut snake = to_snake_case(&item.name);
    if snake.is_empty() {
        snake = String::from("generated_type");
    }
    if RESERVED_FILE_BASENAMES.contains(&snake.as_str()) {
        snake.push_str("_type");
    }
    let file_name = format!("{}.rs", snake);
    let target = dir.join(&file_name);
    if target == path {
        format!("create {}", target.display())
    } else if target.exists() {
        format!("merge into existing {}", target.display())
    } else {
        format!("create {}", target.display())
    }
}

fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    let mut chars = name.chars().peekable();
    let mut prev_is_upper = false;
    let mut prev_is_underscore = false;
    while let Some(ch) = chars.next() {
        if ch.is_uppercase() {
            let next_is_lower = chars.peek().map(|c| c.is_lowercase()).unwrap_or(false);
            if !result.is_empty() && !prev_is_underscore && (!prev_is_upper || next_is_lower) {
                result.push('_');
            }
            for lower in ch.to_lowercase() {
                result.push(lower);
            }
            prev_is_upper = true;
            prev_is_underscore = false;
        } else if ch == '-' || ch == ' ' {
            if !result.ends_with('_') {
                result.push('_');
            }
            prev_is_upper = false;
            prev_is_underscore = true;
        } else {
            result.push(ch.to_ascii_lowercase());
            prev_is_upper = false;
            prev_is_underscore = ch == '_';
        }
    }
    result.trim_matches('_').to_string()
}
