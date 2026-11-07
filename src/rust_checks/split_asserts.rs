//! `assert!(a && b)` -> `assert!(a); assert!(b);`, so a failure points at the exact condition.

use std::path::Path;

use syn::{BinOp, Expr, ExprMacro, Macro, MacroDelimiter, StmtMacro, Token, punctuated::Punctuated, spanned::Spanned, visit::Visit};

use super::{Fix, Violation, path_rewrite::span_to_byte, skip::SkipVisitor};

const RULE: &str = "split-asserts";
pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let visitor = SplitAssertsVisitor {
		path_str: path.display().to_string(),
		content,
		violations: Vec::default(),
	};
	let mut skip_visitor = SkipVisitor::for_rule(visitor, content, RULE);
	skip_visitor.visit_file(file);
	skip_visitor.inner.violations
}

struct SplitAssertsVisitor<'a> {
	path_str: String,
	content: &'a str,
	violations: Vec<Violation>,
}

impl SplitAssertsVisitor<'_> {
	/// `fix_end` is where the statement ends; `None` for an assert in expression position, which can't become several statements
	fn check_macro(&mut self, mac: &Macro, fix_end: Option<usize>) {
		let Some(name) = mac.path.segments.last().map(|s| s.ident.to_string()) else { return };
		if !matches!(name.as_str(), "assert" | "debug_assert") {
			return;
		}
		let Ok(args) = mac.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated) else {
			return; // not valid assert syntax, rustc reports it
		};
		let Some(cond) = args.first() else { return };
		let mut conjuncts = Vec::default();
		collect_conjuncts(cond, &mut conjuncts);
		if conjuncts.len() < 2 {
			return;
		}

		let start = mac.path.span().start();
		self.violations.push(Violation {
			rule: RULE,
			file: self.path_str.clone(),
			line: start.line,
			column: start.column,
			message: format!("`{name}!` over `&&`: split into one `{name}!` per condition, so a failure points at the exact one"),
			fix: fix_end.and_then(|end| self.split_fix(mac, cond, &conjuncts, end)),
		});
	}

	fn split_fix(&self, mac: &Macro, cond: &Expr, conjuncts: &[&Expr], end: usize) -> Option<Fix> {
		let content = self.content;
		let byte = |pos| span_to_byte(content, pos);
		let start = byte(mac.path.span().start())?;
		let delim_span = match &mac.delimiter {
			MacroDelimiter::Paren(d) => d.span,
			MacroDelimiter::Brace(d) => d.span,
			MacroDelimiter::Bracket(d) => d.span,
		};
		let head = &content[start..byte(delim_span.open().end())?];
		let cond_end = byte(cond.span().end())?;
		let close_start = byte(delim_span.close().start())?;
		let rest = &content[cond_end..close_start]; // `, "msg", args` or empty
		let tail = &content[close_start..byte(delim_span.close().end())?];

		let line_start = content[..start].rfind('\n').map_or(0, |i| i + 1);
		let indent: String = content[line_start..start].chars().take_while(|c| c.is_whitespace()).collect();

		let parts = conjuncts
			.iter()
			.map(|c| Some(format!("{head}{}{rest}{tail};", &content[byte(c.span().start())?..byte(c.span().end())?])))
			.collect::<Option<Vec<_>>>()?;
		Some(Fix {
			start_byte: start,
			end_byte: end,
			replacement: parts.join(&format!("\n{indent}")),
		})
	}
}

impl<'a> Visit<'a> for SplitAssertsVisitor<'_> {
	fn visit_stmt_macro(&mut self, node: &'a StmtMacro) {
		let end = match &node.semi_token {
			Some(semi) => semi.span.end(),
			None => node.mac.span().end(),
		};
		let fix_end = if node.attrs.is_empty() { span_to_byte(self.content, end) } else { None }; // attrs would only stay on the first half
		self.check_macro(&node.mac, fix_end);
	}

	fn visit_expr_macro(&mut self, node: &'a ExprMacro) {
		self.check_macro(&node.mac, None);
	}
}

fn collect_conjuncts<'e>(expr: &'e Expr, out: &mut Vec<&'e Expr>) {
	match expr {
		Expr::Binary(bin) if matches!(bin.op, BinOp::And(_)) => {
			collect_conjuncts(&bin.left, out);
			collect_conjuncts(&bin.right, out);
		}
		Expr::Paren(paren) => collect_conjuncts(&paren.expr, out),
		_ => out.push(expr),
	}
}
