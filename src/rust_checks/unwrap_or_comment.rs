//! Lint to require justification comments for `unwrap_or`, `unwrap_or_default`, and `unwrap_or_else` calls.
//!
//! These methods can mask corrupted state by silently providing fallbacks.
//! A comment forces explicit acknowledgment of why a fallback is acceptable.

use std::path::Path;

use syn::{ExprMethodCall, visit::Visit};

use super::Violation;

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let mut visitor = UnwrapOrVisitor::new(path, content);
	visitor.visit_file(file);
	visitor.violations
}

struct UnwrapOrVisitor<'a> {
	path_str: String,
	content: &'a str,
	violations: Vec<Violation>,
}

impl<'a> UnwrapOrVisitor<'a> {
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
}

impl<'a> Visit<'a> for UnwrapOrVisitor<'a> {
	fn visit_expr_method_call(&mut self, node: &'a ExprMethodCall) {
		let method_name = node.method.to_string();
		if matches!(method_name.as_str(), "unwrap_or" | "unwrap_or_default" | "unwrap_or_else") {
			let span_start = node.method.span().start();
			if !self.has_ignored_error_comment(span_start.line) {
				self.violations.push(Violation {
					rule: "unwrap-or-comment",
					file: self.path_str.clone(),
					line: span_start.line,
					column: span_start.column,
					message: format!(
						"`{method_name}` without `//IGNORED_ERROR` comment\n\
						HINT: could the pattern be allowing to continue with corrupted state? Error out properly or explain why it's part of the intended logic."
					),
					fix: None,
				});
			}
		}
		syn::visit::visit_expr_method_call(self, node);
	}
}
