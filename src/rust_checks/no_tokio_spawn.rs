//! Lint to disallow usage of `tokio::spawn`.
//!
//! Spawning unstructured tasks leads to difficult-to-reason-about concurrency.
//! See: "Go statement considered harmful" - https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/

use std::path::Path;

use proc_macro2::Span;
use syn::{Expr, ExprCall, ExprPath, spanned::Spanned, visit::Visit};

use super::Violation;

pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let mut visitor = TokioSpawnVisitor::new(path, content);
	visitor.visit_file(file);
	visitor.violations
}
const GO_STATEMENT_HARMFUL_URL: &str = "https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/";

struct TokioSpawnVisitor<'a> {
	path_str: String,
	#[expect(unused)]
	content: &'a str,
	violations: Vec<Violation>,
}

impl<'a> TokioSpawnVisitor<'a> {
	fn new(path: &Path, content: &'a str) -> Self {
		Self {
			path_str: path.display().to_string(),
			content,
			violations: Vec::new(),
		}
	}

	fn report_tokio_spawn(&mut self, span: Span, variant: &str) {
		self.violations.push(Violation {
			rule: "no-tokio-spawn",
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

impl<'a> Visit<'a> for TokioSpawnVisitor<'a> {
	fn visit_expr_call(&mut self, node: &'a ExprCall) {
		if let Expr::Path(ExprPath { path, .. }) = &*node.func
			&& let Some(variant) = self.is_tokio_spawn_path(path)
		{
			self.report_tokio_spawn(node.func.span(), variant);
		}
		syn::visit::visit_expr_call(self, node);
	}
}
