//! Lint to prefer `ahash::AHashMap` over `std::collections::HashMap`.
//!
//! HashMap uses a cryptographically secure hasher (SipHash) by default, which is
//! slower than ahash for typical use cases. This lint replaces:
//! - `use std::collections::HashMap` → `use ahash::AHashMap`
//! - `use std::collections::{..., HashMap, ...}` → remove HashMap from group, add `use ahash::AHashMap;`
//! - `HashMap<K, V>` type references → `AHashMap<K, V>`
//!
//! Declares a dependency on the `ahash` crate.

use std::path::Path;

use syn::{ItemUse, UseTree, spanned::Spanned, visit::Visit};

use super::{Dependency, Fix, Violation, skip::SkipVisitor};

const RULE: &str = "prefer-ahash";
pub const DEPENDENCIES: &[Dependency] = &[Dependency { crate_name: "ahash", features: &[] }];

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let visitor = AHashVisitor::new(path, content);
	let mut skip_visitor = SkipVisitor::for_rule(visitor, content, RULE);
	skip_visitor.visit_file(file);
	skip_visitor.inner.violations
}

struct AHashVisitor<'a> {
	path_str: String,
	content: &'a str,
	violations: Vec<Violation>,
}

impl<'a> AHashVisitor<'a> {
	fn new(path: &Path, content: &'a str) -> Self {
		Self {
			path_str: path.display().to_string(),
			content,
			violations: Vec::new(),
		}
	}
}

impl<'a> Visit<'a> for AHashVisitor<'_> {
	fn visit_item_use(&mut self, node: &'a ItemUse) {
		if let Some(fix) = detect_use_fix(self.content, node) {
			self.violations.push(Violation {
				rule: RULE,
				file: self.path_str.clone(),
				line: node.span().start().line,
				column: node.span().start().column,
				message: "use `ahash::AHashMap` instead of `std::collections::HashMap`".to_string(),
				fix: Some(fix),
			});
		}
		syn::visit::visit_item_use(self, node);
	}

	fn visit_type_path(&mut self, node: &'a syn::TypePath) {
		if let Some(fix) = detect_type_path_fix(self.content, node) {
			self.violations.push(Violation {
				rule: RULE,
				file: self.path_str.clone(),
				line: node.span().start().line,
				column: node.span().start().column,
				message: "use `AHashMap` instead of `HashMap`".to_string(),
				fix: Some(fix),
			});
		}
		syn::visit::visit_type_path(self, node);
	}

	fn visit_expr_path(&mut self, node: &'a syn::ExprPath) {
		if let Some(fix) = detect_expr_path_fix(self.content, node) {
			self.violations.push(Violation {
				rule: RULE,
				file: self.path_str.clone(),
				line: node.span().start().line,
				column: node.span().start().column,
				message: "use `AHashMap` instead of `HashMap`".to_string(),
				fix: Some(fix),
			});
		}
		syn::visit::visit_expr_path(self, node);
	}
}

/// Detect a fixable `use std::collections::HashMap` or group use containing HashMap.
fn detect_use_fix(content: &str, node: &ItemUse) -> Option<Fix> {
	// Check for `use std::collections::HashMap`
	if is_exact_std_collections_hashmap(&node.tree) {
		let start = span_to_byte(content, node.span().start())?;
		let end = span_to_byte(content, node.span().end())?;
		// Consume trailing newline
		let end = if content.as_bytes().get(end) == Some(&b'\n') { end + 1 } else { end };
		return Some(Fix {
			start_byte: start,
			end_byte: end,
			replacement: "use ahash::AHashMap;\n".to_string(),
		});
	}

	// Check for `use std::collections::{..., HashMap, ...}`
	if let Some((hashmap_start, hashmap_end)) = find_hashmap_in_group(content, &node.tree) {
		// Remove `, HashMap` or `HashMap, ` from the group, add separate use after the statement
		let use_start = span_to_byte(content, node.span().start())?;
		let use_end = span_to_byte(content, node.span().end())?;
		let use_end_with_newline = if content.as_bytes().get(use_end) == Some(&b'\n') { use_end + 1 } else { use_end };

		let original_use = &content[use_start..use_end];

		// Remove the HashMap from the group
		let hashmap_rel_start = hashmap_start - use_start;
		let hashmap_rel_end = hashmap_end - use_start;
		let new_use = remove_from_group(original_use, hashmap_rel_start, hashmap_rel_end);
		let new_use = new_use.trim_end().to_string();

		return Some(Fix {
			start_byte: use_start,
			end_byte: use_end_with_newline,
			replacement: format!("{new_use}\nuse ahash::AHashMap;\n"),
		});
	}

	None
}

/// Returns true if the use tree is exactly `std::collections::HashMap`.
fn is_exact_std_collections_hashmap(tree: &UseTree) -> bool {
	match tree {
		UseTree::Path(p) if p.ident == "std" => match p.tree.as_ref() {
			UseTree::Path(p2) if p2.ident == "collections" => match p2.tree.as_ref() {
				UseTree::Name(n) => n.ident == "HashMap",
				_ => false,
			},
			_ => false,
		},
		_ => false,
	}
}

/// If the use tree is `std::collections::{..., HashMap, ...}`, returns the byte range
/// of the `HashMap` ident within the source content.
fn find_hashmap_in_group(content: &str, tree: &UseTree) -> Option<(usize, usize)> {
	match tree {
		UseTree::Path(p) if p.ident == "std" => match p.tree.as_ref() {
			UseTree::Path(p2) if p2.ident == "collections" => match p2.tree.as_ref() {
				UseTree::Group(group) => {
					for item in &group.items {
						if let UseTree::Name(n) = item {
							if n.ident == "HashMap" {
								let start = span_to_byte(content, n.ident.span().start())?;
								let end = span_to_byte(content, n.ident.span().end())?;
								return Some((start, end));
							}
						}
					}
					None
				}
				_ => None,
			},
			_ => None,
		},
		_ => None,
	}
}

/// Remove the item at [rel_start..rel_end] from the source, stripping a surrounding comma+space.
fn remove_from_group(source: &str, rel_start: usize, rel_end: usize) -> String {
	// Try to remove `, HashMap` (trailing comma pattern)
	if rel_start >= 2 {
		let before = &source[..rel_start];
		// Look for ", " or "," before
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
	// Try to remove `HashMap, ` (leading comma pattern)
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

/// Detect a fixable `HashMap` in an expression path (e.g. `HashMap::new()`).
fn detect_expr_path_fix(content: &str, node: &syn::ExprPath) -> Option<Fix> {
	let path = &node.path;

	// `HashMap::...` — first segment is HashMap
	if path.segments.first().is_some_and(|s| s.ident == "HashMap") {
		let seg = &path.segments[0];
		let start = span_to_byte(content, seg.ident.span().start())?;
		let end = span_to_byte(content, seg.ident.span().end())?;
		return Some(Fix {
			start_byte: start,
			end_byte: end,
			replacement: "AHashMap".to_string(),
		});
	}

	// `std::collections::HashMap::...`
	if path.segments.len() >= 3 && path.segments[0].ident == "std" && path.segments[1].ident == "collections" && path.segments[2].ident == "HashMap" {
		let first_start = span_to_byte(content, path.segments[0].ident.span().start())?;
		let last_end = span_to_byte(content, path.segments[2].ident.span().end())?;
		return Some(Fix {
			start_byte: first_start,
			end_byte: last_end,
			replacement: "AHashMap".to_string(),
		});
	}

	None
}

/// Detect a fixable `HashMap` type reference (not from ahash).
fn detect_type_path_fix(content: &str, node: &syn::TypePath) -> Option<Fix> {
	// Must be a simple `HashMap` or `std::collections::HashMap` path
	let path = &node.path;

	// Simple `HashMap<...>` (single segment)
	if path.segments.len() == 1 && path.segments[0].ident == "HashMap" {
		let seg = &path.segments[0];
		let start = span_to_byte(content, seg.ident.span().start())?;
		let end = span_to_byte(content, seg.ident.span().end())?;
		return Some(Fix {
			start_byte: start,
			end_byte: end,
			replacement: "AHashMap".to_string(),
		});
	}

	// `std::collections::HashMap<...>`
	if path.segments.len() == 3 && path.segments[0].ident == "std" && path.segments[1].ident == "collections" && path.segments[2].ident == "HashMap" {
		// Replace the entire `std::collections::HashMap` prefix with `AHashMap`
		let first_start = span_to_byte(content, path.segments[0].ident.span().start())?;
		let last_end = span_to_byte(content, path.segments[2].ident.span().end())?;
		return Some(Fix {
			start_byte: first_start,
			end_byte: last_end,
			replacement: "AHashMap".to_string(),
		});
	}

	None
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
