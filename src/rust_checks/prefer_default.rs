//! Lint to flag argument-less `fn new()` methods.
//!
//! An argument-less `new()` is equivalent to `Default::default()`. Using `Default`
//! (via `derive` or an explicit impl) is idiomatic Rust and allows callers to use
//! `..Default::default()` struct-update syntax, `unwrap_or_default()`, etc.

use std::path::Path;

use syn::{ImplItem, ImplItemFn, Visibility, visit::Visit};

use super::{Violation, skip::SkipVisitor};

const RULE: &str = "prefer-default";

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let visitor = PreferDefaultVisitor::new(path);
	let mut skip_visitor = SkipVisitor::for_rule(visitor, content, RULE);
	skip_visitor.visit_file(file);
	skip_visitor.inner.violations
}

struct PreferDefaultVisitor {
	path_str: String,
	violations: Vec<Violation>,
}

impl PreferDefaultVisitor {
	fn new(path: &Path) -> Self {
		Self {
			path_str: path.display().to_string(),
			violations: Vec::new(),
		}
	}

	fn check_fn(&mut self, func: &ImplItemFn) {
		let name = func.sig.ident.to_string();
		if name != "new" {
			return;
		}
		if !func.sig.inputs.is_empty() {
			return;
		}
		let span = func.sig.ident.span();
		self.violations.push(Violation {
			rule: RULE,
			file: self.path_str.clone(),
			line: span.start().line,
			column: span.start().column,
			message: "argument-less `fn new()` found; derive or implement `Default` and use it instead (argument-less `new` is bad practice)".to_string(),
			fix: None,
		});
	}
}

impl<'a> Visit<'a> for PreferDefaultVisitor {
	fn visit_item_impl(&mut self, node: &'a syn::ItemImpl) {
		// Skip trait impls (e.g. `impl Default for Foo`)
		if node.trait_.is_some() {
			return;
		}
		for item in &node.items {
			if let ImplItem::Fn(func) = item {
				// Only flag pub fn new() — private constructors are fine
				if matches!(func.vis, Visibility::Public(_)) {
					self.check_fn(func);
				}
			}
		}
		syn::visit::visit_item_impl(self, node);
	}
}
