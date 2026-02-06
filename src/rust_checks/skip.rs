//! Utility for detecting `codestyle::skip` markers on items.
//!
//! When an item is marked with this marker, codestyle checks should skip it.
//!
//! Supported formats (as comments to avoid compiler errors):
//! - `//#[codestyle::skip]` - skip all rules
//! - `// #[codestyle::skip]` - skip all rules
//! - `//@codestyle::skip` - skip all rules
//! - `// @codestyle::skip` - skip all rules
//! - `//#[codestyle::skip(rule-name)]` - skip specific rule
//! - `// #[codestyle::skip(rule-name)]` - skip specific rule
//! - `//@codestyle::skip(rule-name)` - skip specific rule
//! - `// @codestyle::skip(rule-name)` - skip specific rule

use proc_macro2::Span;
use syn::visit::Visit;

/// Result of parsing a skip marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipMarker {
	/// Skip all rules
	All,
	/// Skip only the specified rule
	Rule(String),
}

/// Check if the line before the given span contains a codestyle::skip marker.
/// Returns `true` if there's a skip-all marker.
pub fn has_skip_marker(content: &str, span: Span) -> bool {
	let line = span.start().line;
	has_skip_marker_at_line(content, line)
}

/// Check if the line before the given span contains a codestyle::skip marker for a specific rule.
/// Returns `true` if there's a skip-all marker OR a skip marker for the specified rule.
pub fn has_skip_marker_for_rule(content: &str, span: Span, rule: &str) -> bool {
	let line = span.start().line;
	has_skip_marker_for_rule_at_line(content, line, rule)
}

/// Check if the given line or the line above contains a codestyle::skip marker for a specific rule.
pub fn has_skip_marker_for_rule_at_line(content: &str, line: usize, rule: &str) -> bool {
	match get_skip_marker_at_line(content, line) {
		Some(SkipMarker::All) => true,
		Some(SkipMarker::Rule(r)) => r == rule,
		None => false,
	}
}

/// A visitor wrapper that automatically skips items marked with codestyle::skip.
///
/// Wrap your visitor with this to get automatic skip handling without duplicating
/// the skip logic in every check module.
///
/// Supports both skip-all markers (`//#[codestyle::skip]`) and rule-specific markers
/// (`//#[codestyle::skip(rule-name)]`).
pub struct SkipVisitor<'a, V> {
	pub inner: V,
	pub content: &'a str,
	/// The rule name to check for rule-specific skips. If None, only skip-all markers are checked.
	pub rule: Option<&'a str>,
}
impl<'a, V> SkipVisitor<'a, V> {
	/// Create a SkipVisitor that only checks for skip-all markers.
	pub fn new(inner: V, content: &'a str) -> Self {
		Self { inner, content, rule: None }
	}

	/// Create a SkipVisitor that checks for skip-all markers and rule-specific markers.
	pub fn for_rule(inner: V, content: &'a str, rule: &'a str) -> Self {
		Self { inner, content, rule: Some(rule) }
	}

	fn should_skip(&self, span: Span) -> bool {
		let line = span.start().line;
		match get_skip_marker_at_line(self.content, line) {
			Some(SkipMarker::All) => true,
			Some(SkipMarker::Rule(r)) => self.rule.is_some_and(|rule| r == rule),
			None => false,
		}
	}
}

/// Check if the given line or the line above contains a codestyle::skip marker (skip-all only).
fn has_skip_marker_at_line(content: &str, line: usize) -> bool {
	matches!(get_skip_marker_at_line(content, line), Some(SkipMarker::All))
}

/// Get the skip marker at the given line or the line above.
fn get_skip_marker_at_line(content: &str, line: usize) -> Option<SkipMarker> {
	let lines: Vec<&str> = content.lines().collect();

	// Check current line (inline comment)
	if line > 0 && line <= lines.len() {
		let current_line = lines[line - 1];
		if let Some(marker) = parse_skip_comment(current_line) {
			return Some(marker);
		}
	}

	// Check line above
	if line > 1 {
		let prev_line = lines[line - 2];
		if let Some(marker) = parse_skip_comment(prev_line) {
			return Some(marker);
		}
	}

	None
}

/// Parse a skip comment and return the skip marker if present.
fn parse_skip_comment(line: &str) -> Option<SkipMarker> {
	let trimmed = line.trim();

	// //#[codestyle::skip...] or // #[codestyle::skip...]
	let after_slashes = trimmed.strip_prefix("//")?;
	let after_slashes = after_slashes.trim_start();

	// Try #[codestyle::skip...] format
	if let Some(rest) = after_slashes.strip_prefix("#[codestyle::skip") {
		return parse_skip_suffix(rest);
	}

	// Try @codestyle::skip... format
	if let Some(rest) = after_slashes.strip_prefix("@codestyle::skip") {
		return parse_skip_suffix(rest);
	}

	None
}

/// Parse the suffix after "codestyle::skip" to determine if it's skip-all or skip-specific.
fn parse_skip_suffix(rest: &str) -> Option<SkipMarker> {
	let rest = rest.trim_start();

	// skip] or just end of line for @-style -> skip all
	if rest.is_empty() || rest.starts_with(']') {
		return Some(SkipMarker::All);
	}

	// (rule-name)] -> skip specific rule
	if let Some(after_paren) = rest.strip_prefix('(') {
		// Find the closing paren
		let end = after_paren.find(')')?;
		let rule_name = after_paren[..end].trim();
		if !rule_name.is_empty() {
			return Some(SkipMarker::Rule(rule_name.to_string()));
		}
	}

	None
}

/// Macro for container items that can have skip markers.
/// For these, we check the skip marker, then delegate to the inner visitor.
/// The inner visitor is responsible for both its checks AND recursion.
macro_rules! impl_skip_visit_container {
	($method:ident, $type:ty) => {
		fn $method(&mut self, node: &'ast $type) {
			if self.should_skip(syn::spanned::Spanned::span(node)) {
				return;
			}
			// Delegate to inner visitor - it handles its own checks and recursion
			self.inner.$method(node);
		}
	};
}

impl<'ast, V: Visit<'ast>> Visit<'ast> for SkipVisitor<'_, V> {
	impl_skip_visit_container!(visit_item_fn, syn::ItemFn);

	impl_skip_visit_container!(visit_item_mod, syn::ItemMod);

	impl_skip_visit_container!(visit_item_impl, syn::ItemImpl);

	impl_skip_visit_container!(visit_item_struct, syn::ItemStruct);

	impl_skip_visit_container!(visit_item_enum, syn::ItemEnum);

	impl_skip_visit_container!(visit_item_trait, syn::ItemTrait);

	impl_skip_visit_container!(visit_item_type, syn::ItemType);

	impl_skip_visit_container!(visit_item_const, syn::ItemConst);

	impl_skip_visit_container!(visit_item_static, syn::ItemStatic);

	impl_skip_visit_container!(visit_item_use, syn::ItemUse);

	impl_skip_visit_container!(visit_expr_block, syn::ExprBlock);

	impl_skip_visit_container!(visit_local, syn::Local);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_skip_all_bracket() {
		assert_eq!(parse_skip_comment("//#[codestyle::skip]"), Some(SkipMarker::All));
		assert_eq!(parse_skip_comment("// #[codestyle::skip]"), Some(SkipMarker::All));
		assert_eq!(parse_skip_comment("  //#[codestyle::skip]"), Some(SkipMarker::All));
		assert_eq!(parse_skip_comment("  // #[codestyle::skip]  "), Some(SkipMarker::All));
	}

	#[test]
	fn parse_skip_all_at() {
		assert_eq!(parse_skip_comment("//@codestyle::skip"), Some(SkipMarker::All));
		assert_eq!(parse_skip_comment("// @codestyle::skip"), Some(SkipMarker::All));
		assert_eq!(parse_skip_comment("  //@codestyle::skip"), Some(SkipMarker::All));
	}

	#[test]
	fn parse_skip_specific_rule_bracket() {
		assert_eq!(parse_skip_comment("//#[codestyle::skip(pub-first)]"), Some(SkipMarker::Rule("pub-first".to_string())));
		assert_eq!(
			parse_skip_comment("// #[codestyle::skip(ignored-error-comment)]"),
			Some(SkipMarker::Rule("ignored-error-comment".to_string()))
		);
		assert_eq!(parse_skip_comment("//#[codestyle::skip( loop-comment )]"), Some(SkipMarker::Rule("loop-comment".to_string())));
	}

	#[test]
	fn parse_skip_specific_rule_at() {
		assert_eq!(parse_skip_comment("//@codestyle::skip(pub-first)"), Some(SkipMarker::Rule("pub-first".to_string())));
		assert_eq!(parse_skip_comment("// @codestyle::skip(no-chrono)"), Some(SkipMarker::Rule("no-chrono".to_string())));
	}

	#[test]
	fn parse_skip_not_a_skip() {
		assert_eq!(parse_skip_comment("// some other comment"), None);
		assert_eq!(parse_skip_comment("let x = 1;"), None);
		assert_eq!(parse_skip_comment("// codestyle::skip"), None); // missing # or @
	}

	#[test]
	fn has_skip_marker_for_rule_matches() {
		let content = "//#[codestyle::skip(pub-first)]\nfn foo() {}";
		assert!(has_skip_marker_for_rule_at_line(content, 2, "pub-first"));
		assert!(!has_skip_marker_for_rule_at_line(content, 2, "other-rule"));
	}

	#[test]
	fn has_skip_marker_for_rule_all_matches_any() {
		let content = "//#[codestyle::skip]\nfn foo() {}";
		assert!(has_skip_marker_for_rule_at_line(content, 2, "pub-first"));
		assert!(has_skip_marker_for_rule_at_line(content, 2, "any-rule"));
	}

	#[test]
	fn has_skip_marker_all_ignores_specific() {
		// has_skip_marker (skip-all only) should NOT match rule-specific skips
		let content = "//#[codestyle::skip(pub-first)]\nfn foo() {}";
		assert!(!has_skip_marker_at_line(content, 2));
	}
}
