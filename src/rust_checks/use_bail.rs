//! Lint to replace `return Err(eyre!(...))` with `bail!(...)`.
//!
//! This check detects patterns like `return Err(eyre!("message"))` and suggests
//! using `bail!("message")` instead, adding the import if needed.

use std::{collections::HashSet, path::Path};

use proc_macro2::Span;
use syn::{Expr, ExprCall, ExprMacro, ExprReturn, ItemUse, Macro, UseTree, spanned::Spanned, visit::Visit};

use super::{Fix, Violation};

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let mut visitor = UseBailVisitor::new(path, content, file);
	visitor.visit_file(file);
	visitor.violations
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ErrorCrate {
	Eyre,
	ColorEyre,
}

struct UseBailVisitor<'a> {
	path_str: String,
	content: &'a str,
	violations: Vec<Violation>,
	seen_spans: HashSet<(usize, usize)>,
	/// Which error crate is being used (eyre, color_eyre, anyhow)
	error_crate: Option<ErrorCrate>,
	/// Whether bail is already imported
	bail_imported: bool,
	/// The byte position where we can insert an import (end of first use statement for the crate)
	import_insert_position: Option<usize>,
	/// The prefix to use for bail import (e.g., "eyre", "color_eyre::eyre", "anyhow")
	import_prefix: Option<String>,
}

impl<'a> UseBailVisitor<'a> {
	fn new(path: &Path, content: &'a str, file: &syn::File) -> Self {
		let mut visitor = Self {
			path_str: path.display().to_string(),
			content,
			violations: Vec::new(),
			seen_spans: HashSet::new(),
			error_crate: None,
			bail_imported: false,
			import_insert_position: None,
			import_prefix: None,
		};

		// First pass: scan imports to understand what error crate is used
		visitor.scan_imports(file);

		visitor
	}

	fn scan_imports(&mut self, file: &syn::File) {
		for item in &file.items {
			if let syn::Item::Use(use_item) = item {
				self.check_use_for_error_crate(use_item);
			}
		}
	}

	fn check_use_for_error_crate(&mut self, use_item: &ItemUse) {
		self.check_use_tree_for_error_crate(&use_item.tree, "", use_item.span());
	}

	fn check_use_tree_for_error_crate(&mut self, tree: &UseTree, prefix: &str, span: Span) {
		match tree {
			UseTree::Path(path) => {
				let ident = path.ident.to_string();
				let new_prefix = if prefix.is_empty() { ident.clone() } else { format!("{prefix}::{ident}") };

				// Check if this is an error crate import
				if ident == "eyre" && prefix.is_empty() {
					self.error_crate = Some(ErrorCrate::Eyre);
					self.import_prefix = Some("eyre".to_string());
					self.record_import_position(span);
				} else if ident == "color_eyre" && prefix.is_empty() {
					self.error_crate = Some(ErrorCrate::ColorEyre);
					self.import_prefix = Some("color_eyre::eyre".to_string());
					self.record_import_position(span);
				}

				self.check_use_tree_for_error_crate(&path.tree, &new_prefix, span);
			}
			UseTree::Name(name) =>
				if name.ident == "bail" {
					self.bail_imported = true;
				},
			UseTree::Rename(rename) =>
				if rename.ident == "bail" {
					self.bail_imported = true;
				},
			UseTree::Glob(_) => {
				// Glob import might include bail
				self.bail_imported = true;
			}
			UseTree::Group(group) =>
				for item in &group.items {
					self.check_use_tree_for_error_crate(item, prefix, span);
				},
		}
	}

	fn record_import_position(&mut self, span: Span) {
		if self.import_insert_position.is_none() {
			// Find the end of this use statement in the source
			let start_line = span.start().line;
			let mut pos = 0;
			let mut current_line = 1;

			for (i, ch) in self.content.char_indices() {
				if current_line == start_line {
					// Find the semicolon ending this use statement
					if ch == ';' {
						pos = i + 1;
						break;
					}
				}
				if ch == '\n' {
					current_line += 1;
				}
			}

			if pos > 0 {
				self.import_insert_position = Some(pos);
			}
		}
	}

	fn check_return_err(&mut self, return_expr: &ExprReturn) {
		let Some(ref expr) = return_expr.expr else {
			return;
		};

		// Check if it's Err(...)
		let Expr::Call(call) = expr.as_ref() else {
			return;
		};

		if !is_err_call(call) {
			return;
		}

		// Check if the argument is eyre!(...) or anyhow!(...)
		let Some(first_arg) = call.args.first() else {
			return;
		};

		let Expr::Macro(macro_expr) = first_arg else {
			return;
		};

		let macro_name = get_macro_name(&macro_expr.mac);
		if macro_name != "eyre" {
			return;
		}

		// Deduplicate
		let key = (return_expr.span().start().line, return_expr.span().start().column);
		if self.seen_spans.contains(&key) {
			return;
		}
		self.seen_spans.insert(key);

		// Create the fix
		let fix = self.create_fix(return_expr, macro_expr);

		self.violations.push(Violation {
			rule: "use-bail",
			file: self.path_str.clone(),
			line: return_expr.span().start().line,
			column: return_expr.span().start().column,
			message: format!("use `bail!(...)` instead of `return Err({macro_name}!(...))`"),
			fix,
		});
	}

	fn create_fix(&self, return_expr: &ExprReturn, macro_expr: &ExprMacro) -> Option<Fix> {
		// Get the macro content (everything inside eyre!(...))
		let macro_content = macro_expr.mac.tokens.to_string();

		// Calculate byte positions for the return statement
		let return_start = span_to_byte(self.content, return_expr.span().start())?;
		let return_end = span_to_byte(self.content, return_expr.span().end())?;

		// Build the replacement
		let bail_call = format!("bail!({macro_content})");

		// If bail is not imported and we know where to add the import, we need a more complex fix
		// For now, just replace the return statement - we'll handle imports in a second pass
		if !self.bail_imported
			&& let Some(import_pos) = self.import_insert_position
		{
			// We need to add the import
			let import_prefix = self.import_prefix.as_ref()?;
			let import_stmt = format!("\nuse {import_prefix}::bail;");

			// We can only do one fix at a time, so we need to combine them
			// Since the import comes before the return statement, we'll create a fix
			// that modifies from import position to return end
			if import_pos < return_start {
				let between_content = &self.content[import_pos..return_start];
				let replacement = format!("{import_stmt}{between_content}{bail_call}");
				return Some(Fix {
					start_byte: import_pos,
					end_byte: return_end,
					replacement,
				});
			}
		}

		Some(Fix {
			start_byte: return_start,
			end_byte: return_end,
			replacement: bail_call,
		})
	}
}

impl<'a> Visit<'a> for UseBailVisitor<'a> {
	fn visit_expr_return(&mut self, node: &'a ExprReturn) {
		self.check_return_err(node);
		syn::visit::visit_expr_return(self, node);
	}
}

fn is_err_call(call: &ExprCall) -> bool {
	if let Expr::Path(path) = call.func.as_ref()
		&& let Some(segment) = path.path.segments.last()
	{
		return segment.ident == "Err";
	}
	false
}

fn get_macro_name(mac: &Macro) -> String {
	mac.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default()
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
