mod instrument;
mod loops;

use std::{
	fs,
	path::{Path, PathBuf},
};

pub use instrument::check_instrument;
pub use loops::check_loops;
use syn::{ItemFn, parse_file};
use walkdir::WalkDir;

#[derive(Clone, Default, derive_new::new)]
pub struct FileInfo {
	pub contents: String,
	pub fn_items: Vec<ItemFn>,
	pub path: PathBuf,
}

pub fn collect_rust_files(target_dir: &Path) -> Vec<FileInfo> {
	let mut file_infos = Vec::new();
	for entry in WalkDir::new(target_dir).into_iter().filter_map(Result::ok) {
		let path = entry.path().to_path_buf();
		if path.components().any(|comp| comp == std::path::Component::Normal("target".as_ref())) {
			continue;
		}
		if path.extension().is_some_and(|ext| ext == "rs") {
			if let Some(info) = parse_rust_file(path) {
				file_infos.push(info);
			}
		}
	}
	file_infos
}

fn parse_rust_file(path: PathBuf) -> Option<FileInfo> {
	let contents = fs::read_to_string(&path).ok()?;
	let syntax_tree = match parse_file(&contents) {
		Ok(tree) => tree,
		Err(e) => {
			eprintln!("Failed to parse file {:?}: {}", path, e);
			return None;
		}
	};

	let fn_items = syntax_tree
		.items
		.iter()
		.filter_map(|item| if let syn::Item::Fn(func) = item { Some(func.clone()) } else { None })
		.collect();

	Some(FileInfo { contents, fn_items, path })
}
