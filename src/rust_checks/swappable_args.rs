//! Two parameters of the same primitive type compile silently when swapped at the call site.

use std::{collections::BTreeMap, path::Path};

use syn::{FnArg, ImplItemFn, ItemFn, ItemImpl, ItemMod, Signature, TraitItemFn, Type, spanned::Spanned, visit::Visit};

use super::{
	Violation,
	no_tokio_spawn::{is_cfg_test_attr, is_test_attr},
	skip::{SkipVisitor, has_skip_marker_for_rule},
};

const RULE: &str = "swappable-args";
const PRIMITIVES: &[&str] = &[
	"u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32", "f64", "bool", "char", "str", "String",
];
pub fn check(path: &Path, content: &str, file: &syn::File) -> Vec<Violation> {
	let visitor = SwappableArgsVisitor {
		path_str: path.display().to_string(),
		content,
		test_depth: 0,
		in_trait_impl: false,
		violations: Vec::default(),
	};
	let mut skip_visitor = SkipVisitor::for_rule(visitor, content, RULE);
	skip_visitor.visit_file(file);
	skip_visitor.inner.violations
}

struct SwappableArgsVisitor<'a> {
	content: &'a str,
	path_str: String,
	test_depth: usize,
	in_trait_impl: bool,
	violations: Vec<Violation>,
}

impl SwappableArgsVisitor<'_> {
	fn check_sig(&mut self, sig: &Signature) {
		if self.test_depth > 0 {
			return;
		}
		let mut by_type: BTreeMap<String, usize> = BTreeMap::default();
		for arg in &sig.inputs {
			if let FnArg::Typed(pat) = arg
				&& let Some(key) = primitive_key(&pat.ty)
			{
				*by_type.entry(key).or_default() += 1;
			}
		}
		let repeated: Vec<String> = by_type.into_iter().filter(|(_, n)| *n > 1).map(|(ty, n)| format!("{n} x `{ty}`")).collect();
		if repeated.is_empty() {
			return;
		}
		let start = sig.ident.span().start();
		self.violations.push(Violation {
			rule: RULE,
			file: self.path_str.clone(),
			line: start.line,
			column: start.column,
			message: format!(
				"`{}` takes interchangeable parameters ({})\nHINT: wrap them in newtypes, or pass an args struct",
				sig.ident,
				repeated.join(", ")
			),
			fix: None,
		});
	}
}

impl<'a> Visit<'a> for SwappableArgsVisitor<'_> {
	fn visit_item_fn(&mut self, node: &'a ItemFn) {
		let is_test = node.attrs.iter().any(is_test_attr);
		self.test_depth += usize::from(is_test);
		self.check_sig(&node.sig);
		syn::visit::visit_item_fn(self, node);
		self.test_depth -= usize::from(is_test);
	}

	fn visit_item_mod(&mut self, node: &'a ItemMod) {
		let is_test = node.attrs.iter().any(is_cfg_test_attr);
		self.test_depth += usize::from(is_test);
		syn::visit::visit_item_mod(self, node);
		self.test_depth -= usize::from(is_test);
	}

	fn visit_item_impl(&mut self, node: &'a ItemImpl) {
		let is_test = node.attrs.iter().any(is_cfg_test_attr);
		self.test_depth += usize::from(is_test);
		let outer = std::mem::replace(&mut self.in_trait_impl, node.trait_.is_some());
		syn::visit::visit_item_impl(self, node);
		self.in_trait_impl = outer;
		self.test_depth -= usize::from(is_test);
	}

	fn visit_impl_item_fn(&mut self, node: &'a ImplItemFn) {
		let is_test = node.attrs.iter().any(is_test_attr);
		self.test_depth += usize::from(is_test);
		if !is_test && !self.in_trait_impl && !has_skip_marker_for_rule(self.content, node.span(), RULE) {
			self.check_sig(&node.sig);
		}
		syn::visit::visit_impl_item_fn(self, node);
		self.test_depth -= usize::from(is_test);
	}

	fn visit_trait_item_fn(&mut self, node: &'a TraitItemFn) {
		if !has_skip_marker_for_rule(self.content, node.span(), RULE) {
			self.check_sig(&node.sig);
		}
		syn::visit::visit_trait_item_fn(self, node);
	}
}

/// `u64`, `&str`, `&mut f32`, ... as a comparable string; `None` for anything else
fn primitive_key(ty: &Type) -> Option<String> {
	match ty {
		Type::Reference(r) => Some(format!("&{}{}", if r.mutability.is_some() { "mut " } else { "" }, primitive_key(&r.elem)?)),
		Type::Path(p) if p.qself.is_none() => {
			let ident = p.path.get_ident()?.to_string();
			PRIMITIVES.contains(&ident.as_str()).then_some(ident)
		}
		_ => None,
	}
}
