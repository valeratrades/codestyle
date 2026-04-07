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
//!    function identifier.

use std::path::Path;

use syn::{Expr, ExprCall, ExprPath, ImplItem, ImplItemFn, PathSegment, ReturnType, Type, Visibility, visit::Visit};

use super::{Fix, Violation, skip::SkipVisitor};

const RULE: &str = "unconventional-new";

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let visitor = UnconventionalNewVisitor::new(path, content);
	let mut skip_visitor = SkipVisitor::for_rule(visitor, content, RULE);
	skip_visitor.visit_file(file);
	skip_visitor.inner.violations
}

struct UnconventionalNewVisitor<'a> {
	path_str: String,
	content: &'a str,
	violations: Vec<Violation>,
}

impl<'a> UnconventionalNewVisitor<'a> {
	fn new(path: &Path, content: &'a str) -> Self {
		Self {
			path_str: path.display().to_string(),
			content,
			violations: Vec::new(),
		}
	}

	fn check_impl_fn(&mut self, func: &ImplItemFn) {
		let name = func.sig.ident.to_string();
		if name != "new" {
			return;
		}

		if returns_result(&func.sig.output) {
			// Case 2: fn new() -> Result — rename definition to try_new
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
		} else if func.sig.inputs.is_empty() && matches!(func.vis, Visibility::Public(_)) {
			// Case 1: pub fn new() with no args — flag definition, no fix
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
		// Match: some::path::new(/* no args */)
		let Expr::Path(ExprPath { path, .. }) = expr.func.as_ref() else {
			return;
		};
		if expr.args.len() != 0 {
			return;
		}
		let Some(last) = path.segments.last() else {
			return;
		};
		if last.ident != "new" {
			return;
		}
		// Must have at least one prior segment (e.g. `Vec::new`, not bare `new()`)
		if path.segments.len() < 2 {
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

impl<'a> Visit<'a> for UnconventionalNewVisitor<'a> {
	fn visit_item_impl(&mut self, node: &'a syn::ItemImpl) {
		// Skip trait impls
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
