#![feature(rustc_private)]

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use quote::ToTokens;
use rustc_errors::Diag;
use rustc_hir::{Item, ItemKind, UseKind, UsePath};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::source_map::SourceMap;
use rustc_span::symbol::{sym, Symbol};
use rustc_span::{FileName, RealFileName, Span};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use syn::{Attribute as SynAttribute, Item as SynItem, ItemMod as SynItemMod};

dylint_linting::impl_late_lint! {
    pub MODULE_WIRING,
    Warn,
    "enforce module wiring conventions for re-exports",
    ModuleWiring::default()
}

pub struct ModuleWiring {
  seen_files: HashSet<PathBuf>,
  delegation_cache: HashMap<PathBuf, bool>,
}

impl Default for ModuleWiring {
  fn default() -> Self {
    Self {
      seen_files: HashSet::new(),
      delegation_cache: HashMap::new(),
    }
  }
}

impl<'tcx> LateLintPass<'tcx> for ModuleWiring {
  fn check_item(&mut self, cx: &LateContext<'tcx>, item: &Item<'tcx>) {
    let sm = cx.tcx.sess.source_map();
    let Some(file_path) = file_path_from_span(sm, item.span) else {
      return;
    };
    if is_exception_file(&file_path) {
      return;
    }
    if !self.seen_files.insert(file_path.clone()) {
      return;
    }
    analyze_file(cx, &file_path, &mut self.delegation_cache);
  }
}

struct ModuleData {
  skip_missing: bool,
  mods: HashMap<String, ModEntry>,
  reexports: HashMap<String, Vec<ReexportEntry>>,
}

#[derive(Clone)]
struct ModEntry {
  symbol: Symbol,
  span: Span,
  is_public: bool,
  has_reexport: bool,
  delegates_children: bool,
  is_cfg_test: bool,
}

#[derive(Clone)]
struct ReexportEntry {
  span: Span,
  snippet: Option<String>,
}

const ALLOW_FILE_FLAG: &str = "allow:module-wiring-skip";
const IGNORED_CHILD_MODULES: &[&str] = &["tests", "test", "bench", "benches"];

fn analyze_file(cx: &LateContext<'_>, file_path: &Path, delegation_cache: &mut HashMap<PathBuf, bool>) {
  let Ok(src) = fs::read_to_string(file_path) else {
    return;
  };
  let skip_missing = should_skip_missing_check(&src);

  let mut mods: HashMap<String, ModEntry> = HashMap::new();
  let mut reexports: HashMap<String, Vec<ReexportEntry>> = HashMap::new();

  collect_items_for_file(cx, file_path, delegation_cache, &mut mods, &mut reexports);

  let mut data = ModuleData {
    skip_missing,
    mods,
    reexports,
  };

  emit_violations(cx, &mut data);

  // nothing else to do
}

fn collect_items_for_file(
  cx: &LateContext<'_>,
  file_path: &Path,
  delegation_cache: &mut HashMap<PathBuf, bool>,
  mods: &mut HashMap<String, ModEntry>,
  reexports: &mut HashMap<String, Vec<ReexportEntry>>,
) {
  let sm = cx.tcx.sess.source_map();
  let crate_items = cx.tcx.hir_crate_items(());

  for item_id in crate_items.free_items() {
    let hir_item = cx.tcx.hir_expect_item(item_id.owner_id.def_id);
    let Some(item_path) = file_path_from_span(sm, hir_item.span) else {
      continue;
    };
    if item_path != file_path {
      continue;
    }

    if let ItemKind::Mod(_, module) = &hir_item.kind {
      let child_name = cx.tcx.item_name(hir_item.owner_id.def_id.to_def_id());
      if should_ignore_symbol(child_name) {
        continue;
      }
      if let Some(entry) = build_mod_entry(cx, file_path, hir_item, module, delegation_cache) {
        let key = entry.symbol.to_string();
        mods.insert(key, entry);
      }
    }
  }

  for item_id in crate_items.free_items() {
    let hir_item = cx.tcx.hir_expect_item(item_id.owner_id.def_id);
    let Some(item_path) = file_path_from_span(sm, hir_item.span) else {
      continue;
    };
    if item_path != file_path {
      continue;
    }

    if let ItemKind::Use(path, use_kind) = &hir_item.kind {
      collect_use_item(cx, hir_item, path, *use_kind, mods, reexports);
    }
  }
}

fn build_mod_entry(
  cx: &LateContext<'_>,
  parent_file: &Path,
  item: &Item<'_>,
  _module: &rustc_hir::Mod<'_>,
  delegation_cache: &mut HashMap<PathBuf, bool>,
) -> Option<ModEntry> {
  let child_name = cx.tcx.item_name(item.owner_id.def_id.to_def_id());
  let visibility = cx.tcx.local_visibility(item.owner_id.def_id);
  let is_public = visibility.is_public();
  let is_cfg_test = has_cfg_test_attr(cx, cx.tcx.hir_attrs(item.hir_id()));

  let delegates_children = resolve_child_path(parent_file, child_name.as_str())
    .map(|child_path| module_delegates(&child_path, delegation_cache))
    .unwrap_or(false);

  Some(ModEntry {
    symbol: child_name,
    span: item.span,
    is_public,
    has_reexport: false,
    delegates_children,
    is_cfg_test,
  })
}

fn collect_use_item(
  cx: &LateContext<'_>,
  item: &Item<'_>,
  path: &UsePath<'_>,
  _use_kind: UseKind,
  mods: &HashMap<String, ModEntry>,
  reexports: &mut HashMap<String, Vec<ReexportEntry>>,
) {
  if !cx.tcx.local_visibility(item.owner_id.def_id).is_public() {
    return;
  }

  let segments: Vec<String> = path.segments.iter().map(|seg| seg.ident.to_string()).collect();
  if segments.is_empty() {
    return;
  }

  let first_segment = segments.first().cloned();
  let mut normalized = segments;
  while matches!(
    normalized.first().map(|s| s.as_str()),
    Some("self") | Some("super") | Some("crate")
  ) {
    normalized.remove(0);
  }

  if normalized.is_empty() {
    return;
  }

  let child_key = normalized[0].clone();
  let should_track =
    mods.contains_key(&child_key) || matches!(first_segment.as_deref(), Some("self") | Some("super") | Some("crate"));

  if !should_track {
    return;
  }

  let snippet = cx.tcx.sess.source_map().span_to_snippet(item.span).ok();
  let entry = ReexportEntry {
    span: item.span,
    snippet,
  };
  reexports.entry(child_key).or_default().push(entry);

  // For glob imports, we want to ensure the key exists for subsequent diagnostics even if
  // the parent didn't declare the child explicitly. `mods` remains unchanged here; missing
  // entries will be handled later.
}

fn emit_violations(cx: &LateContext<'_>, data: &mut ModuleData) {
  for (child, entries) in &data.reexports {
    if let Some(entry) = data.mods.get_mut(child) {
      entry.has_reexport = true;
      if entry.is_public {
        for reexport in entries {
          emit_pub_mod_with_reexport(cx, entry, reexport);
        }
      }
      if entry.delegates_children {
        for reexport in entries {
          emit_delegated_reexport(cx, child, reexport);
        }
      }
    } else {
      for reexport in entries {
        emit_stray_reexport(cx, child, reexport);
      }
    }
  }

  for entry in data.mods.values() {
    if entry.is_cfg_test {
      continue;
    }
    if should_ignore_symbol(entry.symbol) {
      continue;
    }
    if entry.delegates_children {
      if !entry.is_public {
        emit_non_public_delegator(cx, entry);
      }
      continue;
    }
    if entry.is_public {
      continue;
    }
    if !entry.has_reexport && !data.skip_missing {
      emit_missing_reexport(cx, entry);
    }
  }
}

fn emit_pub_mod_with_reexport(cx: &LateContext<'_>, entry: &ModEntry, reexport: &ReexportEntry) {
  let child_name = entry.symbol.to_ident_string();
  cx.span_lint(MODULE_WIRING, entry.span, |diag: &mut Diag<'_, ()>| {
    diag.primary_message(format!(
      "`pub mod {child_name};` is declared alongside a public re-export"
    ));
    if let Some(snippet) = &reexport.snippet {
      diag.help(format!(
        "replace this with `mod {child_name};` and keep only `{snippet}`"
      ));
    } else {
      diag.help(format!(
        "replace `pub mod {child_name};` with `mod {child_name};` and keep the re-export only"
      ));
    }
  });
}

fn emit_delegated_reexport(cx: &LateContext<'_>, child: &str, reexport: &ReexportEntry) {
  let snippet = reexport.snippet.as_deref().unwrap_or("public re-export");
  cx.span_lint(MODULE_WIRING, reexport.span, |diag: &mut Diag<'_, ()>| {
    diag.primary_message(format!("`{snippet}` re-exports from the non-leaf module `{child}`"));
    diag.help("only the direct parent module should re-export leaf types");
  });
}

fn emit_stray_reexport(cx: &LateContext<'_>, child: &str, reexport: &ReexportEntry) {
  let snippet = reexport.snippet.as_deref().unwrap_or("public re-export");
  cx.span_lint(MODULE_WIRING, reexport.span, |diag: &mut Diag<'_, ()>| {
    diag.primary_message(format!(
      "`{snippet}` has no matching `mod {child};` in this parent module"
    ));
    diag.help("declare the module locally and re-export from the direct parent only");
  });
}

fn emit_non_public_delegator(cx: &LateContext<'_>, entry: &ModEntry) {
  let child_name = entry.symbol.to_ident_string();
  cx.span_lint(MODULE_WIRING, entry.span, |diag: &mut Diag<'_, ()>| {
    diag.primary_message(format!("`mod {child_name};` aggregates submodules but is not exported"));
    diag.help(format!(
      "change to `pub mod {child_name};` and let deeper modules handle re-exports"
    ));
  });
}

fn emit_missing_reexport(cx: &LateContext<'_>, entry: &ModEntry) {
  let child_name = entry.symbol.to_ident_string();
  cx.span_lint(MODULE_WIRING, entry.span, |diag: &mut Diag<'_, ()>| {
    diag.primary_message(format!(
      "`mod {child_name};` is missing a matching `pub use {child_name}::...;` re-export"
    ));
    diag.help(format!(
      "add `pub use {child_name}::Type;` so the parent re-exports the leaf type"
    ));
  });
}

fn file_path_from_span(sm: &SourceMap, span: Span) -> Option<PathBuf> {
  match sm.span_to_filename(span) {
    FileName::Real(RealFileName::LocalPath(path)) => Some(path.to_path_buf()),
    _ => None,
  }
}

fn should_skip_missing_check(src: &str) -> bool {
  src.lines().take(80).any(|line| line.contains(ALLOW_FILE_FLAG))
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

fn module_delegates(path: &Path, cache: &mut HashMap<PathBuf, bool>) -> bool {
  if let Some(cached) = cache.get(path) {
    return *cached;
  }

  let delegates = fs::read_to_string(path)
    .ok()
    .and_then(|src| syn::parse_file(&src).ok())
    .map(|file| {
      file.items.into_iter().any(|item| match item {
        SynItem::Mod(SynItemMod {
          content: None,
          ident,
          attrs,
          ..
        }) => {
          let name = ident.to_string();
          if should_ignore_name(&name) || name.starts_with("__") {
            return false;
          }
          !syn_has_cfg_test(&attrs)
        }
        _ => false,
      })
    })
    .unwrap_or(false);

  cache.insert(path.to_path_buf(), delegates);
  delegates
}

fn syn_has_cfg_test(attrs: &[SynAttribute]) -> bool {
  attrs.iter().any(|attr| {
    let path = attr.path();
    (path.is_ident("cfg") || path.is_ident("cfg_attr")) && attr.meta.to_token_stream().to_string().contains("test")
  })
}

fn has_cfg_test_attr(cx: &LateContext<'_>, attrs: &[rustc_hir::Attribute]) -> bool {
  let sm = cx.tcx.sess.source_map();
  attrs.iter().any(|attr| {
    (attr.has_name(sym::cfg) || attr.has_name(sym::cfg_attr))
      && sm
        .span_to_snippet(attr.span())
        .map(|snippet| snippet.contains("test"))
        .unwrap_or(false)
  })
}

fn should_ignore_symbol(symbol: Symbol) -> bool {
  IGNORED_CHILD_MODULES.iter().any(|ignored| symbol.as_str() == *ignored)
}

fn should_ignore_name(name: &str) -> bool {
  IGNORED_CHILD_MODULES.iter().any(|ignored| name == *ignored)
}

fn is_exception_file(path: &Path) -> bool {
  matches!(
    path.file_name().and_then(|s| s.to_str()),
    Some("main.rs") | Some("tests.rs") | Some("build.rs")
  )
}
