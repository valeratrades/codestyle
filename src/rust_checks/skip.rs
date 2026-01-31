//! Utility for detecting `codestyle::skip` markers on items.
//!
//! When an item is marked with this marker, all codestyle checks should skip it entirely.
//!
//! Supported formats (as comments to avoid compiler errors):
//! - `//#[codestyle::skip]`
//! - `// #[codestyle::skip]`
//! - `//@codestyle::skip`
//! - `// @codestyle::skip`

use proc_macro2::Span;

/// Check if the line before the given span contains a codestyle::skip marker.
pub fn has_skip_marker(content: &str, span: Span) -> bool {
	let line = span.start().line;
	has_skip_marker_at_line(content, line)
}

/// Check if the given line or the line above contains a codestyle::skip marker.
fn has_skip_marker_at_line(content: &str, line: usize) -> bool {
	let lines: Vec<&str> = content.lines().collect();

	// Check current line (inline comment)
	if line > 0 && line <= lines.len() {
		let current_line = lines[line - 1];
		if is_skip_comment(current_line) {
			return true;
		}
	}

	// Check line above
	if line > 1 {
		let prev_line = lines[line - 2];
		if is_skip_comment(prev_line) {
			return true;
		}
	}

	false
}

/// Check if a line contains a codestyle::skip comment marker.
fn is_skip_comment(line: &str) -> bool {
	let trimmed = line.trim();

	// //#[codestyle::skip] or // #[codestyle::skip]
	if let Some(after_slashes) = trimmed.strip_prefix("//") {
		let after_slashes = after_slashes.trim_start();
		if after_slashes.starts_with("#[codestyle::skip]") || after_slashes.starts_with("@codestyle::skip") {
			return true;
		}
	}

	false
}
