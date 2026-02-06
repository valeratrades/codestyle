use std::path::Path;

use syn::{Item, spanned::Spanned};

use super::{Fix, Violation, skip::has_skip_marker_for_rule};

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let path_str = path.display().to_string();
	let mut violations = Vec::new();

	for item in &file.items {
		let Item::Impl(impl_block) = item else {
			continue;
		};

		// Skip if marked with codestyle::skip comment
		if has_skip_marker_for_rule(content, impl_block.span(), RULE) {
			continue;
		}

		// Skip trait impls - only check direct `impl Type` blocks
		if impl_block.trait_.is_some() {
			continue;
		}

		let span = impl_block.span();
		let start_line = span.start().line;
		let start_col = span.start().column;
		let end_line = span.end().line;
		let end_col = span.end().column;

		let start_byte = match span_position_to_byte(content, start_line, start_col) {
			Some(b) => b,
			None => continue,
		};
		let end_byte = match span_position_to_byte(content, end_line, end_col) {
			Some(b) => b,
			None => continue,
		};

		let impl_text = &content[start_byte..end_byte];

		// Check for opening fold marker first
		let has_open_marker = impl_text.contains(OPEN_MARKER);

		// Find the opening brace position - if there's a marker, find the brace after it
		let brace_open_offset = if has_open_marker {
			// Find the marker, then find the brace after it
			let marker_end = impl_text.find(OPEN_MARKER).unwrap() + OPEN_MARKER.len();
			impl_text[marker_end..].find('{').map(|pos| marker_end + pos)
		} else {
			impl_text.find('{')
		};

		let Some(brace_open_offset) = brace_open_offset else {
			continue;
		};

		// Check if the line following the impl block has the close marker
		let has_close_marker = check_close_marker_after_impl(content, end_byte);

		if has_open_marker && has_close_marker {
			// All good
			continue;
		}

		// Generate the fix
		let fix = generate_fix(content, start_byte, end_byte, brace_open_offset, has_open_marker, has_close_marker);

		let message = if !has_open_marker && !has_close_marker {
			"impl block missing vim fold markers".to_string()
		} else if !has_open_marker {
			"impl block missing opening vim fold marker /*{{{1*/".to_string()
		} else {
			"impl block missing closing vim fold marker //,}}}1".to_string()
		};

		violations.push(Violation {
			rule: RULE,
			file: path_str.clone(),
			line: start_line,
			column: start_col,
			message,
			fix: Some(fix),
		});
	}

	violations
}
const RULE: &str = "impl-folds";

const OPEN_MARKER: &str = "/*{{{1*/";
const CLOSE_MARKER: &str = "//,}}}1";

fn check_close_marker_after_impl(content: &str, impl_end_byte: usize) -> bool {
	let after = &content[impl_end_byte..];

	// Skip whitespace and look for the close marker on the next line
	for line in after.lines() {
		let trimmed = line.trim();
		if trimmed.is_empty() {
			continue;
		}
		return trimmed == CLOSE_MARKER || trimmed.starts_with(CLOSE_MARKER);
	}

	false
}

fn generate_fix(content: &str, start_byte: usize, end_byte: usize, brace_open_offset: usize, has_open: bool, has_close: bool) -> Fix {
	let impl_text = &content[start_byte..end_byte];

	let mut new_impl = String::new();

	if !has_open {
		// Insert opening marker before the brace
		let before_brace = &impl_text[..brace_open_offset];
		let after_brace = &impl_text[brace_open_offset..];

		// Check if the brace is on a new line (where clause case)
		// by looking at the whitespace before the brace
		let trailing_ws = before_brace.trim_end_matches(|c: char| c != '\n' && c.is_whitespace());
		let brace_on_new_line = trailing_ws.ends_with('\n');

		let trimmed_before = before_brace.trim_end();
		new_impl.push_str(trimmed_before);

		if brace_on_new_line {
			// Put marker on its own line before the brace
			new_impl.push('\n');
			new_impl.push_str(OPEN_MARKER);
			new_impl.push(' ');
		} else {
			// Put marker on same line
			new_impl.push(' ');
			new_impl.push_str(OPEN_MARKER);
			new_impl.push(' ');
		}
		new_impl.push_str(after_brace);
	} else {
		new_impl.push_str(impl_text);
	}

	// Handle closing marker
	if !has_close {
		// Add the close marker after the impl block
		let full_replacement = format!("{new_impl}\n{CLOSE_MARKER}\n");

		return Fix {
			start_byte,
			end_byte,
			replacement: full_replacement,
		};
	}

	Fix {
		start_byte,
		end_byte,
		replacement: new_impl,
	}
}

fn span_position_to_byte(content: &str, line: usize, column: usize) -> Option<usize> {
	let mut current_line = 1;
	let mut line_start = 0;

	for (i, ch) in content.char_indices() {
		if current_line == line {
			return Some(line_start + column);
		}
		if ch == '\n' {
			current_line += 1;
			line_start = i + 1;
		}
	}

	if current_line == line {
		return Some(line_start + column);
	}

	None
}
