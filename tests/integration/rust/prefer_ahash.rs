use crate::utils::{assert_check_passing, opts_for, test_case};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("prefer_ahash")
}

// === Passing cases ===

#[test]
fn ahash_map_passes() {
	assert_check_passing(
		r#"
		use ahash::AHashMap;

		fn main() {
			let _map: AHashMap<String, u32> = AHashMap::new();
		}
		"#,
		&opts(),
	);
}

#[test]
fn unrelated_hashmap_name_passes() {
	assert_check_passing(
		r#"
		struct MyHashMap;

		fn main() {
			let _x = MyHashMap;
		}
		"#,
		&opts(),
	);
}

// === Violation cases (with autofix) ===

#[test]
fn simple_use_hashmap() {
	insta::assert_snapshot!(test_case(
		r#"
		use std::collections::HashMap;

		fn main() {
			let _map: HashMap<String, u32> = HashMap::new();
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[prefer-ahash] /main.rs:1: use `ahash::AHashMap` instead of `std::collections::HashMap`
	[prefer-ahash] /main.rs:4: use `AHashMap` instead of `HashMap`
	[prefer-ahash] /main.rs:4: use `AHashMap` instead of `HashMap`

	# Format mode
	use ahash::AHashMap;

	fn main() {
		let _map: AHashMap<String, u32> = AHashMap::new();
	}
	");
}

#[test]
fn group_use_hashmap() {
	insta::assert_snapshot!(test_case(
		r#"
		use std::collections::{BTreeMap, HashMap};

		fn main() {
			let _map: HashMap<String, u32> = HashMap::new();
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[prefer-ahash] /main.rs:1: use `ahash::AHashMap` instead of `std::collections::HashMap`
	[prefer-ahash] /main.rs:4: use `AHashMap` instead of `HashMap`
	[prefer-ahash] /main.rs:4: use `AHashMap` instead of `HashMap`

	# Format mode
	use std::collections::{BTreeMap};
	use ahash::AHashMap;

	fn main() {
		let _map: AHashMap<String, u32> = AHashMap::new();
	}
	");
}

#[test]
fn fully_qualified_type_path() {
	insta::assert_snapshot!(test_case(
		r#"
		fn make_map() -> std::collections::HashMap<String, u32> {
			std::collections::HashMap::new()
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[prefer-ahash] /main.rs:1: use `AHashMap` instead of `HashMap`
	[prefer-ahash] /main.rs:2: use `AHashMap` instead of `HashMap`

	# Format mode
	fn make_map() -> AHashMap<String, u32> {
		AHashMap::new()
	}
	");
}
