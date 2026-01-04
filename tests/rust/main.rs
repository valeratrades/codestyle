#[test]
fn rust_checks() {
	let t = trybuild::TestCases::new();
	t.pass("tests/rust/loops.rs");
	t.pass("tests/rust/instrument.rs");
	t.pass("tests/rust/join_split_impls.rs");
	t.pass("tests/rust/impl_follows_type.rs");
	t.pass("tests/rust/embed_simple_vars.rs");
	t.pass("tests/rust/insta_snapshots.rs");
}
