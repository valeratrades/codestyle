use std::{collections::HashMap, path::Path};

use syn::{Item, ItemEnum, ItemImpl, ItemStruct, ItemUnion, spanned::Spanned};

use super::{Fix, Violation, skip::has_skip_marker_for_rule};

const RULE: &str = "impl-follows-type";
pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let path_str = path.display().to_string();
	let mut type_defs: HashMap<String, TypeDef> = HashMap::new();
	let mut violations = Vec::new();

	// First pass: collect all type definitions
	for item in &file.items {
		let (name, end_line) = match item {
			Item::Struct(ItemStruct { ident, .. }) => (ident.to_string(), item.span().end().line),
			Item::Enum(ItemEnum { ident, .. }) => (ident.to_string(), item.span().end().line),
			Item::Union(ItemUnion { ident, .. }) => (ident.to_string(), item.span().end().line),
			_ => continue,
		};

		let end_byte = span_position_to_byte(content, item.span().end().line, item.span().end().column).unwrap_or(0);
		type_defs.insert(name, TypeDef { end_line, end_byte });
	}

	// Second pass: collect impl blocks with their byte positions
	let impl_blocks: Vec<ImplBlock> = file
		.items
		.iter()
		.filter_map(|item| {
			let Item::Impl(impl_block) = item else {
				return None;
			};

			// Skip if marked with codestyle::skip comment
			if has_skip_marker_for_rule(content, impl_block.span(), RULE) {
				return None;
			}

			// Skip trait impls
			if impl_block.trait_.is_some() {
				return None;
			}

			let type_name = match &*impl_block.self_ty {
				syn::Type::Path(type_path) => type_path.path.segments.last().map(|s| s.ident.to_string()),
				_ => None,
			}?;

			// Skip impl blocks for types not defined in this file
			if !type_defs.contains_key(&type_name) {
				return None;
			}

			let start_line = impl_block.span().start().line;
			let start_byte = span_position_to_byte(content, start_line, impl_block.span().start().column)?;
			let end_byte = span_position_to_byte(content, impl_block.span().end().line, impl_block.span().end().column)?;

			Some(ImplBlock {
				item: impl_block,
				start_line,
				start_byte,
				end_byte,
			})
		})
		.collect();

	for impl_block in &impl_blocks {
		let type_name = match &*impl_block.item.self_ty {
			syn::Type::Path(type_path) => type_path.path.segments.last().map(|s| s.ident.to_string()),
			_ => None,
		};

		let Some(type_name) = type_name else {
			continue;
		};

		let Some(type_def) = type_defs.get(&type_name) else {
			continue;
		};

		let expected_line = type_def.end_line + 1;

		if impl_block.start_line > expected_line + 1 {
			let gap = impl_block.start_line - type_def.end_line - 1;

			// Calculate fix: extract impl block text and create two fixes
			// 1. Delete impl block from current location (including leading newlines)
			// 2. Insert impl block after type definition
			let fix = create_relocation_fix(content, type_def, impl_block);

			violations.push(Violation {
				rule: RULE,
				file: path_str.clone(),
				line: impl_block.start_line,
				column: impl_block.item.span().start().column,
				message: format!("`impl {type_name}` should follow type definition (line {}), but has {gap} blank line(s)", type_def.end_line),
				fix,
			});
		}

		// Update type_def to point to end of this impl block for chained impls
		type_defs.insert(
			type_name,
			TypeDef {
				end_line: impl_block.item.span().end().line,
				end_byte: impl_block.end_byte,
			},
		);
	}

	violations
}

struct TypeDef {
	end_line: usize,
	end_byte: usize,
}

struct ImplBlock<'a> {
	item: &'a ItemImpl,
	start_line: usize,
	start_byte: usize,
	end_byte: usize,
}

/// Creates a fix that relocates an impl block to immediately follow its type definition.
/// The fix replaces the region from type_def end to impl_block end with:
/// - The impl block text (moved to right after type def)
/// - Followed by any code that was between them
fn create_relocation_fix(content: &str, type_def: &TypeDef, impl_block: &ImplBlock) -> Option<Fix> {
	// Find the start of the impl block including any leading whitespace/newlines on that line
	let impl_line_start = find_line_start(content, impl_block.start_byte);

	// Extract the impl block text (from line start to end of impl block)
	let impl_text = &content[impl_line_start..impl_block.end_byte];

	// Find where to insert: right after type_def ends
	// We want to find the newline after type_def.end_byte
	let insert_pos = find_line_end(content, type_def.end_byte);

	// Check what's between type def and impl block
	let between_text = &content[insert_pos..impl_line_start];

	if between_text.trim().is_empty() {
		// Just blank lines - simple case, remove the extra blank lines
		let replacement = format!("\n{}", impl_text.trim_start_matches('\n'));
		Some(Fix {
			start_byte: insert_pos,
			end_byte: impl_block.end_byte,
			replacement,
		})
	} else {
		// There's other code between type def and impl block.
		// Reorder: impl block first, then the between code
		let between_trimmed = between_text.trim();
		let replacement = format!("\n{}\n\n{between_trimmed}", impl_text.trim_start_matches('\n'));
		Some(Fix {
			start_byte: insert_pos,
			end_byte: impl_block.end_byte,
			replacement,
		})
	}
}

/// Convert a line/column position to byte offset in content.
/// Lines are 1-indexed, columns are 0-indexed (byte offset within line).
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

/// Find the byte position of the start of the line containing `pos`
fn find_line_start(content: &str, pos: usize) -> usize {
	content[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

/// Find the byte position of the end of the line containing `pos` (the newline char position)
fn find_line_end(content: &str, pos: usize) -> usize {
	content[pos..].find('\n').map(|i| pos + i).unwrap_or(content.len())
}
