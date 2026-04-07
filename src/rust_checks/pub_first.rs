//! Rule: items are ordered as follows:
//! 1. All const items (regardless of visibility)
//! 2. All type items (regardless of visibility)
//! 3. All pub items (Parser > Subcommand > Args > main > trait > other)
//! 4. All private items (Parser > Subcommand > Args > main > trait > other)

use std::{collections::HashSet, path::Path};

use syn::{Item, Visibility, spanned::Spanned};

use super::{Fix, Violation, skip::has_skip_marker_for_rule};

const RULE: &str = "pub-first";
pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let path_str = path.display().to_string();

	// Collect names of struct/enum/union types defined in this file, so we can detect
	// type aliases that reference them (those should stay near their definition, not move to top).
	let local_type_names: HashSet<String> = file
		.items
		.iter()
		.filter_map(|item| match item {
			Item::Struct(s) => Some(s.ident.to_string()),
			Item::Enum(e) => Some(e.ident.to_string()),
			Item::Union(u) => Some(u.ident.to_string()),
			_ => None,
		})
		.collect();

	// Collect byte ranges of mod/use/extern-crate items so the fix can avoid displacing
	// them when reordering. These conventionally live at the top of the file.
	let anchor_ranges: Vec<(usize, usize)> = file
		.items
		.iter()
		.filter(|item| match item {
			// Only `mod foo;` declarations are anchors, not inline `mod foo { ... }` blocks
			Item::Mod(m) => m.content.is_none(),
			Item::Use(_) | Item::ExternCrate(_) => true,
			_ => false,
		})
		.filter_map(|item| {
			let start_byte = span_position_to_byte(content, item.span().start().line, item.span().start().column)?;
			let end_byte = span_position_to_byte(content, item.span().end().line, item.span().end().column)?;
			let text_start = find_item_text_start(content, start_byte);
			let text_end = find_line_end(content, end_byte);
			Some((text_start, text_end))
		})
		.collect();

	// Collect all top-level items with their visibility and positions
	// We need to track the text boundaries carefully to include doc comments
	let items: Vec<ItemInfo> = file.items.iter().filter_map(|item| get_item_info(item, content, &local_type_names)).collect();

	if items.is_empty() {
		return vec![];
	}

	// 1. Check const ordering - all const items should come first (regardless of visibility)
	let mut first_non_const_idx: Option<usize> = None;
	for (i, item) in items.iter().enumerate() {
		if !item.is_const && first_non_const_idx.is_none() {
			first_non_const_idx = Some(i);
		}
		if item.is_const
			&& let Some(target_idx) = first_non_const_idx
		{
			let fix = create_move_fix(content, &items, &anchor_ranges, i, target_idx);
			return vec![Violation {
				rule: RULE,
				file: path_str,
				line: item.start_line,
				column: 0,
				message: "`const` should come before all other items".to_string(),
				fix,
			}];
		}
	}

	// 2. Check type ordering - all type items should come after const but before everything else
	let mut first_non_const_non_type_idx: Option<usize> = None;
	for (i, item) in items.iter().enumerate() {
		if !item.is_const && !item.is_type && first_non_const_non_type_idx.is_none() {
			first_non_const_non_type_idx = Some(i);
		}
		if item.is_type
			&& let Some(target_idx) = first_non_const_non_type_idx
		{
			let fix = create_move_fix(content, &items, &anchor_ranges, i, target_idx);
			return vec![Violation {
				rule: RULE,
				file: path_str,
				line: item.start_line,
				column: 0,
				message: "`type` should come before all other items (after const)".to_string(),
				fix,
			}];
		}
	}

	// 3. Check pub/private ordering - pub items should come before private (excluding const/type)
	let mut first_private_idx: Option<usize> = None;
	for (i, item) in items.iter().enumerate() {
		// Skip const and type - they're already handled
		if item.is_const || item.is_type {
			continue;
		}

		if !item.is_pub && first_private_idx.is_none() {
			first_private_idx = Some(i);
		}
		if item.is_pub
			&& let Some(target_idx) = first_private_idx
		{
			let fix = create_move_fix(content, &items, &anchor_ranges, i, target_idx);
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

	// 4+. Within each visibility category (pub/private), check sub-ordering:
	// Parser > Subcommand > Args > main > trait > other
	fn is_clap(item: &ItemInfo) -> bool {
		item.is_parser || item.is_subcommand || item.is_args
	}

	let rules = [true, false].into_iter().flat_map(|is_pub| {
		[
			KindRule {
				is_pub,
				is_target: |item: &ItemInfo| item.is_parser,
				is_higher_priority: |_: &ItemInfo| false,
				message: "Parser struct should be at the top of its visibility category",
			},
			KindRule {
				is_pub,
				is_target: |item: &ItemInfo| item.is_subcommand,
				is_higher_priority: |item: &ItemInfo| item.is_parser,
				message: "Subcommand enum should come after Parser",
			},
			KindRule {
				is_pub,
				is_target: |item: &ItemInfo| item.is_args,
				is_higher_priority: |item: &ItemInfo| item.is_parser || item.is_subcommand,
				message: "Args struct should come after Subcommand",
			},
			KindRule {
				is_pub,
				is_target: |item: &ItemInfo| item.is_main_fn,
				is_higher_priority: |item: &ItemInfo| is_clap(item),
				message: "`main` function should be at the top of its visibility category (after clap types)",
			},
			KindRule {
				is_pub,
				is_target: |item: &ItemInfo| item.is_trait,
				is_higher_priority: |item: &ItemInfo| is_clap(item) || item.is_main_fn,
				message: "`trait` should be at the top of its visibility category (after main)",
			},
		]
	});

	for rule in rules {
		if let Some(v) = check_kind_ordering(&items, &anchor_ranges, content, &path_str, rule) {
			return vec![v];
		}
	}

	// 5. Check that non-const/non-type code items don't appear before mod/use/extern-crate anchors.
	// This catches the case where mod/use declarations are scattered below code items (e.g., fn main
	// sitting above `mod foo;` and `use bar;`). Const and type items are allowed above anchors.
	if let Some(last_anchor_end) = anchor_ranges.iter().map(|(_, end)| *end).max() {
		for item in &items {
			if item.is_const || item.is_type {
				continue;
			}
			if item.text_start >= last_anchor_end {
				break; // All remaining items are after anchors
			}
			// This code item is before a later anchor — move it after the last anchor
			let item_text = &content[item.text_start..item.text_end];
			let remove_end = if item.text_end < content.len() && content.as_bytes()[item.text_end] == b'\n' {
				item.text_end + 1
			} else {
				item.text_end
			};

			// Find insert position: right after the last anchor's line end
			let last_anchor_line_end = find_line_end(content, last_anchor_end);
			let insert_pos = if last_anchor_line_end < content.len() && content.as_bytes()[last_anchor_line_end] == b'\n' {
				last_anchor_line_end + 1
			} else {
				last_anchor_line_end
			};

			// Build fix: replace the span from item start to insert_pos
			// with: (content between remove_end and insert_pos) + blank line + item_text + newline
			let between = &content[remove_end..insert_pos];
			let mut replacement = String::default();
			replacement.push_str(between);
			if !replacement.ends_with("\n\n") {
				replacement.push('\n');
			}
			replacement.push_str(item_text);
			replacement.push('\n');

			return vec![Violation {
				rule: RULE,
				file: path_str,
				line: item.start_line,
				column: 0,
				message: "item should come after mod/use declarations".to_string(),
				fix: Some(Fix {
					start_byte: item.text_start,
					end_byte: insert_pos,
					replacement,
				}),
			}];
		}
	}

	vec![]
}

struct KindRule {
	is_pub: bool,
	is_target: fn(&ItemInfo) -> bool,
	is_higher_priority: fn(&ItemInfo) -> bool,
	message: &'static str,
}

/// Check that items of a specific kind (main/trait/struct) appear before lower-priority items
/// within a visibility category (pub/private), excluding const and type items.
fn check_kind_ordering(items: &[ItemInfo], anchor_ranges: &[(usize, usize)], content: &str, path_str: &str, rule: KindRule) -> Option<Violation> {
	let mut first_lower_idx: Option<usize> = None;
	for (i, item) in items.iter().enumerate() {
		if item.is_pub == rule.is_pub && !item.is_const && !item.is_type {
			if !(rule.is_target)(item) && !(rule.is_higher_priority)(item) && first_lower_idx.is_none() {
				first_lower_idx = Some(i);
			}
			if (rule.is_target)(item)
				&& let Some(target_idx) = first_lower_idx
			{
				let fix = create_move_fix(content, items, anchor_ranges, i, target_idx);
				return Some(Violation {
					rule: RULE,
					file: path_str.to_string(),
					line: item.start_line,
					column: 0,
					message: rule.message.to_string(),
					fix,
				});
			}
		}
	}
	None
}

/// Represents an item with its visibility and position info
struct ItemInfo {
	is_pub: bool,
	is_main_fn: bool,
	is_const: bool,
	is_type: bool,
	is_trait: bool,
	is_parser: bool,
	is_subcommand: bool,
	is_args: bool,
	start_line: usize,
	/// Byte offset where the item starts (including any preceding doc comments/attributes on the same "block")
	text_start: usize,
	/// Byte offset where the item ends (end of line containing the item's closing brace/semicolon)
	text_end: usize,
}

/// Returns a fully populated `ItemInfo`, or `None` if the item should be skipped.
fn get_item_info(item: &Item, content: &str, local_type_names: &HashSet<String>) -> Option<ItemInfo> {
	let (vis, is_main_fn, is_const, is_type, is_trait, is_parser, is_subcommand, is_args) = match item {
		Item::Fn(f) => (Some(&f.vis), f.sig.ident == "main", false, false, false, false, false, false),
		Item::Struct(s) => {
			let attrs = &s.attrs;
			(Some(&s.vis), false, false, false, false, has_clap_derive(attrs, "Parser"), false, has_clap_derive(attrs, "Args"))
		}
		Item::Enum(e) => {
			let attrs = &e.attrs;
			(
				Some(&e.vis),
				false,
				false,
				false,
				false,
				has_clap_derive(attrs, "Parser"),
				has_clap_derive(attrs, "Subcommand"),
				false,
			)
		}
		Item::Type(t) => {
			// If the type alias references a struct/enum/union defined in this file,
			// don't treat it as a "type" for ordering — it should stay near its definition.
			let references_local = type_alias_references_local(&t.ty, local_type_names);
			(Some(&t.vis), false, false, !references_local, false, false, false, false)
		}
		Item::Const(c) => (Some(&c.vis), false, true, false, false, false, false, false),
		Item::Static(s) => (Some(&s.vis), false, false, false, false, false, false, false),
		Item::Trait(t) => (Some(&t.vis), false, false, false, true, false, false, false),
		Item::Mod(_) => return None, //HACK: skip `mod` - sorting these conflicts with `rustfmt`'s module reordering
		Item::Union(u) => (Some(&u.vis), false, false, false, false, false, false, false),
		Item::ExternCrate(_) => return None, // Skip extern crate declarations
		Item::Use(_) => return None,         // Skip use statements - they have their own ordering conventions
		Item::Impl(_) => return None,        // Skip impl blocks - they're handled by impl_follows_type
		Item::Macro(_) => return None,       // Skip macro invocations
		Item::ForeignMod(_) => return None,  // Skip extern blocks
		_ => return None,
	};

	if has_skip_marker_for_rule(content, item.span(), RULE) {
		return None;
	}

	let span_start_line = item.span().start().line;
	let span_start_col = item.span().start().column;
	let span_end_line = item.span().end().line;
	let span_end_col = item.span().end().column;
	let span_start_byte = span_position_to_byte(content, span_start_line, span_start_col)?;
	let span_end_byte = span_position_to_byte(content, span_end_line, span_end_col)?;

	Some(ItemInfo {
		is_pub: matches!(vis, Some(Visibility::Public(_))),
		is_main_fn,
		is_const,
		is_type,
		is_trait,
		is_parser,
		is_subcommand,
		is_args,
		start_line: span_start_line,
		text_start: find_item_text_start(content, span_start_byte),
		text_end: find_line_end(content, span_end_byte),
	})
}

/// Check if a type alias's RHS references a type defined in this file.
/// Extracts the root type name from paths like `Book<T>`, `Foo`, etc.
fn type_alias_references_local(ty: &syn::Type, local_type_names: &HashSet<String>) -> bool {
	match ty {
		syn::Type::Path(type_path) =>
			if let Some(segment) = type_path.path.segments.first() {
				local_type_names.contains(&segment.ident.to_string())
			} else {
				false
			},
		_ => false,
	}
}

fn has_clap_derive(attrs: &[syn::Attribute], trait_name: &str) -> bool {
	attrs.iter().any(|attr| {
		if !attr.path().is_ident("derive") {
			return false;
		}
		let Ok(nested) = attr.parse_args_with(syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated) else {
			return false;
		};
		nested.iter().any(|path| {
			let segments: Vec<_> = path.segments.iter().map(|s| s.ident.to_string()).collect();
			segments == [trait_name] || segments == ["clap", trait_name]
		})
	})
}

/// Creates a fix that moves item at `from_idx` to before item at `to_idx`.
///
/// Anchor items (mod/use/extern crate) in the gap between from and to are kept before the
/// reordered code items — the moved item is placed after all anchors.
fn create_move_fix(content: &str, items: &[ItemInfo], anchor_ranges: &[(usize, usize)], from_idx: usize, to_idx: usize) -> Option<Fix> {
	if from_idx <= to_idx {
		return None;
	}

	let from_item = &items[from_idx];
	let to_item = &items[to_idx];

	let item_text = &content[from_item.text_start..from_item.text_end];

	let remove_end = if from_item.text_end < content.len() && content.as_bytes()[from_item.text_end] == b'\n' {
		from_item.text_end + 1
	} else {
		from_item.text_end
	};

	let insert_pos = to_item.text_start;

	// Collect anchor items in the gap between to_item and from_item.
	// These must stay above the reordered code items.
	let mut gap_anchors: Vec<(usize, usize)> = anchor_ranges
		.iter()
		.filter(|(start, _)| *start >= to_item.text_start && *start < from_item.text_start)
		.copied()
		.collect();
	gap_anchors.sort_by_key(|(start, _)| *start);

	if gap_anchors.is_empty() {
		// Simple case: no anchors in the gap, just move the item
		let mut replacement = String::default();
		replacement.push_str(item_text);
		replacement.push('\n');
		replacement.push_str(&content[insert_pos..from_item.text_start]);

		return Some(Fix {
			start_byte: insert_pos,
			end_byte: remove_end,
			replacement,
		});
	}

	// Complex case: anchor items exist in the gap. We reconstruct the range as:
	// 1. All anchor items (with their surrounding whitespace) - kept first
	// 2. The moved item (from_item)
	// 3. All non-anchor gap content (the other code items and their whitespace)
	//
	// Walk through the gap collecting anchor text and non-anchor text separately.
	let mut anchor_text = String::default();
	let mut code_text = String::default();
	let mut pos = insert_pos;

	for (anchor_start, anchor_end) in &gap_anchors {
		// Text before this anchor (could be code items, whitespace)
		if pos < *anchor_start {
			code_text.push_str(&content[pos..*anchor_start]);
		}
		// The anchor itself (including its line)
		let anchor_line_end = find_line_end(content, *anchor_end);
		let anchor_chunk_end = if anchor_line_end < content.len() && content.as_bytes()[anchor_line_end] == b'\n' {
			anchor_line_end + 1
		} else {
			anchor_line_end
		};
		anchor_text.push_str(&content[*anchor_start..anchor_chunk_end]);
		pos = anchor_chunk_end;
	}

	// Remaining gap content after the last anchor
	if pos < from_item.text_start {
		code_text.push_str(&content[pos..from_item.text_start]);
	}

	// Trim trailing blank lines from code_text to avoid accumulation across iterations
	let code_text = code_text.trim_end_matches('\n');
	let code_suffix = if code_text.is_empty() { "" } else { "\n" };

	let mut replacement = String::default();
	replacement.push_str(&anchor_text);
	// Blank line between anchor block and code items
	if !anchor_text.ends_with("\n\n") {
		replacement.push('\n');
	}
	replacement.push_str(item_text);
	replacement.push('\n');
	if !code_text.is_empty() {
		replacement.push_str(code_text);
		replacement.push_str(code_suffix);
	}

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
