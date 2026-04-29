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

use super::{Dependency, Fix, Violation, path_rewrite, skip::SkipVisitor};

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

	let has_ahashmap_import = path_rewrite::is_name_imported(file, "AHashMap");
	let has_ahashset_import = path_rewrite::is_name_imported(file, "AHashSet");

	// We need an import if we're rewriting something to a bare name, OR if the file already
	// uses bare AHashMap/AHashSet without a qualifying path (introduced by a previous fix iteration).
	let needs_ahashmap_import =
		violations.iter().any(|v| v.fix.as_ref().is_some_and(|f| f.replacement == "AHashMap")) || (!has_ahashmap_import && path_rewrite::has_bare_usage(file, "AHashMap"));
	let needs_ahashset_import =
		violations.iter().any(|v| v.fix.as_ref().is_some_and(|f| f.replacement == "AHashSet")) || (!has_ahashset_import && path_rewrite::has_bare_usage(file, "AHashSet"));

	// Check if a violation already rewrites a use-statement into the full ahash import
	// (so no additional import line is needed).
	let violation_inserts_ahashmap = violations.iter().any(|v| v.fix.as_ref().is_some_and(|f| f.replacement.contains("use ahash::AHashMap")));
	let violation_inserts_ahashset = violations.iter().any(|v| v.fix.as_ref().is_some_and(|f| f.replacement.contains("use ahash::AHashSet")));

	// Insert after the first `use` line, so we don't break leading doc comments or `mod` declarations.
	let after_first_use = path_rewrite::first_use_line_start(content);

	if needs_ahashmap_import && !has_ahashmap_import && !violation_inserts_ahashmap {
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

	if needs_ahashset_import && !has_ahashset_import && !violation_inserts_ahashset {
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

	fn push_fix_violation(&mut self, fix: Fix, line: usize, column: usize, std_name: &str, ahash_name: &str) {
		self.violations.push(Violation {
			rule: RULE,
			file: self.path_str.clone(),
			line,
			column,
			message: format!("use `{ahash_name}` instead of `{std_name}`"),
			fix: Some(fix),
		});
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
				self.push_fix_violation(fix, node.span().start().line, node.span().start().column, std_name, ahash_name);
				return;
			}
		}
		syn::visit::visit_type_path(self, node);
	}

	fn visit_expr_path(&mut self, node: &'a syn::ExprPath) {
		for (std_name, ahash_name) in [("HashMap", "AHashMap"), ("HashSet", "AHashSet")] {
			if let Some(fix) = detect_expr_path_fix(self.content, node, std_name, ahash_name) {
				self.push_fix_violation(fix, node.span().start().line, node.span().start().column, std_name, ahash_name);
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
		let start = path_rewrite::span_to_byte(content, node.span().start())?;
		let end = path_rewrite::span_to_byte(content, node.span().end())?;
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
		let use_start = path_rewrite::span_to_byte(content, node.span().start())?;
		let use_end = path_rewrite::span_to_byte(content, node.span().end())?;
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
		let new_use = path_rewrite::remove_from_group(original_use, item_rel_start, item_rel_end);
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
							let start = path_rewrite::span_to_byte(content, n.ident.span().start())?;
							let end = path_rewrite::span_to_byte(content, n.ident.span().end())?;
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

/// Detect a fixable `<std_name>` in an expression path (e.g. `HashMap::new()`).
fn detect_expr_path_fix(content: &str, node: &syn::ExprPath, std_name: &str, ahash_name: &str) -> Option<Fix> {
	let path = &node.path;

	// `HashMap::...` — first segment is the std name (bare name, not a full path)
	if path.segments.first().is_some_and(|s| s.ident == std_name) {
		let seg = &path.segments[0];
		let start = path_rewrite::span_to_byte(content, seg.ident.span().start())?;
		let end = path_rewrite::span_to_byte(content, seg.ident.span().end())?;
		return Some(Fix {
			start_byte: start,
			end_byte: end,
			replacement: ahash_name.to_string(),
		});
	}

	// `std::collections::HashMap::...`
	let segs = ["std", "collections", std_name];
	path_rewrite::rewrite_full_expr_path(content, node, &segs, ahash_name)
}

/// Detect a fixable `<std_name>` type reference (not from ahash).
fn detect_type_path_fix(content: &str, node: &syn::TypePath, std_name: &str, ahash_name: &str) -> Option<Fix> {
	let path = &node.path;

	// Simple `HashMap<...>` (single segment — bare name)
	if path.segments.len() == 1 && path.segments[0].ident == std_name {
		let seg = &path.segments[0];
		let start = path_rewrite::span_to_byte(content, seg.ident.span().start())?;
		let end = path_rewrite::span_to_byte(content, seg.ident.span().end())?;
		return Some(Fix {
			start_byte: start,
			end_byte: end,
			replacement: ahash_name.to_string(),
		});
	}

	// `std::collections::HashMap<...>`
	let segs = ["std", "collections", std_name];
	path_rewrite::rewrite_full_type_path(content, node, &segs, ahash_name)
}
