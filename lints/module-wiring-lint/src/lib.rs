#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

use rustc_ast::ast::{MetaItemInner, MetaItemKind};
use rustc_ast::Attribute;
use rustc_hir::{Item, ItemKind, Mod, OwnerId, Path, UseKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Visibility;
use rustc_session::{declare_lint, declare_lint_pass};
use rustc_span::symbol::{kw, sym, Symbol};
use rustc_span::Span;
use std::collections::{HashMap, HashSet};

declare_lint! {
    pub MODULE_WIRING,
    Warn,
    "enforce module wiring conventions for re-exports"
}

declare_lint_pass!(ModuleWiring => [MODULE_WIRING]);

impl<'tcx> LateLintPass<'tcx> for ModuleWiring {
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        let hir = cx.tcx.hir();
        let mut modules: HashMap<OwnerId, ModuleData> = HashMap::new();

        for item_id in hir.items() {
            let item = hir.item(item_id);
            let parent = hir.get_parent_item(item.hir_id());
            let entry = modules.entry(parent).or_default();
            match &item.kind {
                ItemKind::Mod(_, module) => {
                    if let Some(child) = ChildMod::from_item(cx, item, module) {
                        entry.child_mods.push(child);
                    }
                }
                ItemKind::Use(path, use_kind) => {
                    if let Some(reexport) = Reexport::from_item(cx, item, path, *use_kind) {
                        entry.reexports.push(reexport);
                    }
                }
                _ => {}
            }
        }

        for data in modules.values() {
            analyze_module(cx, data);
        }
    }
}

#[derive(Default)]
struct ModuleData {
    child_mods: Vec<ChildMod>,
    reexports: Vec<Reexport>,
}

struct ChildMod {
    name: Symbol,
    span: Span,
    visibility: Visibility,
    is_cfg_test: bool,
}

impl ChildMod {
    fn from_item(cx: &LateContext<'_>, item: &Item<'_>, module: &rustc_hir::Mod<'_>) -> Option<Self> {
        let sm = cx.tcx.sess.source_map();
        let decl_file = sm.span_to_filename(item.span);
        let inner_file = sm.span_to_filename(module.spans.inner_span);
        if decl_file == inner_file {
            // Inline module: ignore.
            return None;
        }

        let name = item.ident.name;
        if IGNORED_CHILD_MODULES.contains(&name.as_str().as_str()) {
            return None;
        }

        let visibility = cx.tcx.local_visibility(item.owner_id.def_id);
        let is_cfg_test = has_cfg_test(&item.attrs);

        Some(Self {
            name,
            span: item.span,
            visibility,
            is_cfg_test,
        })
    }
}

struct Reexport {
    child: Symbol,
    span: Span,
    snippet: Option<String>,
}

impl Reexport {
    fn from_item<'tcx>(
        cx: &LateContext<'tcx>,
        item: &Item<'tcx>,
        path: &Path<'tcx>,
        _use_kind: UseKind,
    ) -> Option<Self> {
        if !cx.tcx.local_visibility(item.owner_id.def_id).is_public() {
            return None;
        }

        let child = first_real_segment(path)?;
        if IGNORED_CHILD_MODULES.contains(&child.as_str().as_str()) {
            return None;
        }

        let snippet = cx
            .tcx
            .sess
            .source_map()
            .span_to_snippet(item.span)
            .ok();

        Some(Self {
            child,
            span: item.span,
            snippet,
        })
    }
}

const IGNORED_CHILD_MODULES: &[&str] = &["tests", "test", "bench", "benches"];

fn analyze_module(cx: &LateContext<'_>, data: &ModuleData) {
    if data.child_mods.is_empty() && data.reexports.is_empty() {
        return;
    }

    let mut child_lookup: HashMap<Symbol, &ChildMod> = HashMap::new();
    for child in &data.child_mods {
        child_lookup.insert(child.name, child);
    }

    let mut reexports_by_child: HashMap<Symbol, Vec<&Reexport>> = HashMap::new();
    for reexport in &data.reexports {
        reexports_by_child
            .entry(reexport.child)
            .or_default()
            .push(reexport);
    }

    let mut processed: HashSet<Symbol> = HashSet::new();

    for (&symbol, child) in &child_lookup {
        processed.insert(symbol);
        let reexports = reexports_by_child.get(&symbol);

        if child.visibility.is_public() {
            if let Some(entries) = reexports {
                for reexport in entries {
                    emit_pub_mod_with_reexport(cx, child, reexport);
                }
            }
            continue;
        }

        if child.is_cfg_test {
            continue;
        }

        if reexports.is_none() {
            emit_missing_reexport(cx, child);
        }
    }

    for (&symbol, entries) in &reexports_by_child {
        if processed.contains(&symbol) {
            continue;
        }
        if IGNORED_CHILD_MODULES.contains(&symbol.as_str().as_str()) {
            continue;
        }
        for reexport in entries {
            emit_stray_reexport(cx, reexport);
        }
    }
}

fn emit_pub_mod_with_reexport(cx: &LateContext<'_>, child: &ChildMod, reexport: &Reexport) {
    let child_name = child.name.to_ident_string();
    cx.struct_span_lint(MODULE_WIRING, child.span, |lint| {
        let mut diag = lint.build(&format!(
            "`pub mod {child_name};` is declared alongside a public re-export"
        ));
        if let Some(snippet) = &reexport.snippet {
            diag.help(&format!(
                "replace this with `mod {child_name};` and keep only `{snippet}`"
            ));
        } else {
            diag.help(&format!(
                "replace `pub mod {child_name};` with `mod {child_name};` and keep the re-export only"
            ));
        }
        diag.emit();
    });
}

fn emit_missing_reexport(cx: &LateContext<'_>, child: &ChildMod) {
    let child_name = child.name.to_ident_string();
    cx.struct_span_lint(MODULE_WIRING, child.span, |lint| {
        lint.build(&format!(
            "`mod {child_name};` is missing a matching `pub use {child_name}::...;` re-export"
        ))
        .help(&format!(
            "add `pub use {child_name}::Type;` so this parent re-exports the leaf type"
        ))
        .emit();
    });
}

fn emit_stray_reexport(cx: &LateContext<'_>, reexport: &Reexport) {
    let snippet = reexport
        .snippet
        .as_deref()
        .unwrap_or("public re-export");
    cx.struct_span_lint(MODULE_WIRING, reexport.span, |lint| {
        lint.build(&format!(
            "`{snippet}` has no matching `mod {}` in this parent module",
            reexport.child.to_ident_string()
        ))
        .help("only the direct parent module should re-export the leaf types")
        .emit();
    });
}

fn first_real_segment(path: &Path<'_>) -> Option<Symbol> {
    for segment in path.segments {
        let name = segment.ident.name;
        if name == kw::SelfLower || name == kw::Super || name == kw::Crate {
            continue;
        }
        return Some(name);
    }
    None
}

fn has_cfg_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if attr.has_name(sym::cfg) || attr.has_name(sym::cfg_attr) {
            if let Some(list) = attr.meta_item_list() {
                return list.iter().any(|nested| match nested {
                    MetaItemInner::MetaItem(meta) => match &meta.kind {
                        MetaItemKind::Word => meta
                            .path
                            .segments
                            .first()
                            .is_some_and(|seg| seg.ident.name == sym::test),
                        MetaItemKind::List(items) => items.iter().any(|inner| match inner {
                            MetaItemInner::MetaItem(inner_meta) => inner_meta
                                .path
                                .segments
                                .first()
                                .is_some_and(|seg| seg.ident.name == sym::test),
                            MetaItemInner::Lit(_) => false,
                        }),
                        MetaItemKind::NameValue(_) => false,
                    },
                    MetaItemInner::Lit(_) => false,
                });
            }
        }
        false
    })
}
