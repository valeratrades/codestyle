//! Lint to flag `fn new` methods that return `Result<...>`.
//!
//! A constructor that can fail should be named `try_new` to signal the fallibility
//! at the call site. `fn new` implies infallible construction.

use std::path::Path;

use syn::{ImplItem, ImplItemFn, ReturnType, Type, visit::Visit};

use super::{Fix, Violation, skip::SkipVisitor};

const RULE: &str = "semantically-try-new";

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let visitor = TryNewVisitor::new(path, content);
	let mut skip_visitor = SkipVisitor::for_rule(visitor, content, RULE);
	skip_visitor.visit_file(file);
	skip_visitor.inner.violations
}

struct TryNewVisitor<'a> {
	path_str: String,
	content: &'a str,
	violations: Vec<Violation>,
}

impl<'a> TryNewVisitor<'a> {
	fn new(path: &Path, content: &'a str) -> Self {
		Self {
			path_str: path.display().to_string(),
			content,
			violations: Vec::new(),
		}
	}

	fn check_fn(&mut self, func: &ImplItemFn) {
		let name = func.sig.ident.to_string();
		if name != "new" {
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
}

impl<'a> Visit<'a> for TryNewVisitor<'a> {
	fn visit_item_impl(&mut self, node: &'a syn::ItemImpl) {
		// Skip trait impls
		if node.trait_.is_some() {
			return;
		}
		for item in &node.items {
			if let ImplItem::Fn(func) = item {
				self.check_fn(func);
			}
		}
		syn::visit::visit_item_impl(self, node);
	}
}

fn returns_result(output: &ReturnType) -> bool {
	let ReturnType::Type(_, ty) = output else {
		return false;
	};
	let Type::Path(type_path) = ty.as_ref() else {
		return false;
	};
	type_path.path.segments.last().is_some_and(|seg| seg.ident == "Result")
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
