//! Lint to disallow usage of the `chrono` crate.
//!
//! The `chrono` crate has known issues and the `jiff` crate is recommended instead.
//! See miette for proper error handling patterns.

use std::{collections::HashSet, path::Path};

use proc_macro2::Span;
use syn::{ItemUse, UseTree, visit::Visit};

use super::{Violation, skip::has_skip_attr};

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let mut visitor = ChronoVisitor::new(path, content);
	visitor.visit_file(file);
	visitor.violations
}

struct ChronoVisitor<'a> {
	path_str: String,
	#[expect(unused)]
	content: &'a str,
	violations: Vec<Violation>,
	seen_spans: HashSet<(usize, usize)>,
}

impl<'a> ChronoVisitor<'a> {
	fn new(path: &Path, content: &'a str) -> Self {
		Self {
			path_str: path.display().to_string(),
			content,
			violations: Vec::new(),
			seen_spans: HashSet::new(),
		}
	}

	fn report_chrono_usage(&mut self, span: Span, context: &str) {
		let key = (span.start().line, span.start().column);
		if self.seen_spans.contains(&key) {
			return;
		}
		self.seen_spans.insert(key);

		self.violations.push(Violation {
			rule: "no-chrono",
			file: self.path_str.clone(),
			line: span.start().line,
			column: span.start().column,
			message: format!("Usage of `chrono` crate is disallowed{context}. Use `jiff` crate instead."),
			fix: None, // No auto-fix - requires manual migration
		});
	}

	fn check_use_tree(&mut self, tree: &UseTree, prefix: &str) {
		match tree {
			UseTree::Path(path) => {
				let ident = path.ident.to_string();
				let new_prefix = if prefix.is_empty() { ident.clone() } else { format!("{prefix}::{ident}") };
				if ident == "chrono" {
					self.report_chrono_usage(path.ident.span(), " in use statement");
				}
				self.check_use_tree(&path.tree, &new_prefix);
			}
			UseTree::Name(name) =>
				if name.ident == "chrono" {
					self.report_chrono_usage(name.ident.span(), " in use statement");
				},
			UseTree::Rename(rename) =>
				if rename.ident == "chrono" {
					self.report_chrono_usage(rename.ident.span(), " in use statement");
				},
			UseTree::Glob(_) => {}
			UseTree::Group(group) =>
				for item in &group.items {
					self.check_use_tree(item, prefix);
				},
		}
	}

	fn check_path_for_chrono(&mut self, path: &syn::Path) {
		if let Some(first_segment) = path.segments.first()
			&& first_segment.ident == "chrono"
		{
			self.report_chrono_usage(first_segment.ident.span(), "");
		}
	}
}

impl<'a> Visit<'a> for ChronoVisitor<'a> {
	fn visit_item_fn(&mut self, node: &'a syn::ItemFn) {
		if has_skip_attr(&node.attrs) {
			return;
		}
		syn::visit::visit_item_fn(self, node);
	}

	fn visit_item_mod(&mut self, node: &'a syn::ItemMod) {
		if has_skip_attr(&node.attrs) {
			return;
		}
		syn::visit::visit_item_mod(self, node);
	}

	fn visit_item_impl(&mut self, node: &'a syn::ItemImpl) {
		if has_skip_attr(&node.attrs) {
			return;
		}
		syn::visit::visit_item_impl(self, node);
	}

	fn visit_expr_block(&mut self, node: &'a syn::ExprBlock) {
		if has_skip_attr(&node.attrs) {
			return;
		}
		syn::visit::visit_expr_block(self, node);
	}

	fn visit_item_use(&mut self, node: &'a ItemUse) {
		if has_skip_attr(&node.attrs) {
			return;
		}
		self.check_use_tree(&node.tree, "");
		syn::visit::visit_item_use(self, node);
	}

	fn visit_type_path(&mut self, node: &'a syn::TypePath) {
		self.check_path_for_chrono(&node.path);
		syn::visit::visit_type_path(self, node);
	}

	fn visit_path(&mut self, node: &'a syn::Path) {
		self.check_path_for_chrono(node);
		syn::visit::visit_path(self, node);
	}
}
