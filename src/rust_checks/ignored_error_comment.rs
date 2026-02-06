//! Lint to require justification comments for patterns that may silently ignore errors.
//!
//! This includes:
//! - `unwrap_or`, `unwrap_or_default`, `unwrap_or_else` - can mask corrupted state with fallbacks
//! - `let _ = ...` - can silently discard Results or other important values
//!
//! A comment forces explicit acknowledgment of why ignoring the error is acceptable.

use std::{ops::Range, path::Path};

use syn::{ExprMethodCall, Pat, PatWild, Stmt, spanned::Spanned, visit::Visit};

use super::{Violation, skip::has_skip_marker_for_rule};

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let mut visitor = IgnoredErrorVisitor::new(path, content);
	visitor.visit_file(file);
	visitor.violations
}
const RULE: &str = "ignored-error-comment";

struct IgnoredErrorVisitor<'a> {
	path_str: String,
	content: &'a str,
	violations: Vec<Violation>,
	/// Stack of line ranges that are skipped due to codestyle::skip markers
	skipped_ranges: Vec<Range<usize>>,
}

impl<'a> IgnoredErrorVisitor<'a> {
	fn new(path: &Path, content: &'a str) -> Self {
		Self {
			path_str: path.display().to_string(),
			content,
			violations: Vec::new(),
			skipped_ranges: Vec::new(),
		}
	}

	fn is_in_skipped_range(&self, line: usize) -> bool {
		self.skipped_ranges.iter().any(|r| r.contains(&line))
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

/// Macro to implement skip-aware visit methods for container types.
/// If the container has a skip marker (all or for this rule), add its line range to skipped_ranges.
macro_rules! impl_skip_aware_visit {
	($method:ident, $type:ty, $visit_fn:path) => {
		fn $method(&mut self, node: &'a $type) {
			let span = node.span();
			let start_line = span.start().line;
			let end_line = span.end().line;

			if has_skip_marker_for_rule(self.content, span, RULE) {
				self.skipped_ranges.push(start_line..end_line + 1);
				$visit_fn(self, node);
				self.skipped_ranges.pop();
			} else {
				$visit_fn(self, node);
			}
		}
	};
}

impl<'a> Visit<'a> for IgnoredErrorVisitor<'a> {
	// Track skipped regions for various container types
	impl_skip_aware_visit!(visit_item_fn, syn::ItemFn, syn::visit::visit_item_fn);

	impl_skip_aware_visit!(visit_item_mod, syn::ItemMod, syn::visit::visit_item_mod);

	impl_skip_aware_visit!(visit_item_impl, syn::ItemImpl, syn::visit::visit_item_impl);

	impl_skip_aware_visit!(visit_impl_item_fn, syn::ImplItemFn, syn::visit::visit_impl_item_fn);

	impl_skip_aware_visit!(visit_expr_struct, syn::ExprStruct, syn::visit::visit_expr_struct);

	impl_skip_aware_visit!(visit_expr_block, syn::ExprBlock, syn::visit::visit_expr_block);

	fn visit_expr_method_call(&mut self, node: &'a ExprMethodCall) {
		let method_name = node.method.to_string();
		if matches!(method_name.as_str(), "unwrap_or" | "unwrap_or_default" | "unwrap_or_else") {
			let span_start = node.method.span().start();
			// Skip if in a skipped region or has the per-line comment
			if !self.is_in_skipped_range(span_start.line) && !self.has_ignored_error_comment(span_start.line) {
				self.violations.push(Violation {
					rule: RULE,
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
			// Skip if in a skipped region or has the per-line comment
			if !self.is_in_skipped_range(span_start.line) && !self.has_ignored_error_comment(span_start.line) {
				self.violations.push(Violation {
					rule: RULE,
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
