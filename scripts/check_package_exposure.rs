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
use std::collections::{HashMap, HashSet};
use std::{env, fs, path::{Path, PathBuf}};
use syn::{Item, UseTree, Visibility};
use toml::Value;
use walkdir::WalkDir;

const ALLOW_TAGS: [&str; 2] = ["allow:module-expose", "allow:package-reexport"];

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

    let include_tests = env::var("PACKAGE_EXPOSE_INCLUDE_TESTS").ok().as_deref() == Some("1");

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
    src.lines()
        .take(80)
        .any(|line| ALLOW_TAGS.iter().any(|tag| line.contains(tag)))
}

fn is_exception_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|s| s.to_str()),
        Some("lib.rs") | Some("main.rs") | Some("build.rs") | Some("tests.rs") | Some("core.rs") | Some("common.rs")
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

fn format_path(parts: &[String]) -> String {
    if parts.is_empty() {
        "(self)".to_string()
    } else {
        parts.join("::")
    }
}

fn format_use_entry(entry: &UseEntry) -> String {
    let mut s = format_path(&entry.path);
    if let Some(alias) = &entry.alias {
        s.push_str(" as ");
        s.push_str(alias);
    }
    s
}

fn is_public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_) | Visibility::Restricted(_))
}

fn collect_base_path(entry: &UseEntry) -> Option<Vec<String>> {
    if entry.path.is_empty() {
        return None;
    }
    if entry.is_glob {
        Some(entry.path.clone())
    } else if entry.path.len() >= 1 {
        let mut base = entry.path.clone();
        base.pop();
        Some(base)
    } else {
        None
    }
}

fn main() -> Result<()> {
    let root = PathBuf::from(".");
    let dirs = collect_source_dirs(&root)?;
    if dirs.is_empty() {
        println!("[WARN] no source dirs detected from workspace; nothing to check");
        return Ok(());
    }

    let mut violations: HashSet<String> = HashSet::new();

    for dir in &dirs {
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() { continue; }
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }

            if is_exception_file(path) {
                continue;
            }

            let src = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;

            if has_allow_tag(&src) || is_prelude_path(path) {
                continue;
            }

            let syntax = syn::parse_file(&src)
                .with_context(|| format!("failed to parse {}", path.display()))?;

            for item in &syntax.items {
                if let Item::Use(item_use) = item {
                    if !is_public(&item_use.vis) {
                        continue;
                    }

                    let mut entries = Vec::new();
                    gather_use_tree(&item_use.tree, &mut Vec::new(), &mut entries);
                    if entries.is_empty() {
                        continue;
                    }

                    // Detect glob re-exports and deep crate paths
                    for entry in &entries {
                        if entry.is_glob {
                            if entry.path.first().map(|s| s == "crate").unwrap_or(false) {
                                let glob_str = format!("{}::*", format_path(&entry.path));
                                violations.insert(format!(
                                    "{}: wildcard re-export `{}` exposes entire module; prefer explicit package api modules",
                                    path.display(), glob_str
                                ));
                            }
                            continue;
                        }

                        if entry.path.len() > 1 && entry.path.first().map(|s| s == "crate").unwrap_or(false) {
                            let usage = format_use_entry(entry);
                            violations.insert(format!(
                                "{}: re-export `{}` publishes internal module structure; re-export it from the package-level api module instead",
                                path.display(), usage
                            ));
                        }
                    }

                    // Detect multiple deep re-exports from the same crate path
                    let mut base_map: HashMap<Vec<String>, Vec<&UseEntry>> = HashMap::new();
                    for entry in &entries {
                        if entry.is_glob {
                            continue;
                        }
                        if let Some(base) = collect_base_path(entry) {
                            if base.first().map(|s| s == "crate").unwrap_or(false) {
                                base_map.entry(base).or_default().push(entry);
                            }
                        }
                    }

                    for (base, items) in base_map {
                        if items.len() > 1 {
                            let base_str = format_path(&base);
                            let names = items
                                .iter()
                                .map(|entry| entry.alias.as_deref().unwrap_or_else(|| entry.path.last().unwrap()).to_string())
                                .collect::<Vec<_>>()
                                .join(", ");
                            violations.insert(format!(
                                "{}: multiple re-exports from `{}` ({}) detected; create a dedicated package api module instead",
                                path.display(), base_str, names
                            ));
                        }
                    }
                }
            }
        }
    }

    if violations.is_empty() {
        println!("[OK] package exposure policy passed on {} dirs:", dirs.len());
        for d in dirs { println!("  - {}", d.display()); }
        Ok(())
    } else {
        let mut sorted: Vec<_> = violations.into_iter().collect();
        sorted.sort();
        eprintln!("package exposure violations:");
        for violation in sorted {
            eprintln!("  - {violation}");
        }
        Err(anyhow!("violations found"))
    }
}