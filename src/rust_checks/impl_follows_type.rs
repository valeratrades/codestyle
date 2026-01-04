use std::{collections::HashMap, path::Path};

use syn::{Item, ItemEnum, ItemStruct, ItemUnion, spanned::Spanned};

use super::Violation;

struct TypeDef {
	end_line: usize,
}

pub fn check(path: &Path, file: &syn::File) -> Vec<Violation> {
	const RULE: &str = "impl-follows-type";

	let path_str = path.display().to_string();
	let mut type_defs: HashMap<String, TypeDef> = HashMap::new();
	let mut violations = Vec::new();

	for item in &file.items {
		let (name, end_line) = match item {
			Item::Struct(ItemStruct { ident, .. }) => (ident.to_string(), item.span().end().line),
			Item::Enum(ItemEnum { ident, .. }) => (ident.to_string(), item.span().end().line),
			Item::Union(ItemUnion { ident, .. }) => (ident.to_string(), item.span().end().line),
			_ => continue,
		};

		type_defs.insert(name, TypeDef { end_line });
	}

	for item in &file.items {
		let Item::Impl(impl_block) = item else {
			continue;
		};

		let type_name = match &*impl_block.self_ty {
			syn::Type::Path(type_path) => type_path.path.segments.last().map(|s| s.ident.to_string()),
			_ => None,
		};

		let Some(type_name) = type_name else {
			continue;
		};

		// Skip trait impls
		if impl_block.trait_.is_some() {
			continue;
		}

		let Some(type_def) = type_defs.get(&type_name) else {
			continue;
		};

		let impl_start_line = impl_block.span().start().line;
		let expected_line = type_def.end_line + 1;

		if impl_start_line > expected_line + 1 {
			let gap = impl_start_line - type_def.end_line - 1;
			violations.push(Violation {
				rule: RULE,
				file: path_str.clone(),
				line: impl_start_line,
				column: impl_block.span().start().column,
				message: format!("`impl {type_name}` should follow type definition (line {}), but has {gap} blank line(s)", type_def.end_line),
				fix: None,
			});
		}

		// Update type_def to point to end of this impl block for chained impls
		type_defs.insert(
			type_name,
			TypeDef {
				end_line: impl_block.span().end().line,
			},
		);
	}

	violations
}
