//! Unbounded queues turn backpressure into tail-latency spikes and unbounded memory.

use std::path::Path;

use syn::{Expr, ExprCall, spanned::Spanned, visit::Visit};

use super::{
	Violation,
	skip::{SkipVisitor, has_marker_comment},
};

const RULE: &str = "no-unbounded-channels";
pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let visitor = UnboundedVisitor {
		path_str: path.display().to_string(),
		content,
		violations: Vec::default(),
	};
	let mut skip_visitor = SkipVisitor::for_rule(visitor, content, RULE);
	skip_visitor.visit_file(file);
	skip_visitor.inner.violations
}

struct UnboundedVisitor<'a> {
	path_str: String,
	content: &'a str,
	violations: Vec<Violation>,
}

impl<'a> Visit<'a> for UnboundedVisitor<'_> {
	fn visit_expr_call(&mut self, node: &'a ExprCall) {
		if let Expr::Path(func) = &*node.func {
			let segments: Vec<String> = func.path.segments.iter().map(|s| s.ident.to_string()).collect();
			let segments: Vec<&str> = segments.iter().map(String::as_str).collect();
			let unbounded = match segments.as_slice() {
				[.., "unbounded" | "unbounded_channel"] => true,
				[.., "mpsc" | "mpmc", "channel"] => node.args.is_empty(), // std's; tokio's / futures' `channel(cap)` take the bound
				_ => false,
			};
			let start = func.span().start();
			if unbounded && !has_marker_comment(self.content, start.line, "UNBOUNDED") {
				self.violations.push(Violation {
					rule: RULE,
					file: self.path_str.clone(),
					line: start.line,
					column: start.column,
					message: format!(
						"`{}` creates an unbounded queue\nHINT: bound it (`channel(cap)` / `bounded(cap)` / `sync_channel(cap)`), or justify with a `//UNBOUNDED` comment",
						segments.join("::")
					),
					fix: None,
				});
			}
		}
		syn::visit::visit_expr_call(self, node);
	}
}
