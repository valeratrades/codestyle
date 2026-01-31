use std::{collections::HashSet, path::Path};

use proc_macro2::{Span, TokenStream, TokenTree};
use syn::{ExprMacro, Macro, spanned::Spanned, visit::Visit};

use super::{Fix, Violation, skip::has_skip_attr};

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let mut visitor = FormatMacroVisitor::new(path, content);
	visitor.visit_file(file);
	visitor.violations
}
const FORMAT_MACROS: &[&str] = &[
	// std formatting
	"format", "write", "writeln", "print", "println", "eprint", "eprintln", "format_args", // std panicking/unreachable
	"panic", "todo", "unimplemented", "unreachable", // logging (log crate, tracing uses same names)
	"log", "trace", "debug", "info", "warn", "error", // assertions
	"assert", "assert_eq", "assert_ne", "debug_assert", "debug_assert_eq", "debug_assert_ne", // error handling (anyhow, eyre, etc.)
	"bail", "ensure", "anyhow", "eyre",
];

struct FormatMacroVisitor<'a> {
	path_str: String,
	content: &'a str,
	violations: Vec<Violation>,
	seen_spans: HashSet<(usize, usize)>,
}

impl<'a> FormatMacroVisitor<'a> {
	fn new(path: &Path, content: &'a str) -> Self {
		Self {
			path_str: path.display().to_string(),
			content,
			violations: Vec::new(),
			seen_spans: HashSet::new(),
		}
	}

	fn check_format_macro(&mut self, mac: &Macro) {
		// Deduplicate based on span start position
		let start = mac.span().start();
		let key = (start.line, start.column);
		if self.seen_spans.contains(&key) {
			return;
		}
		self.seen_spans.insert(key);

		let macro_name = mac.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();

		if !FORMAT_MACROS.contains(&macro_name.as_str()) {
			return;
		}

		self.analyze_format_macro_tokens(&mac.tokens);
	}

	fn analyze_format_macro_tokens(&mut self, tokens: &TokenStream) {
		let tokens: Vec<TokenTree> = tokens.clone().into_iter().collect();

		// Find the format string (first string literal)
		let mut format_string_idx = None;
		let mut format_string_content = String::new();
		let mut format_string_span: Option<Span> = None;

		for (i, token) in tokens.iter().enumerate() {
			if let TokenTree::Literal(lit) = token {
				let lit_str = lit.to_string();
				if lit_str.starts_with('"') || lit_str.starts_with("r#") || lit_str.starts_with("r\"") {
					format_string_idx = Some(i);
					format_string_content = lit_str;
					format_string_span = Some(lit.span());
					break;
				}
			}
		}

		let Some(fmt_idx) = format_string_idx else {
			return;
		};
		let Some(fmt_span) = format_string_span else {
			return;
		};

		let placeholder_count = count_embeddable_placeholders(&format_string_content);
		if placeholder_count == 0 {
			return;
		}

		// Collect arguments after format string
		let mut args: Vec<(String, Span)> = Vec::new();
		let mut i = fmt_idx + 1;

		while i < tokens.len() {
			if let TokenTree::Punct(p) = &tokens[i]
				&& p.as_char() == ','
			{
				i += 1;
				continue;
			}

			if let Some((arg_str, arg_span, next_i)) = collect_argument(&tokens, i) {
				args.push((arg_str, arg_span));
				i = next_i;
			} else {
				i += 1;
			}
		}

		let placeholders = find_embeddable_placeholders(&format_string_content);

		if placeholders.len() != args.len() {
			return;
		}

		// Collect simple args with their placeholder info
		let simple_args: Vec<(&Placeholder, &str, Span)> = placeholders
			.iter()
			.zip(args.iter())
			.filter_map(|(placeholder, (arg_str, arg_span))| {
				if is_simple_identifier(arg_str) {
					Some((placeholder, arg_str.as_str(), *arg_span))
				} else {
					None
				}
			})
			.collect();

		if simple_args.is_empty() {
			return;
		}

		// Build set of indices for simple args
		let simple_indices: std::collections::HashSet<usize> = placeholders
			.iter()
			.zip(args.iter())
			.enumerate()
			.filter_map(|(idx, (_, (arg_str, _)))| if is_simple_identifier(arg_str) { Some(idx) } else { None })
			.collect();

		// Build new format string with simple vars embedded
		let mut new_fmt = format_string_content.clone();
		for (placeholder, arg_str, _) in simple_args.iter().rev() {
			// Replace the placeholder with {var} or {var:?} or {var:#?}
			let replacement = format!("{{{arg_str}{}}}", placeholder.specifier);
			new_fmt.replace_range(placeholder.start..placeholder.end, &replacement);
		}

		// Build remaining args (non-simple ones only)
		let remaining_args: Vec<&str> = args
			.iter()
			.enumerate()
			.filter_map(|(idx, (arg_str, _))| if simple_indices.contains(&idx) { None } else { Some(arg_str.as_str()) })
			.collect();

		// Create fix
		let fix = if remaining_args.is_empty() {
			// All args were simple, just replace format string through last arg
			let last_arg_span = args.last().map(|(_, span)| *span);
			create_full_macro_fix(&new_fmt, fmt_span, last_arg_span, self.content)
		} else {
			// Some args remain, need to build "new_fmt", remaining_args...
			let remaining_args_str = remaining_args.join(", ");
			let replacement = format!("{new_fmt}, {remaining_args_str}");
			let last_arg_span = args.last().map(|(_, span)| *span);
			create_full_macro_fix(&replacement, fmt_span, last_arg_span, self.content)
		};

		for (placeholder, arg_str, arg_span) in &simple_args {
			let spec_display = if placeholder.specifier.is_empty() {
				"{}".to_string()
			} else {
				format!("{{{}}}", placeholder.specifier)
			};
			self.violations.push(Violation {
				rule: "embed-simple-vars",
				file: self.path_str.clone(),
				line: arg_span.start().line,
				column: arg_span.start().column,
				message: format!(
					"variable `{arg_str}` should be embedded in format string: use `{{{arg_str}{}}}` instead of `{spec_display}, {arg_str}`",
					placeholder.specifier
				),
				fix: fix.clone(),
			});
		}
	}
}

impl<'a> Visit<'a> for FormatMacroVisitor<'a> {
	fn visit_item_fn(&mut self, node: &'a syn::ItemFn) {
		if has_skip_attr(&node.attrs) {
			return;
		}
		syn::visit::visit_item_fn(self, node);
	}

	fn visit_item_mod(&mut self, node: &'a syn::ItemMod) {
		if has_skip_attr(&node.attrs) {
			return;
		}
		syn::visit::visit_item_mod(self, node);
	}

	fn visit_item_impl(&mut self, node: &'a syn::ItemImpl) {
		if has_skip_attr(&node.attrs) {
			return;
		}
		syn::visit::visit_item_impl(self, node);
	}

	fn visit_expr_block(&mut self, node: &'a syn::ExprBlock) {
		if has_skip_attr(&node.attrs) {
			return;
		}
		syn::visit::visit_expr_block(self, node);
	}

	fn visit_local(&mut self, node: &'a syn::Local) {
		if has_skip_attr(&node.attrs) {
			return;
		}
		syn::visit::visit_local(self, node);
	}

	fn visit_expr_macro(&mut self, node: &'a ExprMacro) {
		self.check_format_macro(&node.mac);
		syn::visit::visit_expr_macro(self, node);
	}

	fn visit_macro(&mut self, node: &'a Macro) {
		self.check_format_macro(node);
		syn::visit::visit_macro(self, node);
	}
}

/// Represents a placeholder in a format string that can have a variable embedded.
/// The `specifier` is the format specifier (e.g., `:?`, `:#?`, or empty for Display).
#[derive(Clone, Debug)]
struct Placeholder {
	start: usize,
	end: usize,
	specifier: String,
}

fn count_embeddable_placeholders(format_str: &str) -> usize {
	find_embeddable_placeholders(format_str).len()
}

/// Find placeholders that can have variables embedded into them.
/// This includes `{}`, `{:?}`, and `{:#?}`.
fn find_embeddable_placeholders(format_str: &str) -> Vec<Placeholder> {
	let mut placeholders = Vec::new();
	let bytes = format_str.as_bytes();
	let mut i = 0;

	while i < bytes.len() {
		if bytes[i] == b'{' {
			if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
				// Escaped brace, skip
				i += 2;
				continue;
			}

			let start = i;
			i += 1;

			// Find the end of this placeholder
			let mut end = None;
			let mut j = i;
			while j < bytes.len() {
				if bytes[j] == b'}' {
					end = Some(j);
					break;
				}
				j += 1;
			}

			let Some(end_pos) = end else {
				// No closing brace found, malformed
				continue;
			};

			let content = &format_str[i..end_pos];

			// Check if this is an embeddable placeholder:
			// - "{}" (empty)
			// - "{:specifier}" (any format specifier without a variable name)
			// We don't want to match placeholders that already have a variable name like "{foo:?}"
			let specifier = if content.is_empty() {
				String::new()
			} else if content.starts_with(':') {
				// Format specifier without variable name (e.g., ":?", ":#?", ":.0", ":>10")
				content.to_string()
			} else {
				// Has other content (named variable like "foo" or "foo:?"), skip
				i = end_pos + 1;
				continue;
			};

			placeholders.push(Placeholder { start, end: end_pos + 1, specifier });

			i = end_pos + 1;
		} else {
			i += 1;
		}
	}

	placeholders
}

fn is_simple_identifier(s: &str) -> bool {
	if s.is_empty() {
		return false;
	}

	let mut chars = s.chars();
	let first = chars.next().unwrap();

	if !first.is_alphabetic() && first != '_' {
		return false;
	}

	chars.all(|c| c.is_alphanumeric() || c == '_')
}

fn collect_argument(tokens: &[TokenTree], start: usize) -> Option<(String, Span, usize)> {
	if start >= tokens.len() {
		return None;
	}

	let first = &tokens[start];

	if let TokenTree::Ident(ident) = first {
		if start + 1 < tokens.len()
			&& let TokenTree::Punct(p) = &tokens[start + 1]
		{
			let ch = p.as_char();
			if ch == '.' || ch == ':' {
				return collect_complex_argument(tokens, start);
			}
		}
		return Some((ident.to_string(), ident.span(), start + 1));
	}

	collect_complex_argument(tokens, start)
}

fn collect_complex_argument(tokens: &[TokenTree], start: usize) -> Option<(String, Span, usize)> {
	let mut result = String::new();
	let mut i = start;
	let start_span = tokens.get(start)?.span();
	let mut last_span = start_span;
	let mut depth = 0;

	while i < tokens.len() {
		let token = &tokens[i];

		match token {
			TokenTree::Punct(p) if p.as_char() == ',' && depth == 0 => {
				break;
			}
			TokenTree::Group(g) => {
				depth += 1;
				result.push_str(&g.to_string());
				last_span = g.span();
				depth -= 1;
			}
			_ => {
				result.push_str(&token.to_string());
				last_span = token.span();
			}
		}

		i += 1;
	}

	// Return last_span so that the end position covers the whole argument
	if result.is_empty() { None } else { Some((result.trim().to_string(), last_span, i)) }
}

/// Convert a proc_macro2 line/column position to byte offset in content.
/// Lines are 1-indexed, columns are 0-indexed character offsets within line.
fn span_position_to_byte(content: &str, line: usize, column: usize) -> Option<usize> {
	let mut current_line = 1;
	let mut line_start = 0;

	for (i, ch) in content.char_indices() {
		if current_line == line {
			// Found the line, convert character offset to byte offset
			let line_content = &content[line_start..];
			let byte_offset: usize = line_content.char_indices().take(column).map(|(_, c)| c.len_utf8()).sum();
			return Some(line_start + byte_offset);
		}
		if ch == '\n' {
			current_line += 1;
			line_start = i + 1;
		}
	}

	// Handle last line (no trailing newline)
	if current_line == line {
		let line_content = &content[line_start..];
		let byte_offset: usize = line_content.char_indices().take(column).map(|(_, c)| c.len_utf8()).sum();
		return Some(line_start + byte_offset);
	}

	None
}

fn create_full_macro_fix(new_fmt: &str, fmt_span: Span, last_arg_span: Option<Span>, content: &str) -> Option<Fix> {
	let last_arg_span = last_arg_span?;

	// Get byte position of format string start
	let fmt_start = span_position_to_byte(content, fmt_span.start().line, fmt_span.start().column)?;

	// Get byte position after the last argument
	let last_arg_end = span_position_to_byte(content, last_arg_span.end().line, last_arg_span.end().column)?;

	// Verify the format string is where we expect
	if !content[fmt_start..].starts_with('"') && !content[fmt_start..].starts_with("r#") && !content[fmt_start..].starts_with("r\"") {
		return None;
	}

	Some(Fix {
		start_byte: fmt_start,
		end_byte: last_arg_end,
		replacement: new_fmt.to_string(),
	})
}
