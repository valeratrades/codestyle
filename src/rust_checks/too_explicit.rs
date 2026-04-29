//! Rewrites unnecessarily verbose inline fully-qualified standard library paths to short forms.
//!
//! For example, `std::sync::Arc<T>` in code becomes `Arc<T>`, with `use std::sync::Arc;`
//! inserted at the top of the enclosing scope (file or inline mod) if absent.
//!
//! Only inline usages are flagged — `use` statements with full paths are already correct style.

use std::path::Path;

use syn::{spanned::Spanned, visit::Visit};

use super::{Fix, Violation, path_rewrite, skip::SkipVisitor};

const RULE: &str = "too-explicit";

struct Rewrite {
	/// Fully-qualified path segments, e.g. `["std", "sync", "Arc"]`.
	segments: &'static [&'static str],
	/// Short name to replace the full path with, e.g. `"Arc"`.
	short_name: &'static str,
	/// The `use` line to insert if the import is missing, e.g. `"use std::sync::Arc;\n"`.
	use_import: &'static str,
}

const REWRITES: &[Rewrite] = &[
	Rewrite {
		segments: &["std", "sync", "Arc"],
		short_name: "Arc",
		use_import: "use std::sync::Arc;\n",
	},
	Rewrite {
		segments: &["std", "sync", "Mutex"],
		short_name: "Mutex",
		use_import: "use std::sync::Mutex;\n",
	},
	Rewrite {
		segments: &["std", "sync", "RwLock"],
		short_name: "RwLock",
		use_import: "use std::sync::RwLock;\n",
	},
	Rewrite {
		segments: &["std", "sync", "OnceLock"],
		short_name: "OnceLock",
		use_import: "use std::sync::OnceLock;\n",
	},
	Rewrite {
		segments: &["std", "path", "PathBuf"],
		short_name: "PathBuf",
		use_import: "use std::path::PathBuf;\n",
	},
	Rewrite {
		segments: &["std", "path", "Path"],
		short_name: "Path",
		use_import: "use std::path::Path;\n",
	},
	Rewrite {
		segments: &["std", "collections", "BTreeMap"],
		short_name: "BTreeMap",
		use_import: "use std::collections::BTreeMap;\n",
	},
	Rewrite {
		segments: &["std", "collections", "BTreeSet"],
		short_name: "BTreeSet",
		use_import: "use std::collections::BTreeSet;\n",
	},
	Rewrite {
		segments: &["std", "collections", "VecDeque"],
		short_name: "VecDeque",
		use_import: "use std::collections::VecDeque;\n",
	},
	Rewrite {
		segments: &["std", "collections", "BinaryHeap"],
		short_name: "BinaryHeap",
		use_import: "use std::collections::BinaryHeap;\n",
	},
];

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let visitor = TooExplicitVisitor::new(path, content);
	let mut skip_visitor = SkipVisitor::for_rule(visitor, content, RULE);
	skip_visitor.visit_file(file);
	let mut violations = skip_visitor.inner.violations;
	let violation_scope_bytes = skip_visitor.inner.violation_scope_bytes;

	let file_scope_byte = path_rewrite::first_use_line_start(content);
	let scopes = enumerate_scopes(content, file, file_scope_byte);

	for rewrite in REWRITES {
		let short_name = rewrite.short_name;
		for scope in &scopes {
			let has_import = path_rewrite::is_name_imported_in_scope(scope.items, short_name);
			// Needs import if: a violation in this scope introduced a bare name, OR a previous
			// fix iteration left a bare name without an import in this scope.
			let needs_import = violation_scope_bytes
				.iter()
				.zip(&violations)
				.any(|(&byte, v)| byte == scope.insert_byte && v.fix.as_ref().is_some_and(|f| f.replacement == short_name))
				|| (!has_import && path_rewrite::has_bare_usage_in_scope(scope.items, short_name));

			if needs_import && !has_import {
				violations.push(Violation {
					rule: RULE,
					file: path.display().to_string(),
					line: 1,
					column: 0,
					message: format!("missing `{}` import", rewrite.use_import.trim()),
					fix: Some(Fix {
						start_byte: scope.insert_byte,
						end_byte: scope.insert_byte,
						replacement: format!("{}{}", scope.indent, rewrite.use_import),
					}),
				});
			}
		}
	}

	violations
}

struct ScopeInfo<'a> {
	insert_byte: usize,
	indent: String,
	items: &'a [syn::Item],
}

fn enumerate_scopes<'a>(content: &str, file: &'a syn::File, file_scope_byte: usize) -> Vec<ScopeInfo<'a>> {
	let mut scopes = vec![ScopeInfo {
		insert_byte: file_scope_byte,
		indent: String::new(),
		items: &file.items,
	}];
	collect_inline_mod_scopes(content, &file.items, &mut scopes);
	scopes
}

fn collect_inline_mod_scopes<'a>(content: &str, items: &'a [syn::Item], out: &mut Vec<ScopeInfo<'a>>) {
	for item in items {
		if let syn::Item::Mod(m) = item
			&& let Some((_, mod_items)) = &m.content
			&& let Some(s) = path_rewrite::scope_insert_for_items(content, mod_items)
		{
			out.push(ScopeInfo {
				insert_byte: s.byte,
				indent: s.indent,
				items: mod_items,
			});
			collect_inline_mod_scopes(content, mod_items, out);
		}
	}
}

struct TooExplicitVisitor<'a> {
	path_str: String,
	content: &'a str,
	violations: Vec<Violation>,
	/// Parallel to `violations`: insert-byte of the scope each violation belongs to.
	violation_scope_bytes: Vec<usize>,
	/// Stack of insert-bytes as we descend into inline mods. Empty = file scope.
	scope_stack: Vec<usize>,
	file_scope_byte: usize,
}

impl<'a> TooExplicitVisitor<'a> {
	fn new(path: &Path, content: &'a str) -> Self {
		Self {
			path_str: path.display().to_string(),
			content,
			violations: Vec::default(),
			violation_scope_bytes: Vec::default(),
			scope_stack: Vec::default(),
			file_scope_byte: path_rewrite::first_use_line_start(content),
		}
	}

	fn current_scope_byte(&self) -> usize {
		self.scope_stack.last().copied().unwrap_or(self.file_scope_byte)
	}

	fn push_rewrite_violation(&mut self, fix: Fix, line: usize, column: usize, rewrite: &Rewrite) {
		self.violation_scope_bytes.push(self.current_scope_byte());
		self.violations.push(Violation {
			rule: RULE,
			file: self.path_str.clone(),
			line,
			column,
			message: format!("use `{}` instead of `{}`", rewrite.short_name, rewrite.segments.join("::")),
			fix: Some(fix),
		});
	}
}

impl<'a> Visit<'a> for TooExplicitVisitor<'a> {
	fn visit_item_mod(&mut self, node: &'a syn::ItemMod) {
		if let Some((_, items)) = &node.content
			&& let Some(s) = path_rewrite::scope_insert_for_items(self.content, items)
		{
			self.scope_stack.push(s.byte);
			syn::visit::visit_item_mod(self, node);
			self.scope_stack.pop();
			return;
		}
		syn::visit::visit_item_mod(self, node);
	}

	fn visit_type_path(&mut self, node: &'a syn::TypePath) {
		for rewrite in REWRITES {
			if let Some(fix) = path_rewrite::rewrite_full_type_path(self.content, node, rewrite.segments, rewrite.short_name) {
				self.push_rewrite_violation(fix, node.span().start().line, node.span().start().column, rewrite);
				return;
			}
		}
		syn::visit::visit_type_path(self, node);
	}

	fn visit_expr_path(&mut self, node: &'a syn::ExprPath) {
		for rewrite in REWRITES {
			if let Some(fix) = path_rewrite::rewrite_full_expr_path(self.content, node, rewrite.segments, rewrite.short_name) {
				self.push_rewrite_violation(fix, node.span().start().line, node.span().start().column, rewrite);
				return;
			}
		}
		syn::visit::visit_expr_path(self, node);
	}
}
