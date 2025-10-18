//! ```cargo
//! [dependencies]
//! anyhow = "1"
//! walkdir = "2"
//! syn = { version = "2", features = ["full", "visit"] }
//! quote = "1"
//! proc-macro2 = { version = "1", features = ["span-locations"] }
//! ```

use anyhow::{bail, Context, Result};
use proc_macro2::Span;
use quote::ToTokens;
use std::env;
use std::path::{Path, PathBuf};
use syn::{visit::Visit, Block, Expr, File, ImplItem, Item, ItemImpl, ItemTrait, TraitItem};
use walkdir::WalkDir;

fn main() -> Result<()> {
  let mut args = env::args().skip(1).collect::<Vec<_>>();
  let mut threshold = env::var("CYCLO_THRESHOLD")
    .ok()
    .and_then(|value| value.parse::<usize>().ok())
    .unwrap_or(10);

  let mut targets = Vec::new();

  while let Some(arg) = args.first() {
    if arg == "--threshold" {
      args.remove(0);
      let value = args
        .get(0)
        .context("`--threshold` expects a number")?
        .parse::<usize>()
        .context("failed to parse threshold value")?;
      threshold = value;
      args.remove(0);
      continue;
    }
    break;
  }

  if args.is_empty() {
    targets.extend([
      PathBuf::from("modules/actor-core/src"),
      PathBuf::from("modules/actor-std/src"),
      PathBuf::from("modules/actor-embedded/src"),
    ]);
  } else {
    targets.extend(args.into_iter().map(PathBuf::from));
  }

  let mut results = Vec::new();

  for target in targets {
    if !target.exists() {
      bail!("target path `{}` does not exist", target.display());
    }
    analyse_target(&target, &mut results)?;
  }

  results.sort_by(|a, b| {
    b.complexity
      .cmp(&a.complexity)
      .then_with(|| a.path.cmp(&b.path))
      .then_with(|| a.line.cmp(&b.line))
  });

  let filtered: Vec<_> = results.into_iter().filter(|r| r.complexity >= threshold).collect();

  if filtered.is_empty() {
    println!(
      "No functions exceeded complexity threshold ({}).",
      threshold
    );
    return Ok(());
  }

  println!(
    "{:>8}  {:<60}  {}",
    "Complex", "Location", "Function"
  );
  println!("{}", "-".repeat(8 + 2 + 60 + 2 + 40));

  for entry in filtered {
    println!(
      "{:>8}  {:<60}  {}",
      entry.complexity,
      format!("{}:{}", entry.path.display(), entry.line),
      entry.name
    );
  }

  Ok(())
}

fn analyse_target(target: &Path, results: &mut Vec<FnEntry>) -> Result<()> {
  for entry in WalkDir::new(target).into_iter().filter_map(|e| e.ok()) {
    if !entry.file_type().is_file() {
      continue;
    }
    if entry.path().extension().and_then(|s| s.to_str()) != Some("rs") {
      continue;
    }
    let content = std::fs::read_to_string(entry.path())
      .with_context(|| format!("failed to read {}", entry.path().display()))?;
    let Ok(parsed) = syn::parse_file(&content) else {
      continue;
    };
    analyse_file(&parsed, entry.path(), results)?;
  }
  Ok(())
}

fn analyse_file(file: &File, path: &Path, results: &mut Vec<FnEntry>) -> Result<()> {
  let mut module_stack = Vec::new();
  analyse_items(&file.items, path, &mut module_stack, results)
}

fn analyse_items(
  items: &[Item],
  path: &Path,
  module_stack: &mut Vec<String>,
  results: &mut Vec<FnEntry>,
) -> Result<()> {
  for item in items {
    match item {
      Item::Fn(func) => {
        let name = qualified_name(module_stack, None, &func.sig.ident.to_string());
        let line = line_number(func.sig.ident.span());
        let complexity = compute_complexity(&func.block);
        results.push(FnEntry {
          name,
          complexity,
          path: path.to_path_buf(),
          line,
        });
      },
      Item::Impl(item_impl) => analyse_impl(item_impl, path, module_stack, results)?,
      Item::Trait(item_trait) => analyse_trait(item_trait, path, module_stack, results)?,
      Item::Mod(item_mod) => {
        if let Some((_, items)) = &item_mod.content {
          module_stack.push(item_mod.ident.to_string());
          analyse_items(items, path, module_stack, results)?;
          module_stack.pop();
        }
      },
      _ => {},
    }
  }
  Ok(())
}

fn analyse_impl(
  item_impl: &ItemImpl,
  path: &Path,
  module_stack: &mut Vec<String>,
  results: &mut Vec<FnEntry>,
) -> Result<()> {
  let ty = item_impl.self_ty.to_token_stream().to_string();
  for impl_item in &item_impl.items {
    if let ImplItem::Fn(func) = impl_item {
      let name = qualified_name(module_stack, Some(&ty), &func.sig.ident.to_string());
      let line = line_number(func.sig.ident.span());
      let complexity = compute_complexity(&func.block);
      results.push(FnEntry {
        name,
        complexity,
        path: path.to_path_buf(),
        line,
      });
    }
  }
  Ok(())
}

fn analyse_trait(
  item_trait: &ItemTrait,
  path: &Path,
  module_stack: &mut Vec<String>,
  results: &mut Vec<FnEntry>,
) -> Result<()> {
  let trait_name = item_trait.ident.to_string();
  for item in &item_trait.items {
    if let TraitItem::Fn(func) = item {
      if let Some(block) = &func.default {
        let name = qualified_name(module_stack, Some(&trait_name), &func.sig.ident.to_string());
        let line = line_number(func.sig.ident.span());
        let complexity = compute_complexity(block);
        results.push(FnEntry {
          name,
          complexity,
          path: path.to_path_buf(),
          line,
        });
      }
    }
  }
  Ok(())
}

fn qualified_name(
  modules: &[String],
  receiver: Option<&str>,
  ident: &str,
) -> String {
  let mut parts = Vec::new();
  if !modules.is_empty() {
    parts.push(modules.join("::"));
  }
  if let Some(receiver) = receiver {
    parts.push(receiver.to_string());
  }
  parts.push(ident.to_string());
  parts.join("::")
}

fn line_number(span: Span) -> usize {
  span.start().line
}

fn compute_complexity(block: &Block) -> usize {
  let mut visitor = ComplexityVisitor { decisions: 0 };
  visitor.visit_block(block);
  visitor.decisions + 1
}

struct ComplexityVisitor {
  decisions: usize,
}

impl<'ast> syn::visit::Visit<'ast> for ComplexityVisitor {
  fn visit_expr(&mut self, node: &'ast Expr) {
    match node {
      Expr::If(expr) => {
        self.decisions += 1;
        syn::visit::visit_expr_if(self, expr);
      },
      Expr::ForLoop(expr) => {
        self.decisions += 1;
        syn::visit::visit_expr_for_loop(self, expr);
      },
      Expr::While(expr) => {
        self.decisions += 1;
        syn::visit::visit_expr_while(self, expr);
      },
      Expr::Loop(expr) => {
        self.decisions += 1;
        syn::visit::visit_expr_loop(self, expr);
      },
      Expr::Match(expr) => {
        self.decisions += expr.arms.len();
        syn::visit::visit_expr_match(self, expr);
      },
      Expr::Binary(expr) => {
        use syn::BinOp::*;
        if matches!(expr.op, And(_) | Or(_)) {
          self.decisions += 1;
        }
        syn::visit::visit_expr_binary(self, expr);
      },
      Expr::Try(expr) => {
        self.decisions += 1;
        syn::visit::visit_expr_try(self, expr);
      },
      Expr::Await(expr) => {
        syn::visit::visit_expr_await(self, expr);
      },
      _ => syn::visit::visit_expr(self, node),
    }
  }
}

struct FnEntry {
  name: String,
  complexity: usize,
  path: PathBuf,
  line: usize,
}
