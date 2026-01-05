use codestyle::rust_checks::RustCheckOptions;

pub fn opts_for(check: &str) -> RustCheckOptions {
	RustCheckOptions {
		instrument: check == "instrument",
		join_split_impls: check == "join_split_impls",
		impl_follows_type: check == "impl_follows_type",
		loops: check == "loops",
		embed_simple_vars: check == "embed_simple_vars",
		insta_inline_snapshot: check == "insta_inline_snapshot",
		no_chrono: check == "no_chrono",
		no_tokio_spawn: check == "no_tokio_spawn",
	}
}
