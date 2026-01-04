use syn::{Expr, Stmt, spanned::Spanned};

use super::FileInfo;

pub fn check_loops(file_info: &FileInfo) -> Vec<String> {
	let mut issues = Vec::new();

	for func in &file_info.fn_items {
		collect_loop_issues_from_stmts(&func.block.stmts, &file_info.contents, &file_info.path, &mut issues);
	}

	issues
}

fn collect_loop_issues_from_stmts(stmts: &[Stmt], file_contents: &str, file_path: &std::path::Path, issues: &mut Vec<String>) {
	for stmt in stmts {
		match stmt {
			Stmt::Expr(expr, _) => {
				check_expr_for_loops(expr, file_contents, file_path, issues);
			}
			Stmt::Local(local) =>
				if let Some(init) = &local.init {
					check_expr_for_loops(&init.expr, file_contents, file_path, issues);
				},
			_ => {}
		}
	}
}

fn check_expr_for_loops(expr: &Expr, file_contents: &str, file_path: &std::path::Path, issues: &mut Vec<String>) {
	match expr {
		Expr::Loop(loop_expr) => {
			let span_start = loop_expr.loop_token.span().start();
			if !has_loop_comment(file_contents, span_start.line) {
				issues.push(format!(
					"Endless loop without `//LOOP` comment in {}:{}:{}",
					file_path.display(),
					span_start.line,
					span_start.column
				));
			}
			// Also check inside the loop body
			collect_loop_issues_from_stmts(&loop_expr.body.stmts, file_contents, file_path, issues);
		}
		Expr::Block(block) => {
			collect_loop_issues_from_stmts(&block.block.stmts, file_contents, file_path, issues);
		}
		Expr::If(if_expr) => {
			collect_loop_issues_from_stmts(&if_expr.then_branch.stmts, file_contents, file_path, issues);
			if let Some((_, else_branch)) = &if_expr.else_branch {
				check_expr_for_loops(else_branch, file_contents, file_path, issues);
			}
		}
		Expr::Match(match_expr) =>
			for arm in &match_expr.arms {
				check_expr_for_loops(&arm.body, file_contents, file_path, issues);
			},
		Expr::While(while_expr) => {
			collect_loop_issues_from_stmts(&while_expr.body.stmts, file_contents, file_path, issues);
		}
		Expr::ForLoop(for_expr) => {
			collect_loop_issues_from_stmts(&for_expr.body.stmts, file_contents, file_path, issues);
		}
		Expr::Async(async_expr) => {
			collect_loop_issues_from_stmts(&async_expr.block.stmts, file_contents, file_path, issues);
		}
		Expr::Unsafe(unsafe_expr) => {
			collect_loop_issues_from_stmts(&unsafe_expr.block.stmts, file_contents, file_path, issues);
		}
		Expr::Closure(closure) => {
			check_expr_for_loops(&closure.body, file_contents, file_path, issues);
		}
		_ => {}
	}
}

fn has_loop_comment(file_contents: &str, loop_line: usize) -> bool {
	let lines: Vec<&str> = file_contents.lines().collect();

	// Check current line (inline comment)
	if loop_line > 0 && loop_line <= lines.len() {
		let current_line = lines[loop_line - 1];
		if current_line.contains("//LOOP") || current_line.contains("// LOOP") {
			return true;
		}
	}

	// Check line above
	if loop_line > 1 {
		let prev_line = lines[loop_line - 2];
		if prev_line.contains("//LOOP") || prev_line.contains("// LOOP") {
			return true;
		}
	}

	false
}
