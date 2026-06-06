//! Lint to disallow usage of `tokio::spawn`.
//!
//! Spawning unstructured tasks leads to difficult-to-reason-about concurrency.
//! See: "Go statement considered harmful" - <https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful>

use std::path::Path;

use proc_macro2::Span;
use syn::{Attribute, Expr, ExprCall, ExprPath, ItemFn, ItemMod, Meta, spanned::Spanned, visit::Visit};

use super::{Violation, skip::SkipVisitor};

const RULE: &str = "no-tokio-spawn";
const GO_STATEMENT_HARMFUL_URL: &str = "https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/";
pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let visitor = TokioSpawnVisitor::new(path);
	let mut skip_visitor = SkipVisitor::for_rule(visitor, content, RULE);
	skip_visitor.visit_file(file);
	skip_visitor.inner.violations
}

struct TokioSpawnVisitor {
	path_str: String,
	violations: Vec<Violation>,
	/// Nesting depth of enclosing test contexts (`#[test]`/`#[tokio::test]` fns and
	/// `#[cfg(test)]` modules). Spawns inside tests are intentionally allowed — unstructured
	/// concurrency in a test is short-lived and torn down with the test runtime.
	test_depth: usize,
}

impl TokioSpawnVisitor {
	fn new(path: &Path) -> Self {
		Self {
			path_str: path.display().to_string(),
			violations: Vec::default(),
			test_depth: 0,
		}
	}

	fn report_tokio_spawn(&mut self, span: Span, variant: &str) {
		self.violations.push(Violation {
			rule: RULE,
			file: self.path_str.clone(),
			line: span.start().line,
			column: span.start().column,
			message: format!(
				"Usage of `{variant}` is disallowed. Unstructured concurrency makes code harder to reason about. \
				 See: {GO_STATEMENT_HARMFUL_URL}"
			),
			fix: None, // No auto-fix - requires architectural changes
		});
	}

	fn is_tokio_spawn_path(&self, path: &syn::Path) -> Option<&'static str> {
		let segments: Vec<_> = path.segments.iter().map(|s| s.ident.to_string()).collect();
		let segments_str: Vec<&str> = segments.iter().map(|s| s.as_str()).collect();

		// Note: spawn_blocking is allowed - it runs sync code on a blocking thread pool
		// and doesn't create unstructured concurrent tasks
		match segments_str.as_slice() {
			["tokio", "spawn"] => Some("tokio::spawn"),
			["tokio", "spawn_local"] => Some("tokio::spawn_local"),
			["tokio", "task", "spawn"] => Some("tokio::task::spawn"),
			["tokio", "task", "spawn_local"] => Some("tokio::task::spawn_local"),
			_ => None,
		}
	}
}

impl<'a> Visit<'a> for TokioSpawnVisitor {
	fn visit_expr_call(&mut self, node: &'a ExprCall) {
		if self.test_depth == 0
			&& let Expr::Path(ExprPath { path, .. }) = &*node.func
			&& let Some(variant) = self.is_tokio_spawn_path(path)
		{
			self.report_tokio_spawn(node.func.span(), variant);
		}
		syn::visit::visit_expr_call(self, node);
	}

	fn visit_item_fn(&mut self, node: &'a ItemFn) {
		let is_test = node.attrs.iter().any(is_test_attr);
		self.test_depth += usize::from(is_test);
		syn::visit::visit_item_fn(self, node);
		self.test_depth -= usize::from(is_test);
	}

	fn visit_item_mod(&mut self, node: &'a ItemMod) {
		let is_test = node.attrs.iter().any(is_cfg_test_attr);
		self.test_depth += usize::from(is_test);
		syn::visit::visit_item_mod(self, node);
		self.test_depth -= usize::from(is_test);
	}
}

/// A `#[test]` / `#[tokio::test]` / `#[cfg(test)]` attribute on a function — anything marking it
/// as test-only code. The last segment matching `test` covers `tokio::test`, `rstest::rstest`-style
/// renames are not handled, but the common runtime attributes are.
fn is_test_attr(attr: &Attribute) -> bool {
	if is_cfg_test_attr(attr) {
		return true;
	}
	attr.path().segments.last().is_some_and(|s| s.ident == "test")
}

/// A `#[cfg(test)]` attribute.
fn is_cfg_test_attr(attr: &Attribute) -> bool {
	if !attr.path().is_ident("cfg") {
		return false;
	}
	let Meta::List(list) = &attr.meta else { return false };
	list.tokens.to_string().split(|c: char| !c.is_alphanumeric() && c != '_').any(|t| t == "test")
}
