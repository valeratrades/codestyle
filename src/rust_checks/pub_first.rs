//! Rule: public items should come before private items within each file.
//!
//! Within each visibility category, `main` function should be at the top.

use std::path::Path;

use syn::{Item, Visibility, spanned::Spanned};

use super::{Fix, Violation};

const RULE: &str = "pub-first";

/// Represents an item with its visibility and position info
struct ItemInfo {
	is_pub: bool,
	is_main_fn: bool,
	start_line: usize,
	start_byte: usize,
	end_byte: usize,
}

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let path_str = path.display().to_string();

	// Collect all top-level items with their visibility and positions
	let items: Vec<ItemInfo> = file
		.items
		.iter()
		.filter_map(|item| {
			let (is_pub, is_main_fn) = get_item_visibility_and_main(item)?;
			let start_line = item.span().start().line;
			let start_byte = span_position_to_byte(content, start_line, item.span().start().column)?;
			let end_byte = span_position_to_byte(content, item.span().end().line, item.span().end().column)?;

			Some(ItemInfo {
				is_pub,
				is_main_fn,
				start_line,
				start_byte,
				end_byte,
			})
		})
		.collect();

	if items.is_empty() {
		return vec![];
	}

	// Find first violation: any private item followed by a public item
	// or within same visibility category, a non-main followed by main
	let mut violations = Vec::new();

	for i in 0..items.len() {
		for j in (i + 1)..items.len() {
			let earlier = &items[i];
			let later = &items[j];

			// Private item before public item is a violation
			if !earlier.is_pub && later.is_pub {
				let fix = create_reorder_fix(content, &items);
				violations.push(Violation {
					rule: RULE,
					file: path_str.clone(),
					line: later.start_line,
					column: 0,
					message: "public item should come before private items".to_string(),
					fix,
				});
				return violations; // Return first violation with fix that reorders everything
			}

			// Within same visibility category: main should be first
			if earlier.is_pub == later.is_pub && !earlier.is_main_fn && later.is_main_fn {
				let fix = create_reorder_fix(content, &items);
				violations.push(Violation {
					rule: RULE,
					file: path_str.clone(),
					line: later.start_line,
					column: 0,
					message: "`main` function should be at the top of its visibility category".to_string(),
					fix,
				});
				return violations;
			}
		}
	}

	violations
}

/// Returns (is_pub, is_main_fn) for an item, or None if it should be skipped
fn get_item_visibility_and_main(item: &Item) -> Option<(bool, bool)> {
	let (vis, is_main_fn) = match item {
		Item::Fn(f) => (Some(&f.vis), f.sig.ident == "main"),
		Item::Struct(s) => (Some(&s.vis), false),
		Item::Enum(e) => (Some(&e.vis), false),
		Item::Type(t) => (Some(&t.vis), false),
		Item::Const(c) => (Some(&c.vis), false),
		Item::Static(s) => (Some(&s.vis), false),
		Item::Trait(t) => (Some(&t.vis), false),
		Item::Mod(m) => (Some(&m.vis), false),
		Item::Union(u) => (Some(&u.vis), false),
		Item::ExternCrate(_) => return None, // Skip extern crate declarations
		Item::Use(_) => return None,         // Skip use statements - they have their own ordering conventions
		Item::Impl(_) => return None,        // Skip impl blocks - they're handled by impl_follows_type
		Item::Macro(_) => return None,       // Skip macro invocations
		Item::ForeignMod(_) => return None,  // Skip extern blocks
		_ => return None,
	};

	let is_pub = matches!(vis, Some(Visibility::Public(_)));
	Some((is_pub, is_main_fn))
}

/// Creates a fix that reorders all items: pub items first, then private,
/// with main at the top of each category.
fn create_reorder_fix(content: &str, items: &[ItemInfo]) -> Option<Fix> {
	if items.is_empty() {
		return None;
	}

	// Find the range that spans all items
	let first_item_line_start = find_line_start(content, items[0].start_byte);
	let last_item_end = items.last()?.end_byte;
	let last_item_line_end = find_line_end(content, last_item_end);

	// Collect item texts with their info for sorting
	let mut item_texts: Vec<(&ItemInfo, String)> = items
		.iter()
		.map(|info| {
			let line_start = find_line_start(content, info.start_byte);
			let line_end = find_line_end(content, info.end_byte);
			let text = content[line_start..line_end].to_string();
			(info, text)
		})
		.collect();

	// Sort: pub items first, then private; main first within each category
	item_texts.sort_by(|(a, _), (b, _)| {
		// First sort by visibility (pub first)
		match (a.is_pub, b.is_pub) {
			(true, false) => std::cmp::Ordering::Less,
			(false, true) => std::cmp::Ordering::Greater,
			_ => {
				// Same visibility, sort by main (main first)
				match (a.is_main_fn, b.is_main_fn) {
					(true, false) => std::cmp::Ordering::Less,
					(false, true) => std::cmp::Ordering::Greater,
					_ => std::cmp::Ordering::Equal, // Preserve relative order
				}
			}
		}
	});

	// Build the replacement text
	let replacement = item_texts.iter().map(|(_, text)| text.as_str()).collect::<Vec<_>>().join("\n");

	Some(Fix {
		start_byte: first_item_line_start,
		end_byte: last_item_line_end,
		replacement,
	})
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
