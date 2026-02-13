//! Lint to check that test functions don't have a `test_` prefix.
//!
//! Functions with `#[test]`, `#[rstest]`, or `#[tokio::test]` attributes
//! shouldn't have a `test_` prefix as it's tautological.

use std::path::Path;

use syn::{Attribute, ItemFn, visit::Visit};

use super::{Fix, Violation, skip::SkipVisitor};

const RULE: &str = "test-fn-prefix";
pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let visitor = TestFnPrefixVisitor::new(path, content);
	let mut skip_visitor = SkipVisitor::for_rule(visitor, content, RULE);
	skip_visitor.visit_file(file);
	skip_visitor.inner.violations
}

struct TestFnPrefixVisitor<'a> {
	path_str: String,
	content: &'a str,
	violations: Vec<Violation>,
}

impl<'a> TestFnPrefixVisitor<'a> {
	fn new(path: &Path, content: &'a str) -> Self {
		Self {
			path_str: path.display().to_string(),
			content,
			violations: Vec::new(),
		}
	}

	fn check_fn(&mut self, func: &ItemFn) {
		if !has_test_attr(func) {
			return;
		}

		let fn_name = func.sig.ident.to_string();
		if !fn_name.starts_with("test_") {
			return;
		}

		let new_name = fn_name.strip_prefix("test_").unwrap();
		let span = func.sig.ident.span();

		let fix = span_to_byte(self.content, span.start()).and_then(|start| {
			span_to_byte(self.content, span.end()).map(|end| Fix {
				start_byte: start,
				end_byte: end,
				replacement: new_name.to_string(),
			})
		});

		self.violations.push(Violation {
			rule: RULE,
			file: self.path_str.clone(),
			line: span.start().line,
			column: span.start().column,
			message: format!("test function `{fn_name}` has redundant `test_` prefix"),
			fix,
		});
	}
}

impl<'a> Visit<'a> for TestFnPrefixVisitor<'a> {
	fn visit_item_fn(&mut self, node: &'a ItemFn) {
		self.check_fn(node);
		syn::visit::visit_item_fn(self, node);
	}
}

fn has_test_attr(func: &ItemFn) -> bool {
	func.attrs.iter().any(is_test_attr)
}

fn is_test_attr(attr: &Attribute) -> bool {
	let path = attr.path();

	// Check for #[test]
	if path.is_ident("test") {
		return true;
	}

	// Check for #[rstest]
	if path.is_ident("rstest") {
		return true;
	}

	// Check for #[tokio::test] or similar paths ending in "test"
	if let Some(last) = path.segments.last()
		&& last.ident == "test"
	{
		return true;
	}

	false
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
