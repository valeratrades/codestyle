//! Lint to require justification comments for patterns that may silently ignore errors.
//!
//! This includes:
//! - `unwrap_or`, `unwrap_or_default`, `unwrap_or_else` - can mask corrupted state with fallbacks
//! - `let _ = ...` - can silently discard Results or other important values
//!
//! A comment forces explicit acknowledgment of why ignoring the error is acceptable.

use std::path::Path;

use syn::{ExprMethodCall, Pat, PatWild, Stmt, spanned::Spanned, visit::Visit};

use super::{Violation, skip::has_skip_marker};

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let mut visitor = IgnoredErrorVisitor::new(path, content);
	visitor.visit_file(file);
	visitor.violations
}

struct IgnoredErrorVisitor<'a> {
	path_str: String,
	content: &'a str,
	violations: Vec<Violation>,
}

impl<'a> IgnoredErrorVisitor<'a> {
	fn new(path: &Path, content: &'a str) -> Self {
		Self {
			path_str: path.display().to_string(),
			content,
			violations: Vec::new(),
		}
	}

	fn has_ignored_error_comment(&self, line: usize) -> bool {
		let lines: Vec<&str> = self.content.lines().collect();

		// Check current line (inline comment)
		if line > 0 && line <= lines.len() {
			let current_line = lines[line - 1];
			if current_line.contains("//IGNORED_ERROR") || current_line.contains("// IGNORED_ERROR") {
				return true;
			}
		}

		// Check line above
		if line > 1 {
			let prev_line = lines[line - 2];
			if prev_line.contains("//IGNORED_ERROR") || prev_line.contains("// IGNORED_ERROR") {
				return true;
			}
		}

		false
	}

	fn is_standalone_underscore<'b>(&self, pat: &'b Pat) -> Option<&'b PatWild> {
		// Only match standalone `_`, not `_name` or destructuring like `(a, _)`
		if let Pat::Wild(wild) = pat { Some(wild) } else { None }
	}
}

impl<'a> Visit<'a> for IgnoredErrorVisitor<'a> {
	fn visit_item_fn(&mut self, node: &'a syn::ItemFn) {
		if has_skip_marker(self.content, node.span()) {
			return;
		}
		syn::visit::visit_item_fn(self, node);
	}

	fn visit_item_mod(&mut self, node: &'a syn::ItemMod) {
		if has_skip_marker(self.content, node.span()) {
			return;
		}
		syn::visit::visit_item_mod(self, node);
	}

	fn visit_item_impl(&mut self, node: &'a syn::ItemImpl) {
		if has_skip_marker(self.content, node.span()) {
			return;
		}
		syn::visit::visit_item_impl(self, node);
	}

	fn visit_item_struct(&mut self, node: &'a syn::ItemStruct) {
		if has_skip_marker(self.content, node.span()) {
			return;
		}
		syn::visit::visit_item_struct(self, node);
	}

	fn visit_expr_block(&mut self, node: &'a syn::ExprBlock) {
		if has_skip_marker(self.content, node.span()) {
			return;
		}
		syn::visit::visit_expr_block(self, node);
	}

	fn visit_local(&mut self, node: &'a syn::Local) {
		if has_skip_marker(self.content, node.span()) {
			return;
		}
		syn::visit::visit_local(self, node);
	}

	fn visit_expr_method_call(&mut self, node: &'a ExprMethodCall) {
		let method_name = node.method.to_string();
		if matches!(method_name.as_str(), "unwrap_or" | "unwrap_or_default" | "unwrap_or_else") {
			let span_start = node.method.span().start();
			if !self.has_ignored_error_comment(span_start.line) {
				self.violations.push(Violation {
					rule: "ignored-error-comment",
					file: self.path_str.clone(),
					line: span_start.line,
					column: span_start.column,
					message: format!(
						"`{method_name}` without `//IGNORED_ERROR` comment\n\
						HINT: Error out properly or explain why it's part of the intended logic and simply erroring out / panicking is not an option."
					),
					fix: None,
				});
			}
		}
		syn::visit::visit_expr_method_call(self, node);
	}

	fn visit_stmt(&mut self, stmt: &'a Stmt) {
		if let Stmt::Local(local) = stmt
			&& let Some(wild) = self.is_standalone_underscore(&local.pat)
			&& local.init.is_some()
		{
			let span_start = wild.underscore_token.span.start();
			if !self.has_ignored_error_comment(span_start.line) {
				self.violations.push(Violation {
					rule: "ignored-error-comment",
					file: self.path_str.clone(),
					line: span_start.line,
					column: span_start.column,
					message: "`let _ = ...` without `//IGNORED_ERROR` comment\n\
						HINT: could the pattern be allowing to continue with corrupted state? Error out properly or explain why it's part of the intended logic."
						.to_string(),
					fix: None,
				});
			}
		}
		syn::visit::visit_stmt(self, stmt);
	}
}
