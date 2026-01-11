use std::{collections::HashSet, path::Path};

use proc_macro2::{Span, TokenStream, TokenTree};
use syn::{ExprMacro, Macro, spanned::Spanned, visit::Visit};

use super::{Fix, Violation};

const FORMAT_MACROS: &[&str] = &[
	"format", "write", "writeln", "print", "println", "eprint", "eprintln", "panic", "format_args", "log", "trace", "debug", "info", "warn", "error", "assert", "assert_eq", "assert_ne",
	"debug_assert", "debug_assert_eq", "debug_assert_ne",
];

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let mut visitor = FormatMacroVisitor::new(path, content);
	visitor.visit_file(file);
	visitor.violations
}

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

		let empty_placeholder_count = count_empty_placeholders(&format_string_content);
		if empty_placeholder_count == 0 {
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

		let placeholder_positions = find_empty_placeholder_positions(&format_string_content);

		if placeholder_positions.len() != args.len() {
			return;
		}

		let simple_args: Vec<(usize, &str, Span)> = placeholder_positions
			.iter()
			.zip(args.iter())
			.filter_map(
				|(pos, (arg_str, arg_span))| {
					if is_simple_identifier(arg_str) { Some((*pos, arg_str.as_str(), *arg_span)) } else { None }
				},
			)
			.collect();

		if simple_args.is_empty() {
			return;
		}

		// Build set of indices for simple args
		let simple_indices: std::collections::HashSet<usize> = placeholder_positions
			.iter()
			.zip(args.iter())
			.enumerate()
			.filter_map(|(idx, (_, (arg_str, _)))| if is_simple_identifier(arg_str) { Some(idx) } else { None })
			.collect();

		// Build new format string with simple vars embedded
		let mut new_fmt = format_string_content.clone();
		for (pos, arg_str, _) in simple_args.iter().rev() {
			let end_pos = pos + 2;
			new_fmt.replace_range(*pos..end_pos, &format!("{{{arg_str}}}"));
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

		for (_, arg_str, arg_span) in &simple_args {
			self.violations.push(Violation {
				rule: "embed-simple-vars",
				file: self.path_str.clone(),
				line: arg_span.start().line,
				column: arg_span.start().column,
				message: format!("variable `{arg_str}` should be embedded in format string: use `{{{arg_str}}}` instead of `{{}}, {arg_str}`"),
				fix: fix.clone(),
			});
		}
	}
}

impl<'a> Visit<'a> for FormatMacroVisitor<'a> {
	fn visit_expr_macro(&mut self, node: &'a ExprMacro) {
		self.check_format_macro(&node.mac);
		syn::visit::visit_expr_macro(self, node);
	}

	fn visit_macro(&mut self, node: &'a Macro) {
		self.check_format_macro(node);
		syn::visit::visit_macro(self, node);
	}
}

fn count_empty_placeholders(format_str: &str) -> usize {
	let mut count = 0;
	let mut chars = format_str.chars().peekable();

	while let Some(c) = chars.next() {
		if c == '{'
			&& let Some(&next) = chars.peek()
		{
			if next == '{' {
				chars.next();
			} else if next == '}' {
				count += 1;
				chars.next();
			} else {
				for c in chars.by_ref() {
					if c == '}' {
						break;
					}
				}
			}
		}
	}

	count
}

fn find_empty_placeholder_positions(format_str: &str) -> Vec<usize> {
	let mut positions = Vec::new();
	let mut chars = format_str.char_indices().peekable();

	while let Some((idx, c)) = chars.next() {
		if c == '{'
			&& let Some(&(_, next)) = chars.peek()
		{
			if next == '{' {
				chars.next();
			} else if next == '}' {
				positions.push(idx);
				chars.next();
			} else {
				for (_, c) in chars.by_ref() {
					if c == '}' {
						break;
					}
				}
			}
		}
	}

	positions
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
/// Lines are 1-indexed, columns are 0-indexed (byte offset within line).
fn span_position_to_byte(content: &str, line: usize, column: usize) -> Option<usize> {
	let mut current_line = 1;
	let mut line_start = 0;

	for (i, ch) in content.char_indices() {
		if current_line == line {
			// Found the line, add column offset
			// Column is byte offset, not char offset
			return Some(line_start + column);
		}
		if ch == '\n' {
			current_line += 1;
			line_start = i + 1;
		}
	}

	// Handle last line (no trailing newline)
	if current_line == line {
		return Some(line_start + column);
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
