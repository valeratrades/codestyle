//! Lint for argument-less `pub fn new()` in inherent impls.
//!
//! When a type has `pub fn new()` with no arguments, callers should use
//! `Default::default()` instead. The definition is flagged (no autofix); all
//! zero-argument `Type::new()` call-sites in the project are rewritten to
//! `Type::default()` automatically.
//!
//! **Note:** This rule is disabled by default because not every `new()` is a
//! drop-in replacement for `Default`. For example, `OpenOptions::new()` creates
//! an instance with all options set to false — read, write, append, truncate,
//! create, create_new all off. That's arguably not a useful default (you can't
//! do anything with it), so the std authors chose not to impl Default to avoid
//! confusion. `new()` is explicit about "I'm building one from scratch".

use std::{collections::HashSet, path::Path};

use syn::{Expr, ExprCall, ExprPath, ImplItem, ImplItemFn, Item, Type, Visibility, visit::Visit};

use super::{FileInfo, Fix, Violation, path_rewrite::span_to_byte, skip::SkipVisitor};

const RULE: &str = "prefer-default-over-bare-new";

struct HasFnCallVisitor {
	found: bool,
}

impl<'a> Visit<'a> for HasFnCallVisitor {
	fn visit_expr_call(&mut self, _node: &'a ExprCall) {
		self.found = true;
	}
}

fn default_impl_has_fn_calls(impl_block: &syn::ItemImpl) -> bool {
	let mut visitor = HasFnCallVisitor { found: false };
	visitor.visit_item_impl(impl_block);
	visitor.found
}

/// Collect all type names whose `Default` trait impl body contains any function calls.
/// These types must not have their `Type::new()` call-sites rewritten to `Type::default()`,
/// and their `pub fn new()` definitions must not be flagged.
pub fn collect_nontrivial_default_types(file_infos: &[FileInfo]) -> HashSet<String> {
	let mut types = HashSet::default();
	for info in file_infos {
		let Some(ref tree) = info.syntax_tree else { continue };
		for item in &tree.items {
			let Item::Impl(impl_block) = item else { continue };
			let Some((_, trait_path, _)) = &impl_block.trait_ else { continue };
			let Some(last_seg) = trait_path.segments.last() else { continue };
			if last_seg.ident != "Default" {
				continue;
			}
			if !default_impl_has_fn_calls(impl_block) {
				continue;
			}
			let Type::Path(self_ty) = impl_block.self_ty.as_ref() else { continue };
			let Some(last_ty_seg) = self_ty.path.segments.last() else { continue };
			types.insert(last_ty_seg.ident.to_string());
		}
	}
	types
}

/// Types whose zero-argument `::new()` must NOT be rewritten to `::default()`.
const NEW_EXCEPTIONS: &[&str] = &[
	// `trybuild::TestCases` has only `new()` and no `Default` impl — it's a
	// test-harness type that stupidly exposes bare `new()` with no alternative.
	"TestCases", // until `feature(const_trait_impl` stabilizes, `::new()` is static on it, but `::default()` is not as it needs to go through the trait machinery
	"OnceLock", "OpenOptions", "WayshotConnection",
];

pub fn check(path: &Path, content: &str, file: &syn::File, nontrivial_default_types: &HashSet<String>) -> Vec<Violation> {
	let visitor = PreferDefaultVisitor::new(path, content, nontrivial_default_types);
	let mut skip_visitor = SkipVisitor::for_rule(visitor, content, RULE);
	skip_visitor.visit_file(file);
	skip_visitor.inner.violations
}

struct PreferDefaultVisitor<'a> {
	path_str: String,
	content: &'a str,
	nontrivial_default_types: &'a HashSet<String>,
	violations: Vec<Violation>,
	/// Type name of the current inherent impl block being visited
	current_impl_type: Option<String>,
}

impl<'a> PreferDefaultVisitor<'a> {
	fn new(path: &Path, content: &'a str, nontrivial_default_types: &'a HashSet<String>) -> Self {
		Self {
			path_str: path.display().to_string(),
			content,
			nontrivial_default_types,
			violations: Vec::default(),
			current_impl_type: None,
		}
	}

	fn check_impl_fn(&mut self, func: &ImplItemFn) {
		if func.sig.ident != "new" {
			return;
		}
		if !func.sig.inputs.is_empty() {
			return;
		}
		if !matches!(func.vis, Visibility::Public(_)) {
			return;
		}
		let is_nontrivial_default = self.current_impl_type.as_deref().is_some_and(|t| self.nontrivial_default_types.contains(t));
		if is_nontrivial_default {
			return;
		}
		let span = func.sig.ident.span();
		self.violations.push(Violation {
			rule: RULE,
			file: self.path_str.clone(),
			line: span.start().line,
			column: span.start().column,
			message: "argument-less `pub fn new()` found; implement `Default` instead — callers should use `Type::default()`".to_string(),
			fix: None,
		});
	}

	fn check_call(&mut self, expr: &ExprCall) {
		let Expr::Path(ExprPath { path, .. }) = expr.func.as_ref() else {
			return;
		};
		let Some(last) = path.segments.last() else {
			return;
		};
		if last.ident != "new" {
			return;
		}
		if path.segments.len() < 2 {
			return;
		}
		if !expr.args.is_empty() {
			return;
		}
		let type_name = path.segments[path.segments.len() - 2].ident.to_string();
		if NEW_EXCEPTIONS.contains(&type_name.as_str()) || self.nontrivial_default_types.contains(&type_name) {
			return;
		}
		let span = last.ident.span();
		let fix = span_to_byte(self.content, span.start()).and_then(|start| {
			span_to_byte(self.content, span.end()).map(|end| Fix {
				start_byte: start,
				end_byte: end,
				replacement: "default".to_string(),
			})
		});
		self.violations.push(Violation {
			rule: RULE,
			file: self.path_str.clone(),
			line: span.start().line,
			column: span.start().column,
			message: "`Type::new()` — use `Type::default()` instead".to_string(),
			fix,
		});
	}
}

impl<'a> Visit<'a> for PreferDefaultVisitor<'a> {
	fn visit_item_impl(&mut self, node: &'a syn::ItemImpl) {
		if node.trait_.is_some() {
			return;
		}
		let type_name = if let Type::Path(tp) = node.self_ty.as_ref() {
			tp.path.segments.last().map(|s| s.ident.to_string())
		} else {
			None
		};
		let prev = self.current_impl_type.take();
		self.current_impl_type = type_name;
		for item in &node.items {
			if let ImplItem::Fn(func) = item {
				self.check_impl_fn(func);
			}
		}
		syn::visit::visit_item_impl(self, node);
		self.current_impl_type = prev;
	}

	fn visit_expr_call(&mut self, node: &'a ExprCall) {
		self.check_call(node);
		syn::visit::visit_expr_call(self, node);
	}
}
