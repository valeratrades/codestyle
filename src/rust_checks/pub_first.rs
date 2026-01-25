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
	/// Byte offset where the item starts (including any preceding doc comments/attributes on the same "block")
	text_start: usize,
	/// Byte offset where the item ends (end of line containing the item's closing brace/semicolon)
	text_end: usize,
}

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let path_str = path.display().to_string();

	// Collect all top-level items with their visibility and positions
	// We need to track the text boundaries carefully to include doc comments
	let items: Vec<ItemInfo> = file
		.items
		.iter()
		.filter_map(|item| {
			let (is_pub, is_main_fn) = get_item_visibility_and_main(item)?;

			// Get the span start - this includes attributes but we need to find doc comments ourselves
			let span_start_line = item.span().start().line;
			let span_start_col = item.span().start().column;
			let span_end_line = item.span().end().line;
			let span_end_col = item.span().end().column;

			let span_start_byte = span_position_to_byte(content, span_start_line, span_start_col)?;
			let span_end_byte = span_position_to_byte(content, span_end_line, span_end_col)?;

			// Find the actual start including doc comments by looking backwards
			let text_start = find_item_text_start(content, span_start_byte);
			let text_end = find_line_end(content, span_end_byte);

			Some(ItemInfo {
				is_pub,
				is_main_fn,
				start_line: span_start_line,
				text_start,
				text_end,
			})
		})
		.collect();

	if items.is_empty() {
		return vec![];
	}

	// Find first violation: a private item followed by a public item
	// or within same visibility category, a non-main followed by main
	let mut first_private_idx: Option<usize> = None;

	for (i, item) in items.iter().enumerate() {
		// Track first private item
		if !item.is_pub && first_private_idx.is_none() {
			first_private_idx = Some(i);
		}

		// Check if we found a public item after a private one
		if item.is_pub {
			if let Some(priv_idx) = first_private_idx {
				// This public item should be moved before the first private item
				let fix = create_move_fix(content, &items, i, priv_idx);
				return vec![Violation {
					rule: RULE,
					file: path_str,
					line: item.start_line,
					column: 0,
					message: "public item should come before private items".to_string(),
					fix,
				}];
			}
		}
	}

	// Check for main function ordering within visibility categories
	// Find first pub non-main followed by pub main
	let mut first_pub_non_main_idx: Option<usize> = None;
	for (i, item) in items.iter().enumerate() {
		if item.is_pub {
			if !item.is_main_fn && first_pub_non_main_idx.is_none() {
				first_pub_non_main_idx = Some(i);
			}
			if item.is_main_fn {
				if let Some(target_idx) = first_pub_non_main_idx {
					let fix = create_move_fix(content, &items, i, target_idx);
					return vec![Violation {
						rule: RULE,
						file: path_str,
						line: item.start_line,
						column: 0,
						message: "`main` function should be at the top of its visibility category".to_string(),
						fix,
					}];
				}
			}
		}
	}

	// Find first private non-main followed by private main
	let mut first_priv_non_main_idx: Option<usize> = None;
	for (i, item) in items.iter().enumerate() {
		if !item.is_pub {
			if !item.is_main_fn && first_priv_non_main_idx.is_none() {
				first_priv_non_main_idx = Some(i);
			}
			if item.is_main_fn {
				if let Some(target_idx) = first_priv_non_main_idx {
					let fix = create_move_fix(content, &items, i, target_idx);
					return vec![Violation {
						rule: RULE,
						file: path_str,
						line: item.start_line,
						column: 0,
						message: "`main` function should be at the top of its visibility category".to_string(),
						fix,
					}];
				}
			}
		}
	}

	vec![]
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

/// Creates a fix that moves item at `from_idx` to before item at `to_idx`.
fn create_move_fix(content: &str, items: &[ItemInfo], from_idx: usize, to_idx: usize) -> Option<Fix> {
	if from_idx <= to_idx {
		return None; // Item is already in correct position or before target
	}

	let from_item = &items[from_idx];
	let to_item = &items[to_idx];

	// Extract the text of the item to move (from text_start to text_end)
	let item_text = &content[from_item.text_start..from_item.text_end];

	// We need to:
	// 1. Remove the item from its current position (including trailing newline if present)
	// 2. Insert it at the target position

	// Determine if there's a trailing newline after the item to remove
	let remove_end = if from_item.text_end < content.len() && content.as_bytes()[from_item.text_end] == b'\n' {
		from_item.text_end + 1
	} else {
		from_item.text_end
	};

	// Build the replacement:
	// - From the start of the target item's text to the end of the removed item's text
	// - We insert the moved item before the target, then keep everything in between, minus the original item

	let insert_pos = to_item.text_start;

	// Build the new content for the range [insert_pos, remove_end)
	let mut replacement = String::new();

	// Add the moved item
	replacement.push_str(item_text);
	replacement.push('\n');

	// Add everything that was between insert_pos and from_item.text_start
	replacement.push_str(&content[insert_pos..from_item.text_start]);

	// Skip the item we're moving (from from_item.text_start to remove_end)
	// Add everything from from_item.text_end to original end - but we're replacing the whole range

	// Actually let me reconsider the fix boundaries...
	// We're replacing [insert_pos, remove_end) with:
	//   moved_item + "\n" + content[insert_pos..from_item.text_start]
	// This effectively removes the trailing content[from_item.text_start..remove_end] and prepends it at insert_pos

	// Wait, that's not quite right either. Let me be more precise:
	// Original: ... [insert_pos] <to_item> ... <stuff> ... <from_item> [remove_end] ...
	// Desired:  ... [insert_pos] <from_item>\n<to_item> ... <stuff> ... [remove_end] ...
	//
	// The replacement range is [insert_pos, remove_end)
	// The replacement content is: <from_item>\n + content[insert_pos..from_item.text_start]

	// Hmm, but that loses anything between from_item.text_end and remove_end (which should just be the newline we're handling)

	// Let me recalculate:
	// Original range [insert_pos, remove_end) contains:
	//   content[insert_pos..from_item.text_start] + content[from_item.text_start..from_item.text_end] + content[from_item.text_end..remove_end]
	// = middle_stuff + from_item_text + trailing_newline
	//
	// We want to replace with:
	//   from_item_text + "\n" + middle_stuff
	// = from_item_text + "\n" + content[insert_pos..from_item.text_start]

	// That looks right. The trailing newline after from_item is consumed and we add our own newline after the moved item.

	Some(Fix {
		start_byte: insert_pos,
		end_byte: remove_end,
		replacement,
	})
}

/// Find the start of an item's text, including preceding doc comments and attributes.
/// We look backwards from the span start to find consecutive comment/attribute lines.
fn find_item_text_start(content: &str, span_start: usize) -> usize {
	let line_start = find_line_start(content, span_start);

	// Look backwards line by line to find doc comments or blank lines that should be included
	let mut current_start = line_start;

	loop {
		if current_start == 0 {
			break;
		}

		// Find the previous line
		let prev_line_end = current_start - 1; // Position of the \n
		let prev_line_start = find_line_start(content, prev_line_end.saturating_sub(1));
		let prev_line = content[prev_line_start..prev_line_end].trim_start();

		// Check if previous line is a doc comment (///) or attribute (#[)
		// Note: //! is a module doc comment and should NOT be included
		if prev_line.starts_with("///") || prev_line.starts_with("#[") {
			current_start = prev_line_start;
		} else {
			// Stop if we hit a non-doc-comment, non-attribute line
			break;
		}
	}

	current_start
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
