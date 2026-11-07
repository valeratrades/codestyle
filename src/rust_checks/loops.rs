use syn::{Expr, Stmt, spanned::Spanned};

use super::{
	FileInfo, Violation,
	skip::{has_marker_comment, has_skip_marker_for_rule},
};

const RULE: &str = "loop-comment";
pub fn check_loops(file_info: &FileInfo) -> Vec<Violation> {
	let mut violations = Vec::default();
	let path_str = file_info.path.display().to_string();

	for func in &file_info.fn_items {
		if has_skip_marker_for_rule(&file_info.contents, func.span(), RULE) {
			continue;
		}
		collect_loop_issues_from_stmts(&func.block.stmts, &file_info.contents, &path_str, &mut violations);
	}

	violations
}

//#[codestyle::skip(swappable-args)]
fn collect_loop_issues_from_stmts(stmts: &[Stmt], file_contents: &str, file_path: &str, violations: &mut Vec<Violation>) {
	for stmt in stmts {
		match stmt {
			Stmt::Expr(expr, _) => {
				check_expr_for_loops(expr, file_contents, file_path, violations);
			}
			Stmt::Local(local) =>
				if let Some(init) = &local.init {
					check_expr_for_loops(&init.expr, file_contents, file_path, violations);
				},
			_ => {}
		}
	}
}

//#[codestyle::skip(swappable-args)]
fn check_expr_for_loops(expr: &Expr, file_contents: &str, file_path: &str, violations: &mut Vec<Violation>) {
	match expr {
		Expr::Loop(loop_expr) => {
			let span_start = loop_expr.loop_token.span().start();
			if !has_marker_comment(file_contents, span_start.line, "LOOP") {
				violations.push(Violation {
					rule: RULE,
					file: file_path.to_string(),
					line: span_start.line,
					column: span_start.column,
					message: "Endless loop without `//LOOP` comment\nHINT: try to rewrite the loop with `while let` or justify why a bound can't be enforced".to_string(),
					fix: None,
				});
			}
			collect_loop_issues_from_stmts(&loop_expr.body.stmts, file_contents, file_path, violations);
		}
		Expr::Block(block) => {
			collect_loop_issues_from_stmts(&block.block.stmts, file_contents, file_path, violations);
		}
		Expr::If(if_expr) => {
			collect_loop_issues_from_stmts(&if_expr.then_branch.stmts, file_contents, file_path, violations);
			if let Some((_, else_branch)) = &if_expr.else_branch {
				check_expr_for_loops(else_branch, file_contents, file_path, violations);
			}
		}
		Expr::Match(match_expr) =>
			for arm in &match_expr.arms {
				check_expr_for_loops(&arm.body, file_contents, file_path, violations);
			},
		Expr::While(while_expr) => {
			collect_loop_issues_from_stmts(&while_expr.body.stmts, file_contents, file_path, violations);
		}
		Expr::ForLoop(for_expr) => {
			collect_loop_issues_from_stmts(&for_expr.body.stmts, file_contents, file_path, violations);
		}
		Expr::Async(async_expr) => {
			collect_loop_issues_from_stmts(&async_expr.block.stmts, file_contents, file_path, violations);
		}
		Expr::Unsafe(unsafe_expr) => {
			collect_loop_issues_from_stmts(&unsafe_expr.block.stmts, file_contents, file_path, violations);
		}
		Expr::Closure(closure) => {
			check_expr_for_loops(&closure.body, file_contents, file_path, violations);
		}
		_ => {}
	}
}
