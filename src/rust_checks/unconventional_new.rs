//! Lint for unconventional uses of `fn new` / `Type::new()`.
//!
//! Two cases:
//!
//! 1. `pub fn new()` with no arguments in an inherent impl — callers should use
//!    `Default::default()` instead. The definition is flagged (no autofix); all
//!    zero-argument `Type::new()` call-sites in the project are rewritten to
//!    `Type::default()` automatically.
//!
//! 2. `fn new` returning `Result<…>` in an inherent impl — rename to `try_new`
//!    so fallibility is visible at the call-site. Autofixed by renaming the
//!    function identifier. Call-sites `TypeName::new(…)` are also renamed to
//!    `TypeName::try_new(…)` using the set of known types collected project-wide.

use std::{collections::HashSet, path::Path};

use syn::{Expr, ExprCall, ExprPath, ImplItem, ImplItemFn, Item, PathSegment, ReturnType, Type, Visibility, visit::Visit};

use super::{FileInfo, Fix, Violation, skip::SkipVisitor};

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

const RULE: &str = "unconventional-new";

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

/// Collect all type names that have `fn new(...) -> Result<...>` in inherent impls
/// across the given set of parsed files. Used to find call-sites project-wide.
pub fn collect_try_new_types(file_infos: &[FileInfo]) -> HashSet<String> {
	let mut types = HashSet::default();
	for info in file_infos {
		let Some(ref tree) = info.syntax_tree else { continue };
		for item in &tree.items {
			let Item::Impl(impl_block) = item else { continue };
			if impl_block.trait_.is_some() {
				continue;
			}
			let Type::Path(self_ty) = impl_block.self_ty.as_ref() else { continue };
			let Some(last_seg) = self_ty.path.segments.last() else { continue };
			let type_name = last_seg.ident.to_string();

			for impl_item in &impl_block.items {
				let ImplItem::Fn(func) = impl_item else { continue };
				if func.sig.ident == "new" && returns_result(&func.sig.output) {
					types.insert(type_name.clone());
					break;
				}
			}
		}
	}
	types
}

/// Types whose zero-argument `::new()` must NOT be rewritten to `::default()`.
const NEW_EXCEPTIONS: &[&str] = &[
	// `trybuild::TestCases` has only `new()` and no `Default` impl — it's a
	// test-harness type that stupidly exposes bare `new()` with no alternative.
	"TestCases", // until `feature(const_trait_impl` stabilizes, `::new()` is static on it, but `::default()` is not as it needs to go through the trait machinery
	"OnceLock",
];

pub fn check(path: &Path, content: &str, file: &syn::File, try_new_types: &HashSet<String>, nontrivial_default_types: &HashSet<String>) -> Vec<Violation> {
	let visitor = UnconventionalNewVisitor::new(path, content, try_new_types, nontrivial_default_types);
	let mut skip_visitor = SkipVisitor::for_rule(visitor, content, RULE);
	skip_visitor.visit_file(file);
	skip_visitor.inner.violations
}

struct UnconventionalNewVisitor<'a> {
	path_str: String,
	content: &'a str,
	try_new_types: &'a HashSet<String>,
	nontrivial_default_types: &'a HashSet<String>,
	violations: Vec<Violation>,
	/// Type name of the current inherent impl block being visited
	current_impl_type: Option<String>,
}

impl<'a> UnconventionalNewVisitor<'a> {
	fn new(path: &Path, content: &'a str, try_new_types: &'a HashSet<String>, nontrivial_default_types: &'a HashSet<String>) -> Self {
		Self {
			path_str: path.display().to_string(),
			content,
			try_new_types,
			nontrivial_default_types,
			violations: Vec::default(),
			current_impl_type: None,
		}
	}

	fn check_impl_fn(&mut self, func: &ImplItemFn) {
		let name = func.sig.ident.to_string();
		if name != "new" {
			return;
		}

		let is_nontrivial_default = self.current_impl_type.as_deref().is_some_and(|t| self.nontrivial_default_types.contains(t));

		if returns_result(&func.sig.output) {
			// Case 2: fn new(...) -> Result — rename definition to try_new
			let span = func.sig.ident.span();
			let fix = span_to_byte(self.content, span.start()).and_then(|start| {
				span_to_byte(self.content, span.end()).map(|end| Fix {
					start_byte: start,
					end_byte: end,
					replacement: "try_new".to_string(),
				})
			});
			self.violations.push(Violation {
				rule: RULE,
				file: self.path_str.clone(),
				line: span.start().line,
				column: span.start().column,
				message: "`fn new` returns `Result` — rename to `try_new` to signal fallibility".to_string(),
				fix,
			});
		} else if func.sig.inputs.is_empty() && matches!(func.vis, Visibility::Public(_)) && !is_nontrivial_default {
			// Case 1: pub fn new() with no args — flag definition, no autofix
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
		// Must have a qualifying type segment before `new`
		if path.segments.len() < 2 {
			return;
		}

		// Get the type name (second-to-last segment)
		let type_name = path.segments[path.segments.len() - 2].ident.to_string();

		if self.try_new_types.contains(&type_name) {
			// Targeted: TypeName::new(...) -> TypeName::try_new(...)
			let span = last.ident.span();
			let fix = span_to_byte(self.content, span.start()).and_then(|start| {
				span_to_byte(self.content, span.end()).map(|end| Fix {
					start_byte: start,
					end_byte: end,
					replacement: "try_new".to_string(),
				})
			});
			self.violations.push(Violation {
				rule: RULE,
				file: self.path_str.clone(),
				line: span.start().line,
				column: span.start().column,
				message: format!("`{type_name}::new` was renamed to `try_new`"),
				fix,
			});
		} else if expr.args.is_empty() && !NEW_EXCEPTIONS.contains(&type_name.as_str()) && !self.nontrivial_default_types.contains(&type_name) {
			// Blind: any Type::new() with no args -> Type::default()
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
}

impl<'a> Visit<'a> for UnconventionalNewVisitor<'a> {
	fn visit_item_impl(&mut self, node: &'a syn::ItemImpl) {
		// Skip trait impls
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

fn returns_result(output: &ReturnType) -> bool {
	let ReturnType::Type(_, ty) = output else {
		return false;
	};
	let Type::Path(type_path) = ty.as_ref() else {
		return false;
	};
	type_path.path.segments.last().is_some_and(|seg: &PathSegment| seg.ident == "Result")
}

fn span_to_byte(content: &str, pos: proc_macro2::LineColumn) -> Option<usize> {
	let mut current_line = 1;
	let mut line_start = 0;

	for (i, ch) in content.char_indices() {
		if current_line == pos.line {
			return Some(line_start + pos.column);
		}
		if ch == '\n' {
			current_line += 1;
			line_start = i + 1;
		}
	}

	if current_line == pos.line {
		return Some(line_start + pos.column);
	}

	None
}
