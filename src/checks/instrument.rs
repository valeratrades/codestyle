use syn::ItemFn;

use super::FileInfo;

pub fn check_instrument(file_info: &FileInfo) -> Vec<String> {
	let mut issues = Vec::new();
	let filename = file_info.path.file_name().and_then(|f| f.to_str()).unwrap_or("");

	for func in &file_info.fn_items {
		if has_instrument_attr(func) {
			continue;
		}
		if filename == "utils.rs" || func.sig.ident == "main" {
			continue;
		}

		let span_start = func.sig.ident.span().start();
		issues.push(format!(
			"No #[instrument] on `{}` in {}:{}:{}",
			func.sig.ident,
			file_info.path.display(),
			span_start.line,
			span_start.column
		));
	}
	issues
}

fn has_instrument_attr(func: &ItemFn) -> bool {
	func.attrs.iter().any(|attr| attr.path().is_ident("instrument"))
}
