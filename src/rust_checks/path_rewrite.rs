//! Shared utilities for rules that detect and rewrite fully-qualified paths.

use syn::{ExprPath, TypePath};

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

/// If `node` exactly matches the given `segments`, returns a [`Fix`] replacing the full path
/// with `short_name`.
///
/// Example: `segments = ["std", "sync", "Arc"]`, `short_name = "Arc"` matches `std::sync::Arc<T>`.
pub fn rewrite_full_type_path(content: &str, node: &TypePath, segments: &[&str], short_name: &str) -> Option<Fix> {
	let path = &node.path;
	if path.segments.len() != segments.len() {
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

/// Returns `true` if `name` appears in `content` as a bare identifier (word-boundary matched)
/// in a non-`use` line.
///
/// "Bare" means not prefixed/suffixed by alphanumeric or `_` chars. This avoids false positives
/// where e.g. `"Path"` would match inside `"PathBuf"`.
pub fn bare_name_in_non_use_lines(content: &str, name: &str) -> bool {
	let name_bytes = name.as_bytes();
	for line in content.lines() {
		if line.starts_with("use ") {
			continue;
		}
		let bytes = line.as_bytes();
		for i in 0..=bytes.len().saturating_sub(name_bytes.len()) {
			if !bytes[i..].starts_with(name_bytes) {
				continue;
			}
			let before_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_';
			let after = i + name_bytes.len();
			let after_ok = after >= bytes.len() || !bytes[after].is_ascii_alphanumeric() && bytes[after] != b'_';
			if before_ok && after_ok {
				return true;
			}
		}
	}
	false
}

/// If `node`'s leading segments match `segments`, returns a [`Fix`] replacing those segments
/// with `short_name`.
///
/// Example: `segments = ["std", "sync", "Arc"]` matches `std::sync::Arc::clone(...)`.
pub fn rewrite_full_expr_path(content: &str, node: &ExprPath, segments: &[&str], short_name: &str) -> Option<Fix> {
	let path = &node.path;
	if path.segments.len() < segments.len() {
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
