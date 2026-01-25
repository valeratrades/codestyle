//! Lint to require justification comments for `let _ = ...` patterns.
//!
//! Discarding values with `let _ =` can mask errors by silently ignoring Results or other important values.
//! A comment forces explicit acknowledgment of why ignoring the value is acceptable.

use std::path::Path;

use syn::{Pat, PatWild, Stmt, visit::Visit};

use super::Violation;

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let mut visitor = LetUnderscoreVisitor::new(path, content);
	visitor.visit_file(file);
	visitor.violations
}

struct LetUnderscoreVisitor<'a> {
	path_str: String,
	content: &'a str,
	violations: Vec<Violation>,
}

impl<'a> LetUnderscoreVisitor<'a> {
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

impl<'a> Visit<'a> for LetUnderscoreVisitor<'a> {
	fn visit_stmt(&mut self, stmt: &'a Stmt) {
		if let Stmt::Local(local) = stmt
			&& let Some(wild) = self.is_standalone_underscore(&local.pat)
			&& local.init.is_some()
		{
			let span_start = wild.underscore_token.span.start();
			if !self.has_ignored_error_comment(span_start.line) {
				self.violations.push(Violation {
					rule: "let-underscore-comment",
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
