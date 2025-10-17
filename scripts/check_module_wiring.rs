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
use std::{
    collections::{BTreeMap, HashMap},
    env,
    fs,
    path::{Path, PathBuf},
};
use syn::{Attribute, Item, ItemMod, ItemUse, Meta, UseTree, Visibility};
use toml::Value;
use walkdir::WalkDir;

const ALLOW_FILE_FLAG: &str = "allow:module-wiring-skip";
const IGNORED_CHILD_MODULES: &[&str] = &["tests", "test", "bench", "benches"]; // テストやベンチモジュールは除外

#[derive(Default)]
struct ModEntry {
    is_public: bool,
    has_reexport: bool,
    is_cfg_test: bool,
    child_name: String,
}

struct Violation {
    message: String,
    suggestion: Option<String>,
}

fn main() -> Result<()> {
    let root = PathBuf::from(".");
    let mut dirs = collect_source_dirs(&root)?;

    let filters = parse_filters();
    if !filters.is_empty() {
        let mut label_cache: HashMap<PathBuf, Vec<String>> = HashMap::new();
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

    let mut reports: BTreeMap<PathBuf, Vec<Violation>> = BTreeMap::new();

    for dir in &dirs {
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            if is_exception_file(path) {
                continue;
            }

            let src = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let skip_missing = should_skip_missing_check(&src);
            let syntax = syn::parse_file(&src)
                .with_context(|| format!("failed to parse {}", path.display()))?;

            let mut mods: HashMap<String, ModEntry> = HashMap::new();
            let mut reexports: HashMap<String, Vec<String>> = HashMap::new();

            for item in &syntax.items {
                match item {
                    Item::Mod(item_mod) => {
                        handle_mod_item(path, item_mod, &mut mods)?;
                    }
                    Item::Use(item_use) => {
                        handle_use_item(item_use, &mods, &mut reexports);
                    }
                    _ => {}
                }
            }

            let mut file_violations: Vec<Violation> = Vec::new();

            for (child, paths) in &reexports {
                if let Some(entry) = mods.get_mut(child) {
                    entry.has_reexport = true;
                    if entry.is_public {
                        for path_repr in paths {
                            file_violations.push(Violation {
                                message: format!(
                                    "`pub mod {child};` と `{}` の再エクスポートが同居しています",
                                    path_repr
                                ),
                                suggestion: Some(format!(
                                    "Use `mod {child};` in the direct parent and keep only `pub use {child}::Type;`."
                                )),
                            });
                        }
                    }
                } else {
                    for path_repr in paths {
                        file_violations.push(Violation {
                            message: format!(
                                "Public re-export `{}` has no matching `mod {child};` in this file",
                                path_repr
                            ),
                            suggestion: Some(format!(
                                "Declare `mod {child};` in this parent file and keep the re-export local."
                            )),
                        });
                    }
                }
            }

            for (child, entry) in mods {
                if entry.is_cfg_test {
                    continue;
                }
                if IGNORED_CHILD_MODULES.iter().any(|name| *name == entry.child_name) {
                    continue;
                }
                if entry.is_public {
                    continue; // 公開モジュール側は既にチェック済み
                }
                if !entry.has_reexport && !skip_missing {
                    file_violations.push(Violation {
                        message: format!(
                            "`mod {}` is missing a matching `pub use {}::...;` re-export",
                            child, child
                        ),
                        suggestion: Some(format!(
                            "Add `pub use {child}::Type;` so the parent re-exports the child type. Coordinate with maintainers if this violation is intentional."
                        )),
                    });
                }
            }

            if !file_violations.is_empty() {
                reports.insert(path.to_path_buf(), file_violations);
            }
        }
    }

    if reports.is_empty() {
        println!("[OK] module wiring policy passed on {} dirs:", dirs.len());
        for d in dirs {
            println!("  - {}", d.display());
        }
        Ok(())
    } else {
        eprintln!("module wiring violations:");
        for (path, violations) in &reports {
            eprintln!(
                "  file: {} ({} violation(s))",
                path.display(),
                violations.len()
            );
            for violation in violations {
                eprintln!("    - {}", violation.message);
                if let Some(suggestion) = &violation.suggestion {
                    eprintln!("      > {}", suggestion);
                }
            }
        }
        Err(anyhow!("violations found"))
    }
}

fn handle_mod_item(
    file_path: &Path,
    item_mod: &ItemMod,
    mods: &mut HashMap<String, ModEntry>,
) -> Result<()> {
    if item_mod.content.is_some() {
        return Ok(()); // インラインモジュールは対象外
    }

    let ident = item_mod.ident.to_string();
    if IGNORED_CHILD_MODULES.iter().any(|name| *name == ident.as_str()) {
        return Ok(());
    }

    if has_cfg_test(&item_mod.attrs) {
        mods.insert(
            ident.clone(),
            ModEntry {
                is_public: !matches!(item_mod.vis, Visibility::Inherited),
                has_reexport: false,
                is_cfg_test: true,
                child_name: ident,
            },
        );
        return Ok(());
    }

    // 対象となる子ファイルが存在するか確認しておく
    if resolve_child_path(file_path, &ident).is_none() {
        return Ok(()); // 外部ファイルが存在しなければスキップ
    }

    mods.insert(
        ident.clone(),
        ModEntry {
            is_public: !matches!(item_mod.vis, Visibility::Inherited),
            has_reexport: false,
            is_cfg_test: false,
            child_name: ident,
        },
    );
    Ok(())
}

fn handle_use_item(
    item_use: &ItemUse,
    mods: &HashMap<String, ModEntry>,
    reexports: &mut HashMap<String, Vec<String>>,
) {
    if !is_public_visibility(&item_use.vis) {
        return;
    }

    let mut paths: Vec<(Vec<String>, String)> = Vec::new();
    collect_use_paths(&item_use.tree, Vec::new(), &mut paths);

    for (segments, display) in paths {
        if segments.is_empty() {
            continue;
        }
        let mut normalized = segments;
        while matches!(normalized.first().map(String::as_str), Some("self") | Some("super") | Some("crate")) {
            normalized.remove(0);
        }
        if normalized.is_empty() {
            continue;
        }
        let child = normalized[0].clone();
        if mods.contains_key(&child) {
            reexports.entry(child).or_default().push(display);
        }
    }
}

fn collect_use_paths(
    tree: &UseTree,
    base: Vec<String>,
    acc: &mut Vec<(Vec<String>, String)>,
) {
    match tree {
        UseTree::Path(path) => {
            let mut next = base;
            next.push(path.ident.to_string());
            collect_use_paths(&path.tree, next, acc);
        }
        UseTree::Name(name) => {
            let mut path = base;
            path.push(name.ident.to_string());
            let display = path.join("::");
            acc.push((path, display));
        }
        UseTree::Rename(rename) => {
            let mut path = base;
            path.push(rename.ident.to_string());
            let display = format!("{} as {}", path.join("::"), rename.rename);
            acc.push((path, display));
        }
        UseTree::Glob(_) => {
            let display = if base.is_empty() {
                String::from("*")
            } else {
                format!("{}::*", base.join("::"))
            };
            acc.push((base, display));
        }
        UseTree::Group(group) => {
            for subtree in &group.items {
                collect_use_paths(subtree, base.clone(), acc);
            }
        }
    }
}

fn is_public_visibility(vis: &Visibility) -> bool {
    !matches!(vis, Visibility::Inherited)
}

fn has_cfg_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let path_ident = attr.path().segments.first().map(|seg| seg.ident.to_string());
        match path_ident.as_deref() {
            Some("cfg") | Some("cfg_attr") => match &attr.meta {
                Meta::List(list) => list.tokens.to_string().contains("test"),
                _ => false,
            },
            _ => false,
        }
    })
}

fn should_skip_missing_check(src: &str) -> bool {
    src.lines()
        .take(80)
        .any(|line| line.contains(ALLOW_FILE_FLAG))
}

fn resolve_child_path(parent: &Path, child: &str) -> Option<PathBuf> {
    let module_dir = module_directory(parent)?;
    let candidate = module_dir.join(format!("{child}.rs"));
    if candidate.exists() {
        return Some(candidate);
    }
    let mod_candidate = module_dir.join(child).join("mod.rs");
    if mod_candidate.exists() {
        return Some(mod_candidate);
    }
    None
}

fn module_directory(file_path: &Path) -> Option<PathBuf> {
    let parent_dir = file_path.parent()?;
    let file_name = file_path.file_name()?.to_str()?;
    if matches!(file_name, "lib.rs" | "main.rs") {
        return Some(parent_dir.to_path_buf());
    }
    if let Some(stem) = file_path.file_stem() {
        return Some(parent_dir.join(stem));
    }
    Some(parent_dir.to_path_buf())
}

fn is_exception_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|s| s.to_str()),
        Some("main.rs") | Some("tests.rs") | Some("build.rs")
    )
}

fn parse_filters() -> Vec<String> {
    env::var("CARGO_MAKE_TASK_ARGS")
        .ok()
        .map(|args| {
            args
                .split(|c: char| c.is_whitespace() || c == ';')
                .filter(|s| {
                    let trimmed = s.trim();
                    !trimmed.is_empty() && trimmed != "--"
                })
                .map(|s| s.trim().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn collect_source_dirs(root: &Path) -> Result<Vec<PathBuf>> {
    let root_toml = root.join("Cargo.toml");
    let doc = read_toml(&root_toml)
        .with_context(|| "failed to parse top-level Cargo.toml")?;

    let include_tests = env::var("MODULE_WIRING_INCLUDE_TESTS").ok().as_deref() == Some("1");

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

    let mut crate_dirs: HashMap<PathBuf, ()> = HashMap::new();
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
                        if dir.join("Cargo.toml").exists() {
                            crate_dirs.insert(dir, ());
                        }
                    }
                }
            }
        }
    }

    if doc.get("package").is_some() && !excluded(root) {
        crate_dirs.insert(root.to_path_buf(), ());
    }

    let mut dirs: Vec<PathBuf> = Vec::new();
    for cd in crate_dirs.keys() {
        let src = cd.join("src");
        if src.exists() {
            dirs.push(src);
        }
        if include_tests {
            for extra in ["tests", "benches", "examples"] {
                let p = cd.join(extra);
                if p.exists() {
                    dirs.push(p);
                }
            }
        }
    }

    dirs.sort();
    dirs.dedup();
    Ok(dirs)
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

fn read_toml(path: &Path) -> Result<Value> {
    let s = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    Ok(s.parse::<Value>()?)
}
