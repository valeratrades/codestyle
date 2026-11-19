//! Lint for unconventional uses of `fn new` / `Type::new()`.
//!
//! `fn new` returning `Result<…>` in an inherent impl — rename to `try_new`
//! so fallibility is visible at the call-site. Autofixed by renaming the
//! function identifier. Call-sites `TypeName::new(…)` are also renamed to
//! `TypeName::try_new(…)` using the set of known types collected project-wide.

use std::{collections::HashSet, path::Path};

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use syn::{ExprPath, ImplItem, PathSegment, ReturnType, Type, visit::Visit};

use super::{FileInfo, Fix, Violation, path_rewrite::span_to_byte, skip::SkipVisitor};

const RULE: &str = "unconventional-new";

/// Collect all type names that have `fn new(...) -> Result<...>` in inherent impls, at any
/// nesting depth, across the given set of parsed files. Used to find call-sites project-wide.
pub fn collect_try_new_types(file_infos: &[FileInfo]) -> HashSet<String> {
	struct Collector(HashSet<String>);
	impl<'a> Visit<'a> for Collector {
		fn visit_item_impl(&mut self, node: &'a syn::ItemImpl) {
			if node.trait_.is_none()
				&& let Some(type_name) = impl_type_name(node)
				&& node
					.items
					.iter()
					.any(|item| matches!(item, ImplItem::Fn(func) if func.sig.ident == "new" && returns_result(&func.sig.output)))
			{
				self.0.insert(type_name);
			}
			syn::visit::visit_item_impl(self, node);
		}
	}
	let mut collector = Collector(HashSet::default());
	for info in file_infos {
		if let Some(ref tree) = info.syntax_tree {
			collector.visit_file(tree);
		}
	}
	collector.0
}

pub fn check(path: &Path, content: &str, file: &syn::File, try_new_types: &HashSet<String>) -> Vec<Violation> {
	let visitor = UnconventionalNewVisitor::new(path, content, try_new_types);
	let mut skip_visitor = SkipVisitor::for_rule(visitor, content, RULE);
	skip_visitor.visit_file(file);
	skip_visitor.inner.violations
}

struct UnconventionalNewVisitor<'a> {
	path_str: String,
	content: &'a str,
	try_new_types: &'a HashSet<String>,
	/// Type `Self` names in the impl currently being visited
	self_ty: Option<String>,
	violations: Vec<Violation>,
}

impl<'a> UnconventionalNewVisitor<'a> {
	fn new(path: &Path, content: &'a str, try_new_types: &'a HashSet<String>) -> Self {
		Self {
			path_str: path.display().to_string(),
			content,
			try_new_types,
			self_ty: None,
			violations: Vec::default(),
		}
	}

	fn check_impl_fn(&mut self, func: &syn::ImplItemFn) {
		if func.sig.ident != "new" {
			return;
		}
		if !returns_result(&func.sig.output) {
			return;
		}
		let span = func.sig.ident.span();
		let fix = span_to_byte(self.content, span.start()).and_then(|start| {
			span_to_byte(self.content, span.end()).map(|end| Fix {
				start_byte: start,
				end_byte: end,
				replacement: "try_new".to_string(),
			})
		});
		self.violations.push(Violation {
			rule: RULE,
			hint: None,
			file: self.path_str.clone(),
			line: span.start().line,
			column: span.start().column,
			message: "`fn new` returns `Result` — rename to `try_new` to signal fallibility".to_string(),
			fix,
		});
	}

	/// syn leaves macro arguments unparsed, so `Type::new(` call-sites in them are found on the raw token tree.
	fn check_macro_tokens(&mut self, tokens: TokenStream) {
		let tts: Vec<TokenTree> = tokens.into_iter().collect();
		for (i, tt) in tts.iter().enumerate() {
			if let TokenTree::Group(g) = tt {
				self.check_macro_tokens(g.stream());
			}
			let [TokenTree::Ident(ty), TokenTree::Punct(c1), TokenTree::Punct(c2), TokenTree::Ident(new), TokenTree::Group(args)] = &tts[i..(i + 5).min(tts.len())] else {
				continue;
			};
			if c1.as_char() != ':' || c2.as_char() != ':' || new != "new" || args.delimiter() != Delimiter::Parenthesis {
				continue;
			}
			let Some(type_name) = self.resolve(&ty.to_string()) else { continue };
			self.push_call_violation(&type_name, new.span());
		}
	}

	fn push_call_violation(&mut self, type_name: &str, span: proc_macro2::Span) {
		let fix = span_to_byte(self.content, span.start()).and_then(|start| {
			span_to_byte(self.content, span.end()).map(|end| Fix {
				start_byte: start,
				end_byte: end,
				replacement: "try_new".to_string(),
			})
		});
		self.violations.push(Violation {
			rule: RULE,
			hint: None,
			file: self.path_str.clone(),
			line: span.start().line,
			column: span.start().column,
			message: format!("`{type_name}::new` was renamed to `try_new`"),
			fix,
		});
	}

	/// The type a `<name>::new` path segment names, if its `new` is fallible.
	fn resolve(&self, name: &str) -> Option<String> {
		let type_name = match name {
			"Self" => self.self_ty.clone()?,
			_ => name.to_string(),
		};
		self.try_new_types.contains(&type_name).then_some(type_name)
	}

	/// Calls and fn references alike: `Foo::new(x)`, `.map(Foo::new)`.
	fn check_path(&mut self, path: &syn::Path) {
		let [.., ty, new] = path.segments.iter().collect::<Vec<_>>()[..] else {
			return;
		};
		if new.ident != "new" {
			return;
		}
		let Some(type_name) = self.resolve(&ty.ident.to_string()) else { return };
		self.push_call_violation(&type_name, new.ident.span());
	}
}

impl<'a> Visit<'a> for UnconventionalNewVisitor<'a> {
	fn visit_item_impl(&mut self, node: &'a syn::ItemImpl) {
		if node.trait_.is_none() {
			for item in &node.items {
				if let ImplItem::Fn(func) = item {
					self.check_impl_fn(func);
				}
			}
		}
		let outer = std::mem::replace(&mut self.self_ty, impl_type_name(node));
		syn::visit::visit_item_impl(self, node);
		self.self_ty = outer;
	}

	fn visit_macro(&mut self, node: &'a syn::Macro) {
		self.check_macro_tokens(node.tokens.clone());
		syn::visit::visit_macro(self, node);
	}

	fn visit_expr_path(&mut self, node: &'a ExprPath) {
		self.check_path(&node.path);
		syn::visit::visit_expr_path(self, node);
	}
}

fn impl_type_name(node: &syn::ItemImpl) -> Option<String> {
	let Type::Path(self_ty) = node.self_ty.as_ref() else { return None };
	self_ty.path.segments.last().map(|seg| seg.ident.to_string())
}

fn returns_result(output: &ReturnType) -> bool {
	let ReturnType::Type(_, ty) = output else {
		return false;
	};
	let Type::Path(type_path) = ty.as_ref() else {
		return false;
	};
	type_path.path.segments.last().is_some_and(|seg: &PathSegment| seg.ident == "Result")
}
