//! Lint to inline `impl Default for T` when the body is a simple struct literal.
//!
//! Rust 1.82 stabilised RFC 3681 which allows `foo: T = expr` directly in the struct
//! definition. A manual `impl Default` that does nothing but build `Self { field: expr, … }`
//! is pure boilerplate and should be replaced with inline field defaults.
//!
//! Detection criteria:
//! - `impl Default for T` whose `fn default()` body is **exactly one expression**
//! - That expression is a struct literal `Self { … }` (or `TypeName { … }`)
//! - No spread (`..Default::default()` or any `..expr`)
//! - The struct must be defined in the same file with named fields and no generics
//!
//! Fix strategy:
//! Since syn cannot parse `field: Type = expr` syntax (RFC 3681 is not yet in syn),
//! a two-pass fix would leave an intermediate state that syn can't re-parse, breaking
//! the iterative formatter. Instead we emit a single compound Fix that spans from
//! struct_start to impl_end, replacing the entire region with just the rewritten struct.
//! This requires the struct and impl to be adjacent (at most blank lines between them).
//! Non-adjacent pairs are flagged without a fix.

use std::{collections::HashMap, path::Path};

use syn::{Block, Expr, ExprCall, ExprStruct, Fields, ImplItem, Item, ItemImpl, ItemStruct, spanned::Spanned, visit::Visit};

fn block_has_fn_calls(block: &Block) -> bool {
	struct Finder(bool);
	impl<'a> Visit<'a> for Finder {
		fn visit_expr_call(&mut self, _: &'a ExprCall) {
			self.0 = true;
		}
	}
	let mut f = Finder(false);
	f.visit_block(block);
	f.0
}

use super::{Fix, Violation, skip::has_skip_marker_for_rule};

const RULE: &str = "inline-default";

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let path_str = path.display().to_string();
	let mut violations = Vec::default();

	// First pass: collect all named structs without generics by name.
	let mut structs: HashMap<String, &ItemStruct> = HashMap::default();
	for item in &file.items {
		if let Item::Struct(s) = item {
			// Skip generics — RFC 3681 with generics is complex, skip for safety
			if !s.generics.params.is_empty() {
				continue;
			}
			// Only named fields
			if matches!(s.fields, Fields::Named(_)) {
				structs.insert(s.ident.to_string(), s);
			}
		}
	}

	// Second pass: find eligible `impl Default for T` blocks.
	for item in &file.items {
		let Item::Impl(impl_block) = item else {
			continue;
		};

		if has_skip_marker_for_rule(content, impl_block.span(), RULE) {
			continue;
		}

		let Some(struct_item) = eligible_impl(impl_block, &structs) else {
			continue;
		};

		let Some(init_expr) = simple_default_body(impl_block) else {
			continue;
		};

		// Build field initialiser map: field name -> expression text
		let field_inits = build_field_inits(content, &init_expr);
		if field_inits.is_empty() {
			continue;
		}

		// Make sure every struct field has an initialiser in the Default body
		let Fields::Named(ref named) = struct_item.fields else {
			continue;
		};
		let all_covered = named.named.iter().all(|f| f.ident.as_ref().is_some_and(|id| field_inits.contains_key(&id.to_string())));
		if !all_covered {
			continue;
		}

		// Attempt a compound fix: rewrite struct + delete impl in one shot.
		// This only works when there's nothing but blank lines between them.
		let fix = make_compound_fix(content, struct_item, impl_block, &field_inits);

		violations.push(Violation {
			rule: RULE,
			file: path_str.clone(),
			line: struct_item.span().start().line,
			column: struct_item.span().start().column,
			message: format!("`impl Default for {}` can be inlined as field defaults (RFC 3681)", struct_item.ident),
			fix,
		});
	}

	violations
}

/// Returns the struct item if `impl_block` is `impl Default for T` where T is a known struct.
fn eligible_impl<'a>(impl_block: &ItemImpl, structs: &HashMap<String, &'a ItemStruct>) -> Option<&'a ItemStruct> {
	// Must be a trait impl
	let (_, trait_path, _) = impl_block.trait_.as_ref()?;

	// Trait must be `Default`
	let last = trait_path.segments.last()?;
	if last.ident != "Default" {
		return None;
	}

	// Self type must be a simple path
	let syn::Type::Path(ref type_path) = *impl_block.self_ty else {
		return None;
	};
	let type_name = type_path.path.segments.last()?.ident.to_string();

	structs.get(&type_name).copied()
}

/// Returns the `ExprStruct` if `default()` body has no function calls and is a single struct literal.
fn simple_default_body(impl_block: &ItemImpl) -> Option<ExprStruct> {
	let default_fn = impl_block.items.iter().find_map(|item| {
		if let ImplItem::Fn(f) = item {
			if f.sig.ident == "default" { Some(f) } else { None }
		} else {
			None
		}
	})?;

	if block_has_fn_calls(&default_fn.block) {
		return None;
	}

	extract_struct_expr(&default_fn.block)
}

/// Extracts a bare `Self { … }` / `T { … }` expression from a block, if and only if:
/// - The block has exactly one statement that is an expression (with or without semicolon)
/// - That expression is `ExprStruct` with no spread (`..`)
fn extract_struct_expr(block: &Block) -> Option<ExprStruct> {
	let expr: &Expr = match block.stmts.as_slice() {
		[syn::Stmt::Expr(e, None)] => e,
		[syn::Stmt::Expr(e, Some(_))] => e,
		_ => return None,
	};

	let Expr::Struct(expr_struct) = expr else {
		return None;
	};

	if expr_struct.rest.is_some() {
		return None;
	}

	Some(expr_struct.clone())
}

/// Build a map of field name -> source text of the initialiser expression.
fn build_field_inits(content: &str, expr_struct: &ExprStruct) -> HashMap<String, String> {
	let mut map = HashMap::default();
	for field in &expr_struct.fields {
		let syn::Member::Named(ref ident) = field.member else {
			continue;
		};
		let text = expr_src(content, &field.expr);
		map.insert(ident.to_string(), text);
	}
	map
}

/// Extract the source text for an expression using its span.
fn expr_src(content: &str, expr: &Expr) -> String {
	let span = expr.span();
	let start = span_to_byte(content, span.start()).unwrap_or(0);
	let end = span_to_byte(content, span.end()).unwrap_or(content.len());
	content[start..end].to_string()
}

/// Build a compound Fix that rewrites the struct with inline defaults AND deletes the impl block,
/// all in a single byte-range replacement.
///
/// The fix spans [struct_start .. impl_end_with_newline] and replaces everything with just
/// the rewritten struct text. This avoids leaving an intermediate state that syn can't parse.
///
/// Returns `None` if the struct and impl are not adjacent (real code between them), in which
/// case the violation is emitted without a fix.
fn make_compound_fix(content: &str, struct_item: &ItemStruct, impl_block: &ItemImpl, field_inits: &HashMap<String, String>) -> Option<Fix> {
	let struct_start = span_to_byte(content, struct_item.span().start())?;
	let struct_end = span_to_byte(content, struct_item.span().end())?;
	let impl_start = span_to_byte(content, impl_block.span().start())?;
	let impl_end = span_to_byte(content, impl_block.span().end())?;

	// Struct must come before impl
	if struct_start >= impl_start {
		return None;
	}

	// Between struct_end and impl_start there must be only whitespace/blank lines
	let between = &content[struct_end..impl_start];
	if !between.chars().all(|c| c.is_whitespace()) {
		// Non-adjacent: cannot safely merge into one fix
		return None;
	}

	// Build the rewritten struct text
	let rewritten_struct = rewrite_struct(content, struct_item, struct_start, struct_end, field_inits)?;

	// Consume trailing newline after impl `}`
	let delete_end = consume_trailing_newline(content, impl_end);

	Some(Fix {
		start_byte: struct_start,
		end_byte: delete_end,
		replacement: rewritten_struct,
	})
}

/// Rewrite struct source by inserting ` = <init>` after each field's type annotation.
fn rewrite_struct(content: &str, struct_item: &ItemStruct, struct_start: usize, struct_end: usize, field_inits: &HashMap<String, String>) -> Option<String> {
	let Fields::Named(ref named) = struct_item.fields else {
		return None;
	};

	let struct_src = &content[struct_start..struct_end];

	// Collect splice points: (rel_pos, insert_text) where rel_pos is relative to struct_src.
	// We insert ` = <init>` at the end of each field's span (after the type annotation).
	let mut splices: Vec<(usize, String)> = Vec::default();

	for field in &named.named {
		let ident = field.ident.as_ref()?;
		let init = field_inits.get(&ident.to_string())?;

		let field_end = span_to_byte(content, field.span().end())?;
		let rel_end = field_end - struct_start;
		splices.push((rel_end, format!(" = {init}")));
	}

	// Apply right-to-left to preserve offsets
	splices.sort_by_key(|&(pos, _)| std::cmp::Reverse(pos));

	let mut result = struct_src.to_string();
	for (pos, insertion) in splices {
		result.insert_str(pos, &insertion);
	}

	Some(result)
}

/// Advance past the newline immediately after `pos`.
fn consume_trailing_newline(content: &str, pos: usize) -> usize {
	if content.as_bytes().get(pos) == Some(&b'\n') { pos + 1 } else { pos }
}

fn span_to_byte(content: &str, pos: proc_macro2::LineColumn) -> Option<usize> {
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
