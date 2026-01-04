use std::{collections::HashSet, path::Path};

use proc_macro2::{Span, TokenTree};
use syn::{ExprMacro, Macro, spanned::Spanned, visit::Visit};

use super::{Fix, Violation};

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

pub fn check(path: &Path, content: &str, file: &syn::File, is_format_mode: bool) -> Vec<Violation> {
	let mut visitor = InstaSnapshotVisitor::new(path, content, is_format_mode);
	visitor.visit_file(file);
	visitor.violations
}

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
		let inline_snapshot_info = find_inline_snapshot(&tokens);

		match inline_snapshot_info {
			Some((snapshot_span, snapshot_content, is_empty)) => {
				if self.is_format_mode && !is_empty {
					// In format mode, replace non-empty inline snapshot with empty @""
					let fix = create_clear_snapshot_fix(snapshot_span, self.content);
					self.violations.push(Violation {
						rule: "insta-inline-snapshot",
						file: self.path_str.clone(),
						line: snapshot_span.start().line,
						column: snapshot_span.start().column,
						message: format!(
							"`{macro_name}!` has inline snapshot content that will be cleared for formatting (content: {})",
							truncate_content(&snapshot_content, 40)
						),
						fix,
					});
				}
				// If it has an inline snapshot (even empty), it's correct
			}
			None => {
				// No inline snapshot found - this is a violation
				let fix = create_add_inline_snapshot_fix(mac, self.content);
				self.violations.push(Violation {
					rule: "insta-inline-snapshot",
					file: self.path_str.clone(),
					line: start_line(mac.span()),
					column: start_column(mac.span()),
					message: format!("`{macro_name}!` must use inline snapshot with `@r\"\"` or `@\"\"`"),
					fix,
				});
			}
		}
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
/// Returns (span of @ through string, content of string, is_empty)
fn find_inline_snapshot(tokens: &[TokenTree]) -> Option<(Span, String, bool)> {
	for (i, token) in tokens.iter().enumerate() {
		if let TokenTree::Punct(p) = token
			&& p.as_char() == '@'
			&& let Some(TokenTree::Literal(lit)) = tokens.get(i + 1)
		{
			let lit_str = lit.to_string();
			if lit_str.starts_with('"') || lit_str.starts_with("r#") || lit_str.starts_with("r\"") {
				let content = extract_string_content(&lit_str);
				let is_empty = content.trim().is_empty();
				return Some((lit.span(), content, is_empty));
			}
		}
	}
	None
}

fn extract_string_content(lit_str: &str) -> String {
	if lit_str.starts_with("r#") {
		// Raw string with # delimiters: r#"..."#, r##"..."##, etc.
		let hash_count = lit_str.chars().skip(1).take_while(|c| *c == '#').count();
		let start = 2 + hash_count; // r + # * hash_count + "
		let end = lit_str.len() - hash_count - 1; // " + # * hash_count
		if start < end { lit_str[start..end].to_string() } else { String::new() }
	} else if lit_str.starts_with("r\"") {
		// Raw string without #: r"..."
		if lit_str.len() > 3 { lit_str[2..lit_str.len() - 1].to_string() } else { String::new() }
	} else if lit_str.starts_with('"') {
		// Regular string: "..."
		if lit_str.len() > 2 { lit_str[1..lit_str.len() - 1].to_string() } else { String::new() }
	} else {
		String::new()
	}
}

fn truncate_content(s: &str, max_len: usize) -> String {
	if s.len() <= max_len { format!("\"{s}\"") } else { format!("\"{}...\"", &s[..max_len]) }
}

fn create_clear_snapshot_fix(snapshot_span: Span, content: &str) -> Option<Fix> {
	let lines: Vec<&str> = content.lines().collect();
	let line_idx = snapshot_span.start().line - 1;
	if line_idx >= lines.len() {
		return None;
	}

	let line = lines[line_idx];
	let col = snapshot_span.start().column;

	// Find the @ before the snapshot string in the line
	// We need to find where @"..." or @r"..." starts and replace with @""
	let at_pos = line[..col].rfind('@').unwrap_or(col.saturating_sub(1));

	// Find the end of the snapshot string (could span multiple lines)
	let end_line_idx = snapshot_span.end().line - 1;
	let end_col = snapshot_span.end().column;

	let mut line_start_byte = 0;
	for (i, l) in lines.iter().enumerate() {
		if i == line_idx {
			break;
		}
		line_start_byte += l.len() + 1;
	}

	let start_byte = line_start_byte + at_pos;

	let mut end_byte = 0;
	for (i, l) in lines.iter().enumerate() {
		if i == end_line_idx {
			end_byte += end_col;
			break;
		}
		end_byte += l.len() + 1;
	}

	Some(Fix {
		start_byte,
		end_byte,
		replacement: "@\"\"".to_string(),
	})
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
