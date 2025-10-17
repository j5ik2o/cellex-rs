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
use toml::Value;
use walkdir::WalkDir;

#[derive(Clone)]
struct FileEntry {
    path: PathBuf,
    allow_scatter: bool,
}

#[derive(Hash, Eq, PartialEq, Clone)]
struct TypeKey {
    crate_root: PathBuf,
    name: String,
}

#[derive(Default)]
struct TypeData {
    defs: Vec<FileEntry>,
    impls: Vec<FileEntry>,
}

const MULTI_DEF_ALLOWLIST: &[&str] = &["Inner"];

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

fn has_allow_tag(src: &str, tag: &str) -> bool {
    src.lines().take(80).any(|l| l.contains(tag))
}

fn is_exception_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|s| s.to_str()),
        Some("lib.rs") | Some("main.rs") | Some("build.rs") | Some("tests.rs") | Some("core.rs") | Some("common.rs")
    )
}

fn dedup_entries(mut entries: Vec<FileEntry>) -> Vec<FileEntry> {
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries.dedup_by(|a, b| a.path == b.path);
    entries
}

fn find_crate_root(dir: &Path) -> Option<PathBuf> {
    let mut current = dir;
    loop {
        if is_file_exists(&current.join("Cargo.toml")) {
            return Some(current.to_path_buf());
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return None,
        }
    }
}

fn collect_items(
    items: &[syn::Item],
    map: &mut HashMap<TypeKey, TypeData>,
    file_entry: &FileEntry,
    crate_root: &Path,
) {
    use syn::Item;
    for item in items {
        match item {
            Item::Struct(item_struct) => {
                let key = TypeKey {
                    crate_root: crate_root.to_path_buf(),
                    name: item_struct.ident.to_string(),
                };
                map.entry(key).or_default().defs.push(file_entry.clone());
            }
            Item::Impl(item_impl) => {
                if item_impl.trait_.is_none() {
                    if let Some(name) = extract_type_ident(&item_impl.self_ty) {
                        let key = TypeKey {
                            crate_root: crate_root.to_path_buf(),
                            name,
                        };
                        map.entry(key).or_default().impls.push(file_entry.clone());
                    }
                }
            }
            Item::Mod(item_mod) => {
                if let Some((_, nested)) = &item_mod.content {
                    collect_items(nested, map, file_entry, crate_root);
                }
            }
            _ => {}
        }
    }
}

fn extract_type_ident(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(type_path) => {
            if type_path.qself.is_some() {
                return None;
            }
            type_path.path.segments.last().map(|seg| seg.ident.to_string())
        }
        _ => None,
    }
}

fn main() -> Result<()> {
    let root = PathBuf::from(".");
    let dirs = collect_source_dirs(&root)?;
    if dirs.is_empty() {
        println!("[WARN] no source dirs detected from workspace; nothing to check");
        return Ok(());
    }

    let mut types: HashMap<TypeKey, TypeData> = HashMap::new();

    for dir in &dirs {
        let crate_root = find_crate_root(dir).unwrap_or_else(|| dir.clone());
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
            let allow_scatter = has_allow_tag(&src, "allow:impl-scatter");

            let parsed = syn::parse_file(&src)
                .with_context(|| format!("failed to parse {}", path.display()))?;

            let file_entry = FileEntry {
                path: path.to_path_buf(),
                allow_scatter,
            };

            collect_items(&parsed.items, &mut types, &file_entry, &crate_root);
        }
    }

    let mut errors: Vec<String> = Vec::new();

    for (key, data) in types {
        let defs = dedup_entries(data.defs);
        let impls = dedup_entries(data.impls);

        if defs.is_empty() {
            continue; // likely external type
        }

        if defs.len() > 1 {
            if MULTI_DEF_ALLOWLIST.iter().any(|&name| name == key.name) {
                continue;
            }
            let locations = defs
                .iter()
                .map(|entry| entry.path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            errors.push(format!(
                "構造体 `{}` が複数のファイルで定義されています: {}",
                key.name,
                locations
            ));
            continue;
        }

        let def = &defs[0];
        if def.allow_scatter {
            continue;
        }

        for impl_entry in impls {
            if impl_entry.allow_scatter {
                continue;
            }

            if impl_entry.path != def.path {
                errors.push(format!(
                    "構造体 `{}` の定義({})とinherent impl({})が別ファイルにあります",
                    key.name,
                    def.path.display(),
                    impl_entry.path.display()
                ));
            }
        }
    }

    if errors.is_empty() {
        println!("[OK] impl co-location policy passed on {} dirs:", dirs.len());
        for d in dirs { println!("  - {}", d.display()); }
        Ok(())
    } else {
        eprintln!("impl co-location violations:");
        for err in &errors {
            eprintln!("  - {err}");
        }
        Err(anyhow!("violations found"))
    }
}