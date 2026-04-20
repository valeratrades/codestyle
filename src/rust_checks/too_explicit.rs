//! Rewrites unnecessarily verbose inline fully-qualified standard library paths to short forms.
//!
//! For example, `std::sync::Arc<T>` in code becomes `Arc<T>`, with `use std::sync::Arc;`
//! inserted at the top of the file if absent.
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

	let after_first_use = path_rewrite::first_use_line_start(content);

	for rewrite in REWRITES {
		let short_name = rewrite.short_name;
		let has_import = content.lines().any(|l| l.starts_with("use ") && l.contains(short_name));
		// Also trigger import when a previous fix introduced a bare name that's now in code
		// without a corresponding import (e.g. after rewriting `std::sync::Arc` → `Arc`).
		let needs_import =
			violations.iter().any(|v| v.fix.as_ref().is_some_and(|f| f.replacement == short_name)) || (!has_import && path_rewrite::bare_name_in_non_use_lines(content, short_name));

		if needs_import && !has_import {
			violations.push(Violation {
				rule: RULE,
				file: path.display().to_string(),
				line: 1,
				column: 0,
				message: format!("missing `{}` import", rewrite.use_import.trim()),
				fix: Some(Fix {
					start_byte: after_first_use,
					end_byte: after_first_use,
					replacement: rewrite.use_import.to_string(),
				}),
			});
		}
	}

	violations
}

struct TooExplicitVisitor<'a> {
	path_str: String,
	content: &'a str,
	violations: Vec<Violation>,
}

impl<'a> TooExplicitVisitor<'a> {
	fn new(path: &Path, content: &'a str) -> Self {
		Self {
			path_str: path.display().to_string(),
			content,
			violations: Vec::default(),
		}
	}
}

impl<'a> Visit<'a> for TooExplicitVisitor<'_> {
	fn visit_type_path(&mut self, node: &'a syn::TypePath) {
		for rewrite in REWRITES {
			if let Some(fix) = path_rewrite::rewrite_full_type_path(self.content, node, rewrite.segments, rewrite.short_name) {
				self.violations.push(Violation {
					rule: RULE,
					file: self.path_str.clone(),
					line: node.span().start().line,
					column: node.span().start().column,
					message: format!("use `{}` instead of `{}`", rewrite.short_name, rewrite.segments.join("::")),
					fix: Some(fix),
				});
				return;
			}
		}
		syn::visit::visit_type_path(self, node);
	}

	fn visit_expr_path(&mut self, node: &'a syn::ExprPath) {
		for rewrite in REWRITES {
			if let Some(fix) = path_rewrite::rewrite_full_expr_path(self.content, node, rewrite.segments, rewrite.short_name) {
				self.violations.push(Violation {
					rule: RULE,
					file: self.path_str.clone(),
					line: node.span().start().line,
					column: node.span().start().column,
					message: format!("use `{}` instead of `{}`", rewrite.short_name, rewrite.segments.join("::")),
					fix: Some(fix),
				});
				return;
			}
		}
		syn::visit::visit_expr_path(self, node);
	}
}
