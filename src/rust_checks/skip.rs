//! Utility for detecting `#[codestyle::skip]` attributes on items.
//!
//! When an item is marked with this attribute, all codestyle checks should skip it entirely.

use syn::Attribute;

/// Check if any of the attributes contain `#[codestyle::skip]`.
pub fn has_skip_attr(attrs: &[Attribute]) -> bool {
	attrs.iter().any(is_skip_attr)
}

/// Check if a single attribute is `#[codestyle::skip]`.
fn is_skip_attr(attr: &Attribute) -> bool {
	let path = attr.path();
	let segments: Vec<_> = path.segments.iter().collect();

	if segments.len() != 2 {
		return false;
	}

	segments[0].ident == "codestyle" && segments[1].ident == "skip"
}
