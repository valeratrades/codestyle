use std::{collections::HashMap, path::Path};

use syn::{Item, spanned::Spanned};

use super::{Fix, Violation};

struct ImplBlockInfo {
	start_line: usize,
	start_byte: usize,
	end_byte: usize,
	/// Byte position of the opening brace
	brace_open_byte: usize,
	/// The content inside the braces (the items)
	items_text: String,
}

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	const RULE: &str = "join-split-impls";

	let path_str = path.display().to_string();
	let mut violations = Vec::new();

	// Group inherent impl blocks by type name
	// Key: type name, Value: list of impl block info
	let mut inherent_impls: HashMap<String, Vec<ImplBlockInfo>> = HashMap::new();

	for item in &file.items {
		let Item::Impl(impl_block) = item else {
			continue;
		};

		// Skip trait impls - they can't be joined with inherent impls
		if impl_block.trait_.is_some() {
			continue;
		}

		let type_name = match &*impl_block.self_ty {
			syn::Type::Path(type_path) => type_path.path.segments.last().map(|s| s.ident.to_string()),
			_ => None,
		};

		let Some(type_name) = type_name else {
			continue;
		};

		let start_line = impl_block.span().start().line;
		let start_byte = span_position_to_byte(content, start_line, impl_block.span().start().column);
		let end_byte = span_position_to_byte(content, impl_block.span().end().line, impl_block.span().end().column);

		let (Some(start_byte), Some(end_byte)) = (start_byte, end_byte) else {
			continue;
		};

		// Find the opening and closing braces, skipping braces inside comments
		let impl_text = &content[start_byte..end_byte];
		let brace_open_offset = find_impl_brace(impl_text);
		let brace_close_offset = impl_text.rfind('}');

		let (Some(brace_open_offset), Some(brace_close_offset)) = (brace_open_offset, brace_close_offset) else {
			continue;
		};

		let brace_open_byte = start_byte + brace_open_offset;
		let brace_close_byte = start_byte + brace_close_offset;

		// Extract the items text (content between braces, excluding braces)
		let items_text = content[brace_open_byte + 1..brace_close_byte].to_string();

		inherent_impls.entry(type_name).or_default().push(ImplBlockInfo {
			start_line,
			start_byte,
			end_byte,
			brace_open_byte,
			items_text,
		});
	}

	// Find types with multiple inherent impl blocks
	for (type_name, impl_blocks) in &inherent_impls {
		if impl_blocks.len() < 2 {
			continue;
		}

		// Create a fix that joins all impl blocks into the first one
		// Strategy:
		// 1. Keep the first impl block's header and opening brace
		// 2. Append all items from subsequent impl blocks (preserving their indentation)
		// 3. Remove all subsequent impl blocks

		let first = &impl_blocks[0];
		let last = impl_blocks.last().unwrap();

		// Collect all items from all impl blocks, preserving original indentation
		let mut all_items_parts: Vec<String> = Vec::new();
		for block in impl_blocks {
			// Strip only leading/trailing blank lines, not indentation
			let stripped = strip_blank_lines(&block.items_text);
			if !stripped.is_empty() {
				all_items_parts.push(stripped);
			}
		}

		// Find what's between impl blocks that we need to preserve
		// Collect intervening code between impl blocks
		let mut between_sections = Vec::new();
		for i in 0..impl_blocks.len() - 1 {
			let current = &impl_blocks[i];
			let next = &impl_blocks[i + 1];

			// Get the text between end of current impl and start of next impl
			let between = &content[current.end_byte..next.start_byte];
			let trimmed = between.trim();
			if !trimmed.is_empty() {
				between_sections.push(trimmed.to_string());
			}
		}

		// Build the replacement:
		// - First impl header + opening brace + all items + closing brace
		// - Then any code that was between impl blocks
		let impl_header = &content[first.start_byte..first.brace_open_byte + 1];

		let mut replacement = String::from(impl_header);
		replacement.push('\n');

		// Add all items - they already have proper indentation
		for (i, items) in all_items_parts.iter().enumerate() {
			if i > 0 {
				replacement.push('\n');
			}
			replacement.push_str(items);
		}

		replacement.push('\n');
		replacement.push('}');

		if !between_sections.is_empty() {
			replacement.push_str("\n\n");
			replacement.push_str(&between_sections.join("\n\n"));
		}

		let fix = Some(Fix {
			start_byte: first.start_byte,
			end_byte: last.end_byte,
			replacement,
		});

		violations.push(Violation {
			rule: RULE,
			file: path_str.clone(),
			line: impl_blocks[1].start_line,
			column: 0,
			message: format!("split `impl {type_name}` blocks should be joined into one"),
			fix,
		});
	}

	violations
}

/// Convert a line/column position to byte offset in content.
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

/// Find the opening brace of an impl block, skipping braces inside comments.
/// This handles fold markers like `/*{{{1*/` which contain braces in comments.
fn find_impl_brace(text: &str) -> Option<usize> {
	let mut in_block_comment = false;
	let mut in_line_comment = false;
	let chars: Vec<char> = text.chars().collect();
	let mut i = 0;

	while i < chars.len() {
		let ch = chars[i];
		let next_ch = chars.get(i + 1).copied();

		// Handle comment boundaries
		if !in_block_comment && !in_line_comment {
			if ch == '/' && next_ch == Some('*') {
				in_block_comment = true;
				i += 2;
				continue;
			}
			if ch == '/' && next_ch == Some('/') {
				in_line_comment = true;
				i += 2;
				continue;
			}
			// Found a brace outside of comments
			if ch == '{' {
				return Some(i);
			}
		} else if in_block_comment {
			if ch == '*' && next_ch == Some('/') {
				in_block_comment = false;
				i += 2;
				continue;
			}
		} else if in_line_comment && ch == '\n' {
			in_line_comment = false;
		}

		i += 1;
	}

	None
}

/// Strip leading and trailing blank lines from text, preserving internal structure.
fn strip_blank_lines(text: &str) -> String {
	let lines: Vec<&str> = text.lines().collect();

	// Find first non-empty line
	let start = lines.iter().position(|line| !line.trim().is_empty()).unwrap_or(0);

	// Find last non-empty line
	let end = lines.iter().rposition(|line| !line.trim().is_empty()).map(|i| i + 1).unwrap_or(lines.len());

	lines[start..end].join("\n")
}
