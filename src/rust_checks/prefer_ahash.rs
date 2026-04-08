//! Lint to prefer `ahash::AHashMap`/`ahash::AHashSet` over `std::collections::HashMap`/`HashSet`.
//!
//! HashMap/HashSet use a cryptographically secure hasher (SipHash) by default, which is
//! slower than ahash for typical use cases. This lint replaces:
//! - `use std::collections::HashMap` → `use ahash::AHashMap`
//! - `use std::collections::HashSet` → `use ahash::AHashSet`
//! - `use std::collections::{..., HashMap, ...}` → remove HashMap from group, add `use ahash::AHashMap;`
//! - `use std::collections::{..., HashSet, ...}` → remove HashSet from group, add `use ahash::AHashSet;`
//! - `HashMap<K, V>` type references → `AHashMap<K, V>`
//! - `HashSet<T>` type references → `AHashSet<T>`
//!
//! Also inserts missing `use ahash::AHashMap;` / `use ahash::AHashSet;` imports when bare
//! `AHashMap`/`AHashSet` names appear after conversion.
//!
//! Declares a dependency on the `ahash` crate.

use std::path::Path;

use syn::{ItemUse, UseTree, spanned::Spanned, visit::Visit};

use super::{Dependency, Fix, Violation, skip::SkipVisitor};

const RULE: &str = "prefer-ahash";
pub const DEPENDENCIES: &[Dependency] = &[Dependency {
	crate_name: "ahash",
	features: &["serde"],
}];

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let visitor = AHashVisitor::new(path, content);
	let mut skip_visitor = SkipVisitor::for_rule(visitor, content, RULE);
	skip_visitor.visit_file(file);
	let mut violations = skip_visitor.inner.violations;

	// Collect all `use` lines from leading paragraphs (stop at first paragraph with no `use` lines).
	let has_ahashmap_import = content.contains("use ahash::AHashMap");
	let has_ahashset_import = content.contains("use ahash::AHashSet");

	// Determine if the file uses bare AHashMap/AHashSet anywhere (existing or after fixes).
	// "bare" means not prefixed with `ahash::`.
	let uses_bare_ahashmap = uses_bare_name(content, "AHashMap") || violations.iter().any(|v| v.fix.as_ref().is_some_and(|f| f.replacement == "AHashMap"));
	let uses_bare_ahashset = uses_bare_name(content, "AHashSet") || violations.iter().any(|v| v.fix.as_ref().is_some_and(|f| f.replacement == "AHashSet"));

	// Check if a violation already rewrites a use-statement into the full ahash import
	// (so no additional import line is needed).
	let violation_inserts_ahashmap = violations.iter().any(|v| v.fix.as_ref().is_some_and(|f| f.replacement.contains("use ahash::AHashMap")));
	let violation_inserts_ahashset = violations.iter().any(|v| v.fix.as_ref().is_some_and(|f| f.replacement.contains("use ahash::AHashSet")));

	// Insert after the first `use` line, so we don't break leading doc comments or `mod` declarations.
	let after_first_use = first_use_line_start(content);

	if uses_bare_ahashmap && !has_ahashmap_import && !violation_inserts_ahashmap {
		violations.push(Violation {
			rule: RULE,
			file: path.display().to_string(),
			line: 1,
			column: 0,
			message: "missing `use ahash::AHashMap` import".to_string(),
			fix: Some(Fix {
				start_byte: after_first_use,
				end_byte: after_first_use,
				replacement: "use ahash::AHashMap;\n".to_string(),
			}),
		});
	}

	if uses_bare_ahashset && !has_ahashset_import && !violation_inserts_ahashset {
		violations.push(Violation {
			rule: RULE,
			file: path.display().to_string(),
			line: 1,
			column: 0,
			message: "missing `use ahash::AHashSet` import".to_string(),
			fix: Some(Fix {
				start_byte: after_first_use,
				end_byte: after_first_use,
				replacement: "use ahash::AHashSet;\n".to_string(),
			}),
		});
	}

	violations
}

/// Returns the byte offset of the start of the first `use ` line, or 0 if no such line exists.
fn first_use_line_start(content: &str) -> usize {
	let mut pos = 0;
	for line in content.lines() {
		if line.starts_with("use ") {
			return pos;
		}
		pos += line.len() + 1;
	}
	0
}

/// Returns true if `name` (e.g. `"AHashMap"`) appears in the content as a bare identifier —
/// i.e. not prefixed by `ahash::`.
fn uses_bare_name(content: &str, name: &str) -> bool {
	let mut pos = 0;
	while let Some(idx) = content[pos..].find(name) {
		let abs = pos + idx;
		// Check it's not preceded by `ahash::` (i.e. already fully qualified)
		let prefix = "ahash::";
		let is_qualified = abs >= prefix.len() && content[abs - prefix.len()..abs] == *prefix;
		if !is_qualified {
			return true;
		}
		pos = abs + name.len();
	}
	false
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
			violations: Vec::default(),
		}
	}
}

impl<'a> Visit<'a> for AHashVisitor<'_> {
	fn visit_item_use(&mut self, node: &'a ItemUse) {
		for (std_name, ahash_replacement, message) in [
			("HashMap", "use ahash::AHashMap;\n", "use `ahash::AHashMap` instead of `std::collections::HashMap`"),
			("HashSet", "use ahash::AHashSet;\n", "use `ahash::AHashSet` instead of `std::collections::HashSet`"),
		] {
			if let Some(fix) = detect_use_fix(self.content, node, std_name, ahash_replacement) {
				self.violations.push(Violation {
					rule: RULE,
					file: self.path_str.clone(),
					line: node.span().start().line,
					column: node.span().start().column,
					message: message.to_string(),
					fix: Some(fix),
				});
				return;
			}
		}

		syn::visit::visit_item_use(self, node);
	}

	fn visit_type_path(&mut self, node: &'a syn::TypePath) {
		for (std_name, ahash_name) in [("HashMap", "AHashMap"), ("HashSet", "AHashSet")] {
			if let Some(fix) = detect_type_path_fix(self.content, node, std_name, ahash_name) {
				self.violations.push(Violation {
					rule: RULE,
					file: self.path_str.clone(),
					line: node.span().start().line,
					column: node.span().start().column,
					message: format!("use `{ahash_name}` instead of `{std_name}`"),
					fix: Some(fix),
				});
				return;
			}
		}
		syn::visit::visit_type_path(self, node);
	}

	fn visit_expr_path(&mut self, node: &'a syn::ExprPath) {
		for (std_name, ahash_name) in [("HashMap", "AHashMap"), ("HashSet", "AHashSet")] {
			if let Some(fix) = detect_expr_path_fix(self.content, node, std_name, ahash_name) {
				self.violations.push(Violation {
					rule: RULE,
					file: self.path_str.clone(),
					line: node.span().start().line,
					column: node.span().start().column,
					message: format!("use `{ahash_name}` instead of `{std_name}`"),
					fix: Some(fix),
				});
				return;
			}
		}
		syn::visit::visit_expr_path(self, node);
	}
}

/// Detect a fixable `use std::collections::<std_name>` or group use containing it.
fn detect_use_fix(content: &str, node: &ItemUse, std_name: &str, ahash_import: &str) -> Option<Fix> {
	// Check for `use std::collections::<std_name>`
	if is_exact_std_collections(std_name, &node.tree) {
		let start = span_to_byte(content, node.span().start())?;
		let end = span_to_byte(content, node.span().end())?;
		// Consume trailing newline
		let end = if content.as_bytes().get(end) == Some(&b'\n') { end + 1 } else { end };
		return Some(Fix {
			start_byte: start,
			end_byte: end,
			replacement: ahash_import.to_string(),
		});
	}

	// Check for `use std::collections::{..., <std_name>, ...}`
	if let Some((item_start, item_end, group_len)) = find_name_in_group(content, &node.tree, std_name) {
		let use_start = span_to_byte(content, node.span().start())?;
		let use_end = span_to_byte(content, node.span().end())?;
		let use_end_with_newline = if content.as_bytes().get(use_end) == Some(&b'\n') { use_end + 1 } else { use_end };

		if group_len == 1 {
			// Sole item in group — remove the entire use statement, just emit the ahash import
			return Some(Fix {
				start_byte: use_start,
				end_byte: use_end_with_newline,
				replacement: ahash_import.to_string(),
			});
		}

		let original_use = &content[use_start..use_end];

		let item_rel_start = item_start - use_start;
		let item_rel_end = item_end - use_start;
		let new_use = remove_from_group(original_use, item_rel_start, item_rel_end);
		let new_use = new_use.trim_end().to_string();

		return Some(Fix {
			start_byte: use_start,
			end_byte: use_end_with_newline,
			replacement: format!("{new_use}\n{ahash_import}"),
		});
	}

	None
}

/// Returns true if the use tree is exactly `std::collections::<name>`.
fn is_exact_std_collections(name: &str, tree: &UseTree) -> bool {
	match tree {
		UseTree::Path(p) if p.ident == "std" => match p.tree.as_ref() {
			UseTree::Path(p2) if p2.ident == "collections" => match p2.tree.as_ref() {
				UseTree::Name(n) => n.ident == name,
				_ => false,
			},
			_ => false,
		},
		_ => false,
	}
}

/// If the use tree is `std::collections::{..., <name>, ...}`, returns the byte range of the ident
/// and the total number of items in the group.
fn find_name_in_group(content: &str, tree: &UseTree, name: &str) -> Option<(usize, usize, usize)> {
	match tree {
		UseTree::Path(p) if p.ident == "std" => match p.tree.as_ref() {
			UseTree::Path(p2) if p2.ident == "collections" => match p2.tree.as_ref() {
				UseTree::Group(group) => {
					for item in &group.items {
						if let UseTree::Name(n) = item
							&& n.ident == name
						{
							let start = span_to_byte(content, n.ident.span().start())?;
							let end = span_to_byte(content, n.ident.span().end())?;
							return Some((start, end, group.items.len()));
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

/// Detect a fixable `<std_name>` in an expression path (e.g. `HashMap::new()`).
fn detect_expr_path_fix(content: &str, node: &syn::ExprPath, std_name: &str, ahash_name: &str) -> Option<Fix> {
	let path = &node.path;

	// `HashMap::...` — first segment is the std name
	if path.segments.first().is_some_and(|s| s.ident == std_name) {
		let seg = &path.segments[0];
		let start = span_to_byte(content, seg.ident.span().start())?;
		let end = span_to_byte(content, seg.ident.span().end())?;
		return Some(Fix {
			start_byte: start,
			end_byte: end,
			replacement: ahash_name.to_string(),
		});
	}

	// `std::collections::HashMap::...`
	if path.segments.len() >= 3 && path.segments[0].ident == "std" && path.segments[1].ident == "collections" && path.segments[2].ident == std_name {
		let first_start = span_to_byte(content, path.segments[0].ident.span().start())?;
		let last_end = span_to_byte(content, path.segments[2].ident.span().end())?;
		return Some(Fix {
			start_byte: first_start,
			end_byte: last_end,
			replacement: ahash_name.to_string(),
		});
	}

	None
}

/// Detect a fixable `<std_name>` type reference (not from ahash).
fn detect_type_path_fix(content: &str, node: &syn::TypePath, std_name: &str, ahash_name: &str) -> Option<Fix> {
	let path = &node.path;

	// Simple `HashMap<...>` (single segment)
	if path.segments.len() == 1 && path.segments[0].ident == std_name {
		let seg = &path.segments[0];
		let start = span_to_byte(content, seg.ident.span().start())?;
		let end = span_to_byte(content, seg.ident.span().end())?;
		return Some(Fix {
			start_byte: start,
			end_byte: end,
			replacement: ahash_name.to_string(),
		});
	}

	// `std::collections::HashMap<...>`
	if path.segments.len() == 3 && path.segments[0].ident == "std" && path.segments[1].ident == "collections" && path.segments[2].ident == std_name {
		let first_start = span_to_byte(content, path.segments[0].ident.span().start())?;
		let last_end = span_to_byte(content, path.segments[2].ident.span().end())?;
		return Some(Fix {
			start_byte: first_start,
			end_byte: last_end,
			replacement: ahash_name.to_string(),
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
