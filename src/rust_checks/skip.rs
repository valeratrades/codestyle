//! Utility for detecting `codestyle::skip` markers on items.
//!
//! When an item is marked with this marker, all codestyle checks should skip it entirely.
//!
//! Supported formats (as comments to avoid compiler errors):
//! - `//#[codestyle::skip]`
//! - `// #[codestyle::skip]`
//! - `//@codestyle::skip`
//! - `// @codestyle::skip`

use proc_macro2::Span;
use syn::visit::Visit;

/// Check if the line before the given span contains a codestyle::skip marker.
pub fn has_skip_marker(content: &str, span: Span) -> bool {
	let line = span.start().line;
	has_skip_marker_at_line(content, line)
}

/// Check if the given line or the line above contains a codestyle::skip marker.
fn has_skip_marker_at_line(content: &str, line: usize) -> bool {
	let lines: Vec<&str> = content.lines().collect();

	// Check current line (inline comment)
	if line > 0 && line <= lines.len() {
		let current_line = lines[line - 1];
		if is_skip_comment(current_line) {
			return true;
		}
	}

	// Check line above
	if line > 1 {
		let prev_line = lines[line - 2];
		if is_skip_comment(prev_line) {
			return true;
		}
	}

	false
}

/// Check if a line contains a codestyle::skip comment marker.
fn is_skip_comment(line: &str) -> bool {
	let trimmed = line.trim();

	// //#[codestyle::skip] or // #[codestyle::skip]
	if let Some(after_slashes) = trimmed.strip_prefix("//") {
		let after_slashes = after_slashes.trim_start();
		if after_slashes.starts_with("#[codestyle::skip]") || after_slashes.starts_with("@codestyle::skip") {
			return true;
		}
	}

	false
}

/// A visitor wrapper that automatically skips items marked with codestyle::skip.
///
/// Wrap your visitor with this to get automatic skip handling without duplicating
/// the skip logic in every check module.
pub struct SkipVisitor<'a, V> {
	pub inner: V,
	pub content: &'a str,
}

impl<'a, V> SkipVisitor<'a, V> {
	pub fn new(inner: V, content: &'a str) -> Self {
		Self { inner, content }
	}
}

/// Macro for container items that can have skip markers.
/// For these, we check the skip marker, then delegate to the inner visitor.
/// The inner visitor is responsible for both its checks AND recursion.
macro_rules! impl_skip_visit_container {
	($method:ident, $type:ty) => {
		fn $method(&mut self, node: &'ast $type) {
			if has_skip_marker(self.content, syn::spanned::Spanned::span(node)) {
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
