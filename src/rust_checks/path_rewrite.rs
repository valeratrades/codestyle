//! Shared utilities for rules that detect and rewrite fully-qualified paths.

use syn::{ExprPath, TypePath, UseTree, spanned::Spanned, visit::Visit};

use super::Fix;

/// Converts a `LineColumn` position to a byte offset in `content`.
pub fn span_to_byte(content: &str, pos: proc_macro2::LineColumn) -> Option<usize> {
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

/// Returns the byte offset of the start of the first `use ` line, or 0 if no such line exists.
pub fn first_use_line_start(content: &str) -> usize {
	let mut pos = 0;
	for line in content.lines() {
		if line.starts_with("use ") {
			return pos;
		}
		pos += line.len() + 1;
	}
	0
}

/// Byte offset and leading-whitespace prefix for inserting a `use` line into a scope.
#[derive(Clone)]
pub struct ScopeInsert {
	pub byte: usize,
	pub indent: String,
}

/// Computes where to insert a new `use` line inside a scope defined by `items`.
///
/// Walks to the first `use` item (or first item if none), finds the start of its line, and
/// captures any leading whitespace as the indentation prefix for the new import.
pub fn scope_insert_for_items(content: &str, items: &[syn::Item]) -> Option<ScopeInsert> {
	let target = items.iter().find(|i| matches!(i, syn::Item::Use(_))).or_else(|| items.first())?;
	let target_byte = span_to_byte(content, target.span().start())?;
	let line_start = content[..target_byte].rfind('\n').map(|i| i + 1).unwrap_or(0);
	let indent = content[line_start..target_byte].to_string();
	Some(ScopeInsert { byte: line_start, indent })
}

/// Returns `true` if `short_name` is imported within the given scope items.
///
/// Recurses into function bodies but NOT into child `mod` blocks (those are separate scopes with
/// their own import namespaces).
pub fn is_name_imported_in_scope(items: &[syn::Item], short_name: &str) -> bool {
	struct V<'a> {
		name: &'a str,
		found: bool,
	}
	impl<'ast> Visit<'ast> for V<'_> {
		fn visit_item_mod(&mut self, _: &'ast syn::ItemMod) {}

		fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
			if !self.found && use_tree_contains_name(&node.tree, self.name) {
				self.found = true;
			}
		}
	}
	let mut v = V { name: short_name, found: false };
	for item in items {
		if v.found {
			break;
		}
		v.visit_item(item);
	}
	v.found
}

/// Returns `true` if `short_name` appears as a bare identifier within the given scope items.
///
/// Recurses into function bodies but NOT into child `mod` blocks.
pub fn has_bare_usage_in_scope(items: &[syn::Item], short_name: &str) -> bool {
	struct V<'a> {
		name: &'a str,
		found: bool,
	}
	impl<'ast> Visit<'ast> for V<'_> {
		fn visit_item_mod(&mut self, _: &'ast syn::ItemMod) {}

		fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
			if self.found {
				return;
			}
			if node.path.segments.len() == 1 && node.path.segments[0].ident == self.name {
				self.found = true;
				return;
			}
			syn::visit::visit_type_path(self, node);
		}

		fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
			if self.found {
				return;
			}
			if node.path.segments.first().is_some_and(|s| s.ident == self.name) {
				self.found = true;
				return;
			}
			syn::visit::visit_expr_path(self, node);
		}
	}
	let mut v = V { name: short_name, found: false };
	for item in items {
		if v.found {
			break;
		}
		v.visit_item(item);
	}
	v.found
}

/// Remove the item at `[rel_start..rel_end]` from the source, stripping a surrounding comma+space.
pub fn remove_from_group(source: &str, rel_start: usize, rel_end: usize) -> String {
	// Try to remove `, Name` (trailing comma pattern)
	if rel_start >= 2 {
		let before = &source[..rel_start];
		if before.ends_with(", ") {
			let mut result = source[..rel_start - 2].to_string();
			result.push_str(&source[rel_end..]);
			return result;
		}
		if before.ends_with(',') {
			let mut result = source[..rel_start - 1].to_string();
			result.push_str(&source[rel_end..]);
			return result;
		}
	}
	// Try to remove `Name, ` (leading comma pattern)
	let after = &source[rel_end..];
	if after.starts_with(", ") {
		let mut result = source[..rel_start].to_string();
		result.push_str(&source[rel_end + 2..]);
		return result;
	}
	if after.starts_with(',') {
		let mut result = source[..rel_start].to_string();
		result.push_str(&source[rel_end + 1..]);
		return result;
	}
	// Sole item — shouldn't happen if there's a group, but handle gracefully
	let mut result = source[..rel_start].to_string();
	result.push_str(&source[rel_end..]);
	result
}

fn rewrite_path_segments(content: &str, path: &syn::Path, segments: &[&str], short_name: &str, exact: bool) -> Option<Fix> {
	if exact {
		if path.segments.len() != segments.len() {
			return None;
		}
	} else if path.segments.len() < segments.len() {
		return None;
	}
	for (seg, expected) in path.segments.iter().zip(segments) {
		if seg.ident != *expected {
			return None;
		}
	}
	let first_start = span_to_byte(content, path.segments[0].ident.span().start())?;
	let last_end = span_to_byte(content, path.segments[segments.len() - 1].ident.span().end())?;
	Some(Fix {
		start_byte: first_start,
		end_byte: last_end,
		replacement: short_name.to_string(),
	})
}

/// If `node` exactly matches the given `segments`, returns a [`Fix`] replacing the full path
/// with `short_name`.
///
/// Example: `segments = ["std", "sync", "Arc"]`, `short_name = "Arc"` matches `std::sync::Arc<T>`.
pub fn rewrite_full_type_path(content: &str, node: &TypePath, segments: &[&str], short_name: &str) -> Option<Fix> {
	rewrite_path_segments(content, &node.path, segments, short_name, true)
}

/// Returns `true` if `short_name` is imported anywhere in `file` (top-level or nested in mods /
/// function bodies / cfg blocks, etc.).
///
/// Handles single-line (`use std::path::PathBuf;`) and multi-line / grouped imports
/// (`use std::{path::{Path, PathBuf}, ...}`) correctly by walking the full syn AST.
pub fn is_name_imported(file: &syn::File, short_name: &str) -> bool {
	struct ImportVisitor<'a> {
		name: &'a str,
		found: bool,
	}

	impl<'ast> Visit<'ast> for ImportVisitor<'_> {
		fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
			if !self.found && use_tree_contains_name(&node.tree, self.name) {
				self.found = true;
			}
		}
	}

	let mut visitor = ImportVisitor { name: short_name, found: false };
	syn::visit::visit_file(&mut visitor, file);
	visitor.found
}

fn use_tree_contains_name(tree: &UseTree, name: &str) -> bool {
	match tree {
		UseTree::Name(n) => n.ident == name,
		UseTree::Rename(r) => r.rename == name,
		UseTree::Path(p) => use_tree_contains_name(&p.tree, name),
		UseTree::Group(g) => g.items.iter().any(|t| use_tree_contains_name(t, name)),
		UseTree::Glob(_) => false,
	}
}

/// Returns `true` if `short_name` appears as a bare identifier in non-use code in `file`.
///
/// "Bare" means used without a qualifying path prefix — e.g. `Arc<T>` or `Arc::new(…)`.
/// This is needed to detect cases where a previous fix iteration introduced a bare name
/// that now needs an import.
pub fn has_bare_usage(file: &syn::File, short_name: &str) -> bool {
	struct BareNameVisitor<'a> {
		name: &'a str,
		found: bool,
	}

	impl<'ast> Visit<'ast> for BareNameVisitor<'_> {
		fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
			if self.found {
				return;
			}
			if node.path.segments.len() == 1 && node.path.segments[0].ident == self.name {
				self.found = true;
				return;
			}
			syn::visit::visit_type_path(self, node);
		}

		fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
			if self.found {
				return;
			}
			// Matches both `Arc` (standalone) and `Arc::new(…)` (method prefix).
			// Full paths like `std::sync::Arc::new` start with "std", not the short name.
			if node.path.segments.first().is_some_and(|s| s.ident == self.name) {
				self.found = true;
				return;
			}
			syn::visit::visit_expr_path(self, node);
		}
	}

	let mut visitor = BareNameVisitor { name: short_name, found: false };
	syn::visit::visit_file(&mut visitor, file);
	visitor.found
}

/// If `node`'s leading segments match `segments`, returns a [`Fix`] replacing those segments
/// with `short_name`.
///
/// Example: `segments = ["std", "sync", "Arc"]` matches `std::sync::Arc::clone(...)`.
pub fn rewrite_full_expr_path(content: &str, node: &ExprPath, segments: &[&str], short_name: &str) -> Option<Fix> {
	rewrite_path_segments(content, &node.path, segments, short_name, false)
}
