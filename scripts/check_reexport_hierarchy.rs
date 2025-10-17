//! ```cargo
//! [dependencies]
//! walkdir = "2"
//! glob = "0.3"
//! anyhow = "1"
//! syn = { version = "2", features = ["full", "parsing"] }
//! toml = "0.8"
//! ```
use anyhow::{anyhow, Context, Result};
use glob::glob;
use std::collections::{BTreeMap, HashSet};
use std::{env, fs, path::{Path, PathBuf}};
use syn::{Item, UseTree, Visibility};
use toml::Value;
use walkdir::WalkDir;

const ALLOW_TAG: &str = "allow:cross-reexport";

#[derive(Clone)]
struct UseEntry {
    path: Vec<String>,
    alias: Option<String>,
    is_glob: bool,
}

fn read_toml(path: &Path) -> Result<Value> {
    let s = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    Ok(s.parse::<Value>()?)
}

fn is_dir_exists(p: &Path) -> bool { p.exists() && p.is_dir() }
fn is_file_exists(p: &Path) -> bool { p.exists() && p.is_file() }

fn collect_source_dirs(root: &Path) -> Result<Vec<PathBuf>> {
    let root_toml = root.join("Cargo.toml");
    let doc = read_toml(&root_toml)
        .with_context(|| "failed to parse top-level Cargo.toml")?;

    let include_tests = env::var("REEXPORT_INCLUDE_TESTS").ok().as_deref() == Some("1");

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
                        let dir = if path.is_file() { path.parent().unwrap().to_path_buf() } else { path.clone() };
                        if excluded(&dir) { continue; }
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
        if is_dir_exists(&src) { dirs.push(src); }
        if include_tests {
            for extra in ["tests", "benches", "examples"] {
                let p = cd.join(extra);
                if is_dir_exists(&p) { dirs.push(p); }
            }
        }
    }

    dirs.sort();
    dirs.dedup();
    Ok(dirs)
}

fn has_allow_tag(src: &str) -> bool {
    src.lines().take(80).any(|line| line.contains(ALLOW_TAG))
}

fn is_exception_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|s| s.to_str()),
        Some("build.rs") | Some("tests.rs")
    )
}

fn is_prelude_path(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == "prelude")
        || path.file_stem().and_then(|s| s.to_str()) == Some("prelude")
}

fn gather_use_tree(tree: &UseTree, prefix: &mut Vec<String>, entries: &mut Vec<UseEntry>) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            gather_use_tree(&path.tree, prefix, entries);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let mut path = prefix.clone();
            path.push(name.ident.to_string());
            entries.push(UseEntry { path, alias: None, is_glob: false });
        }
        UseTree::Rename(rename) => {
            let mut path = prefix.clone();
            path.push(rename.ident.to_string());
            entries.push(UseEntry {
                path,
                alias: Some(rename.rename.to_string()),
                is_glob: false,
            });
        }
        UseTree::Glob(_) => {
            entries.push(UseEntry {
                path: prefix.clone(),
                alias: None,
                is_glob: true,
            });
        }
        UseTree::Group(group) => {
            for item in &group.items {
                gather_use_tree(item, prefix, entries);
            }
        }
    }
}

fn module_path_from(src_root: &Path, file: &Path) -> Option<Vec<String>> {
    let rel = file.strip_prefix(src_root).ok()?;
    let mut parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    if parts.is_empty() {
        return Some(vec![]);
    }

    let file_name = parts.pop().unwrap();
    let mut module_path = parts;

    match file_name.as_str() {
        "lib.rs" | "main.rs" => {}
        "mod.rs" => return None,
        _ => {
            let stem = Path::new(&file_name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&file_name);
            module_path.push(stem.to_string());
        }
    }

    Some(module_path)
}

fn resolve_absolute_path(module_path: &[String], segments: &[String]) -> Option<Vec<String>> {
    if segments.is_empty() {
        return None;
    }

    match segments[0].as_str() {
        "crate" => {
            if segments.len() == 1 {
                Some(vec![])
            } else {
                Some(segments[1..].to_vec())
            }
        }
        "self" => {
            let mut base = module_path.to_vec();
            base.extend_from_slice(&segments[1..]);
            Some(base)
        }
        "super" => {
            let mut base = module_path.to_vec();
            let mut idx = 0usize;
            while idx < segments.len() && segments[idx] == "super" {
                if base.is_empty() {
                    return None;
                }
                base.pop();
                idx += 1;
            }
            base.extend_from_slice(&segments[idx..]);
            Some(base)
        }
        _ => None,
    }
}

fn module_path_to_string(path: &[String]) -> String {
    if path.is_empty() {
        "crate".to_string()
    } else {
        path.join("::")
    }
}

fn format_use_entry(entry: &UseEntry) -> String {
    let mut s = if entry.path.is_empty() {
        String::from("(self)")
    } else {
        entry.path.join("::")
    };
    if let Some(alias) = &entry.alias {
        s.push_str(" as ");
        s.push_str(alias);
    }
    if entry.is_glob {
        s.push_str("::*");
    }
    s
}

fn main() -> Result<()> {
    let root = PathBuf::from(".");
    let dirs = collect_source_dirs(&root)?;
    if dirs.is_empty() {
        println!("[WARN] no source dirs detected from workspace; nothing to check");
        return Ok(());
    }

    let mut violations: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for src_dir in &dirs {
        for entry in WalkDir::new(src_dir).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() { continue; }
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }

            if is_exception_file(path) || is_prelude_path(path) {
                continue;
            }

            let module_path = match module_path_from(src_dir, path) {
                Some(mp) => mp,
                None => continue,
            };

            let src = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;

            if has_allow_tag(&src) {
                continue;
            }

            let syntax = syn::parse_file(&src)
                .with_context(|| format!("failed to parse {}", path.display()))?;

            let module_string = module_path_to_string(&module_path);

            for item in &syntax.items {
                if let Item::Use(item_use) = item {
                    if !matches!(item_use.vis, Visibility::Public(_)) {
                        continue;
                    }

                    let mut entries = Vec::new();
                    gather_use_tree(&item_use.tree, &mut Vec::new(), &mut entries);

                    for entry in entries {
                        if entry.is_glob {
                            continue;
                        }

                        let Some(abs_path) = resolve_absolute_path(&module_path, &entry.path) else {
                            continue;
                        };
                        if abs_path.is_empty() {
                            continue;
                        }

                        if abs_path.len() <= 1 {
                            continue;
                        }

                        let target_module = &abs_path[..abs_path.len() - 1];

                        let allowed = module_path.is_empty()
                            || target_module == module_path
                            || (target_module.len() > module_path.len()
                                && target_module[..module_path.len()] == module_path[..]);

                        if !allowed {
                            let target_string = module_path_to_string(target_module);
                            let use_repr = format_use_entry(&entry);
                            let mut message = format!(
                                "violating re-export `{}`: {} re-exports `{}` which is outside its subtree",
                                use_repr, module_string, target_string
                            );
                            message.push('\n');
                            message.push_str(&format!(
                                "    > Suggested fix: move the re-export into `{}` (or expose it via the descendant's api module).",
                                target_string
                            ));
                            message.push('\n');
                            message.push_str("    > If this cross-module re-export is intentional, document it and seek approval before adding `// allow:cross-reexport`.");

                            violations
                                .entry(path.display().to_string())
                                .or_default()
                                .push(message);
                        }
                    }
                }
            }
        }
    }

    if violations.is_empty() {
        println!("[OK] re-export hierarchy passed on {} dirs:", dirs.len());
        for d in dirs { println!("  - {}", d.display()); }
        Ok(())
    } else {
        const MAX_FILES: usize = 20;
        const MAX_WARNINGS_PER_FILE: usize = 5;

        eprintln!("re-export hierarchy violations:");
        for (idx, (file, messages)) in violations.iter().enumerate() {
            if idx >= MAX_FILES {
                let remaining = violations.len().saturating_sub(MAX_FILES);
                if remaining > 0 {
                    eprintln!("  ... {} more file(s) with violations (output truncated)", remaining);
                }
                break;
            }

            eprintln!("  file: {} ({} violation(s))", file, messages.len());
            for (i, message) in messages.iter().enumerate() {
                if i >= MAX_WARNINGS_PER_FILE {
                    let remaining = messages.len().saturating_sub(MAX_WARNINGS_PER_FILE);
                    if remaining > 0 {
                        eprintln!("    ... {} additional violation(s) in this file", remaining);
                    }
                    break;
                }
                eprintln!("    - {message}");
            }
        }
        Err(anyhow!("violations found"))
    }
}