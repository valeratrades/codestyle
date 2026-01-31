use std::{collections::HashSet, path::Path};

use proc_macro2::{Span, TokenTree};
use syn::{ExprMacro, ItemFn, Macro, spanned::Spanned, visit::Visit};

use super::{Fix, Violation, skip::SkipVisitor};

pub fn check(path: &Path, content: &str, file: &syn::File, is_format_mode: bool) -> Vec<Violation> {
	let visitor = InstaSnapshotVisitor::new(path, content, is_format_mode);
	let mut skip_visitor = SkipVisitor::new(visitor, content);
	skip_visitor.visit_file(file);
	let mut violations = skip_visitor.inner.violations;

	// Check for sequential snapshots in functions
	let seq_visitor = SequentialSnapshotVisitor::new(path);
	let mut seq_skip_visitor = SkipVisitor::new(seq_visitor, content);
	seq_skip_visitor.visit_file(file);
	violations.extend(seq_skip_visitor.inner.violations);

	violations
}
const INSTA_SNAPSHOT_MACROS: &[&str] = &[
	"assert_snapshot",
	"assert_debug_snapshot",
	"assert_display_snapshot",
	"assert_json_snapshot",
	"assert_yaml_snapshot",
	"assert_ron_snapshot",
	"assert_toml_snapshot",
	"assert_csv_snapshot",
	"assert_compact_json_snapshot",
	"assert_compact_debug_snapshot",
];

struct InstaSnapshotVisitor<'a> {
	path_str: String,
	content: &'a str,
	violations: Vec<Violation>,
	seen_spans: HashSet<(usize, usize)>,
	is_format_mode: bool,
}

impl<'a> InstaSnapshotVisitor<'a> {
	fn new(path: &Path, content: &'a str, is_format_mode: bool) -> Self {
		Self {
			path_str: path.display().to_string(),
			content,
			violations: Vec::new(),
			seen_spans: HashSet::new(),
			is_format_mode,
		}
	}

	fn check_insta_macro(&mut self, mac: &Macro) {
		let start = mac.span().start();
		let key = (start.line, start.column);
		if self.seen_spans.contains(&key) {
			return;
		}
		self.seen_spans.insert(key);

		let macro_name = mac.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();

		if !INSTA_SNAPSHOT_MACROS.contains(&macro_name.as_str()) {
			return;
		}

		// Check if this is insta:: prefixed or just the macro name
		let is_insta_macro = mac.path.segments.len() == 1 || (mac.path.segments.len() == 2 && mac.path.segments.first().map(|s| s.ident.to_string()).as_deref() == Some("insta"));

		if !is_insta_macro {
			return;
		}

		self.analyze_insta_macro(mac, &macro_name);
	}

	fn analyze_insta_macro(&mut self, mac: &Macro, macro_name: &str) {
		let tokens: Vec<TokenTree> = mac.tokens.clone().into_iter().collect();

		// Find if there's an @"..." or @r"..." or @r#"..."# inline snapshot
		let has_inline_snapshot = find_inline_snapshot(&tokens).is_some();

		if !has_inline_snapshot {
			// No inline snapshot found - this is a violation
			// In format mode, we provide a fix to add @""
			let fix = if self.is_format_mode { create_add_inline_snapshot_fix(mac, self.content) } else { None };
			self.violations.push(Violation {
				rule: "insta-inline-snapshot",
				file: self.path_str.clone(),
				line: start_line(mac.span()),
				column: start_column(mac.span()),
				message: format!("`{macro_name}!` must use inline snapshot with `@r\"\"` or `@\"\"`"),
				fix,
			});
		}
		// If it has an inline snapshot (empty or not), it's correct - never touch it
	}
}

impl<'a> Visit<'a> for InstaSnapshotVisitor<'a> {
	fn visit_expr_macro(&mut self, node: &'a ExprMacro) {
		self.check_insta_macro(&node.mac);
		syn::visit::visit_expr_macro(self, node);
	}

	fn visit_macro(&mut self, node: &'a Macro) {
		self.check_insta_macro(node);
		syn::visit::visit_macro(self, node);
	}
}

fn start_line(span: Span) -> usize {
	span.start().line
}

fn start_column(span: Span) -> usize {
	span.start().column
}

/// Find inline snapshot in tokens: looks for @ followed by a string literal
fn find_inline_snapshot(tokens: &[TokenTree]) -> Option<()> {
	for (i, token) in tokens.iter().enumerate() {
		if let TokenTree::Punct(p) = token
			&& p.as_char() == '@'
			&& let Some(TokenTree::Literal(lit)) = tokens.get(i + 1)
		{
			let lit_str = lit.to_string();
			if lit_str.starts_with('"') || lit_str.starts_with("r#") || lit_str.starts_with("r\"") {
				return Some(());
			}
		}
	}
	None
}

fn create_add_inline_snapshot_fix(mac: &Macro, content: &str) -> Option<Fix> {
	let span = mac.span();
	let lines: Vec<&str> = content.lines().collect();
	let end_line_idx = span.end().line - 1;

	if end_line_idx >= lines.len() {
		return None;
	}

	let line = lines[end_line_idx];

	// Find the closing ) of the macro on this line
	// The macro span ends at the closing ), we need to insert before it
	let end_col = span.end().column;

	// Calculate byte position
	let mut line_start_byte = 0;
	for (i, l) in lines.iter().enumerate() {
		if i == end_line_idx {
			break;
		}
		line_start_byte += l.len() + 1;
	}

	// Find the closing parenthesis position
	// We want to insert `, @""` before the closing )
	let closing_paren_pos = if end_col > 0 && end_col <= line.len() {
		// span.end() usually points just after the ), so we need the position of )
		let pos = line_start_byte + end_col - 1;
		// Verify it's actually a )
		if content.as_bytes().get(pos) == Some(&b')') {
			Some(pos)
		} else {
			// Search backwards for )
			find_closing_paren_before(content, line_start_byte + end_col)
		}
	} else {
		find_closing_paren_before(content, line_start_byte + line.len())
	};

	let paren_pos = closing_paren_pos?;

	// Check if there's content before the ), if so we need a comma
	let before_paren = &content[..paren_pos];
	let needs_comma = !before_paren.trim_end().ends_with('(') && !before_paren.trim_end().ends_with(',');

	let replacement = if needs_comma { ", @\"\")" } else { "@\"\")" };

	Some(Fix {
		start_byte: paren_pos,
		end_byte: paren_pos + 1, // Replace the )
		replacement: replacement.to_string(),
	})
}

fn find_closing_paren_before(content: &str, max_pos: usize) -> Option<usize> {
	let search_start = max_pos.saturating_sub(50);
	for (i, c) in content[search_start..max_pos].char_indices().rev() {
		if c == ')' {
			return Some(search_start + i);
		}
	}
	None
}

/// Visitor that detects sequential snapshot assertions within the same function
struct SequentialSnapshotVisitor {
	path_str: String,
	violations: Vec<Violation>,
}

impl SequentialSnapshotVisitor {
	fn new(path: &Path) -> Self {
		Self {
			path_str: path.display().to_string(),
			violations: Vec::new(),
		}
	}

	fn is_insta_snapshot_macro(mac: &Macro) -> bool {
		let macro_name = mac.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();

		if !INSTA_SNAPSHOT_MACROS.contains(&macro_name.as_str()) {
			return false;
		}

		// Check if this is insta:: prefixed or just the macro name
		mac.path.segments.len() == 1 || (mac.path.segments.len() == 2 && mac.path.segments.first().map(|s| s.ident.to_string()).as_deref() == Some("insta"))
	}

	fn check_function_for_sequential_snapshots(&mut self, func: &ItemFn) {
		// Collect all snapshot macros in the function
		let mut collector = SnapshotCollector::default();
		collector.visit_block(&func.block);

		if collector.snapshots.len() > 1 {
			// Report violation on each snapshot after the first
			let first_line = collector.snapshots[0].0;
			for (line, column) in collector.snapshots.into_iter().skip(1) {
				self.violations.push(Violation {
					rule: "insta-sequential-snapshots",
					file: self.path_str.clone(),
					line,
					column,
					message: format!(
						"multiple snapshot assertions in one test (first at line {first_line}); \
						join tested strings together or split into separate tests"
					),
					fix: None,
				});
			}
		}
	}
}

impl<'a> Visit<'a> for SequentialSnapshotVisitor {
	fn visit_item_fn(&mut self, node: &'a ItemFn) {
		self.check_function_for_sequential_snapshots(node);
		syn::visit::visit_item_fn(self, node);
	}
}

/// Collects all insta snapshot macro positions within a block (recursively)
#[derive(Default)]
struct SnapshotCollector {
	snapshots: Vec<(usize, usize)>, // (line, column)
}

impl<'a> Visit<'a> for SnapshotCollector {
	fn visit_expr_macro(&mut self, node: &'a ExprMacro) {
		if SequentialSnapshotVisitor::is_insta_snapshot_macro(&node.mac) {
			let span = node.mac.span();
			self.snapshots.push((span.start().line, span.start().column));
		}
		syn::visit::visit_expr_macro(self, node);
	}

	fn visit_macro(&mut self, node: &'a Macro) {
		if SequentialSnapshotVisitor::is_insta_snapshot_macro(node) {
			let span = node.span();
			self.snapshots.push((span.start().line, span.start().column));
		}
		syn::visit::visit_macro(self, node);
	}

	// Don't descend into nested functions - they have their own scope
	fn visit_item_fn(&mut self, _node: &'a ItemFn) {}
}
