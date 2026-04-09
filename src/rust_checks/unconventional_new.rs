//! Lint for unconventional uses of `fn new` / `Type::new()`.
//!
//! `fn new` returning `Result<…>` in an inherent impl — rename to `try_new`
//! so fallibility is visible at the call-site. Autofixed by renaming the
//! function identifier. Call-sites `TypeName::new(…)` are also renamed to
//! `TypeName::try_new(…)` using the set of known types collected project-wide.

use std::{collections::HashSet, path::Path};

use syn::{Expr, ExprCall, ExprPath, ImplItem, Item, PathSegment, ReturnType, Type, visit::Visit};

use super::{FileInfo, Fix, Violation, skip::SkipVisitor};

const RULE: &str = "unconventional-new";

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

pub fn check(path: &Path, content: &str, file: &syn::File, try_new_types: &HashSet<String>) -> Vec<Violation> {
	let visitor = UnconventionalNewVisitor::new(path, content, try_new_types);
	let mut skip_visitor = SkipVisitor::for_rule(visitor, content, RULE);
	skip_visitor.visit_file(file);
	skip_visitor.inner.violations
}

struct UnconventionalNewVisitor<'a> {
	path_str: String,
	content: &'a str,
	try_new_types: &'a HashSet<String>,
	violations: Vec<Violation>,
}

impl<'a> UnconventionalNewVisitor<'a> {
	fn new(path: &Path, content: &'a str, try_new_types: &'a HashSet<String>) -> Self {
		Self {
			path_str: path.display().to_string(),
			content,
			try_new_types,
			violations: Vec::default(),
		}
	}

	fn check_impl_fn(&mut self, func: &syn::ImplItemFn) {
		if func.sig.ident != "new" {
			return;
		}
		if !returns_result(&func.sig.output) {
			return;
		}
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
		let type_name = path.segments[path.segments.len() - 2].ident.to_string();
		if !self.try_new_types.contains(&type_name) {
			return;
		}
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
	}
}

impl<'a> Visit<'a> for UnconventionalNewVisitor<'a> {
	fn visit_item_impl(&mut self, node: &'a syn::ItemImpl) {
		if node.trait_.is_some() {
			return;
		}
		for item in &node.items {
			if let ImplItem::Fn(func) = item {
				self.check_impl_fn(func);
			}
		}
		syn::visit::visit_item_impl(self, node);
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
