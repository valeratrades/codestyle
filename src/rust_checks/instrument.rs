use syn::ItemFn;

use super::{FileInfo, Violation};

pub fn check_instrument(file_info: &FileInfo) -> Vec<Violation> {
	let mut violations = Vec::new();
	let filename = file_info.path.file_name().and_then(|f| f.to_str()).unwrap_or("");
	let path_str = file_info.path.display().to_string();

	for func in &file_info.fn_items {
		// Only check async functions
		if func.sig.asyncness.is_none() {
			continue;
		}
		if has_instrument_attr(func) {
			continue;
		}
		if filename == "utils.rs" || func.sig.ident == "main" {
			continue;
		}

		let span_start = func.sig.ident.span().start();
		violations.push(Violation {
			rule: "instrument",
			file: path_str.clone(),
			line: span_start.line,
			column: span_start.column,
			message: format!("No #[instrument] on async fn `{}`", func.sig.ident),
			fix: None,
		});
	}
	violations
}

fn has_instrument_attr(func: &ItemFn) -> bool {
	func.attrs.iter().any(|attr| attr.path().is_ident("instrument"))
}
