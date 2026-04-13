use crate::utils::{assert_check_passing, opts_for, test_case};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("inline_default")
}

// === Passing cases ===

#[test]
fn derive_default_passes() {
	assert_check_passing(
		r#"
		#[derive(Default)]
		struct Yak {
			foo: i32,
		}
		"#,
		&opts(),
	);
}

#[test]
fn no_default_impl_passes() {
	assert_check_passing(
		r#"
		struct Yak {
			foo: i32,
		}
		impl Yak {
			fn something(&self) -> i32 {
				self.foo
			}
		}
		"#,
		&opts(),
	);
}

#[test]
fn complex_default_body_passes() {
	// Body has more than just the struct literal → skip
	assert_check_passing(
		r#"
		struct Yak {
			foo: i32,
		}
		impl Default for Yak {
			fn default() -> Self {
				let yak = Self { foo: 42 };
				assert!(yak.foo > 0);
				yak
			}
		}
		"#,
		&opts(),
	);
}

#[test]
fn spread_syntax_passes() {
	// ..Default::default() spread → skip
	assert_check_passing(
		r#"
		struct Yak {
			foo: i32,
			bar: i32,
		}
		impl Default for Yak {
			fn default() -> Self {
				Self { foo: 1, ..Default::default() }
			}
		}
		"#,
		&opts(),
	);
}

#[test]
fn generic_struct_passes() {
	// Generic structs are skipped for safety
	assert_check_passing(
		r#"
		struct Yak<T> {
			foo: T,
		}
		impl<T: Default> Default for Yak<T> {
			fn default() -> Self {
				Self { foo: T::default() }
			}
		}
		"#,
		&opts(),
	);
}

#[test]
fn let_binding_in_body_passes() {
	assert_check_passing(
		r#"
		struct Yak {
			foo: i32,
		}
		impl Default for Yak {
			fn default() -> Self {
				let x = 42;
				Self { foo: x }
			}
		}
		"#,
		&opts(),
	);
}

// === Passing: new() calls are not const, so we skip these ===

#[test]
fn new_call_in_field_passes() {
	// Bar::new(2.0) is not const — skip
	assert_check_passing(
		r#"
		struct Yak {
			foo: Bar,
		}
		impl Default for Yak {
			fn default() -> Self {
				Self {
					foo: Bar::new(2.0),
				}
			}
		}
		"#,
		&opts(),
	);
}

#[test]
fn string_new_in_field_passes() {
	// String::new() is not const — skip even if other fields are plain literals
	assert_check_passing(
		r#"
		struct Yak {
			foo: i32,
			bar: String,
		}
		impl Default for Yak {
			fn default() -> Self {
				Self {
					foo: 0,
					bar: String::new(),
				}
			}
		}
		"#,
		&opts(),
	);
}

#[test]
fn arbitrary_fn_call_in_field_passes() {
	// rand::make_rng() is not a ::new() call but still not const — skip
	assert_check_passing(
		r#"
		struct RandomPlayer {
			params: Random,
			rng: SmallRng,
		}
		impl Default for RandomPlayer {
			fn default() -> Self {
				Self {
					params: Random {},
					rng: rand::make_rng(),
				}
			}
		}
		"#,
		&opts(),
	);
}

#[test]
fn method_call_in_field_passes() {
	// "Player".to_string() is a method call — skip
	assert_check_passing(
		r#"
		#[derive(Clone, Debug, Eq, PartialEq)]
		pub struct ManualPlayer {
			pub name: String,
		}
		impl Default for ManualPlayer {
			fn default() -> Self {
				Self { name: "Player".to_string() }
			}
		}
		"#,
		&opts(),
	);
}

// === Violation + fix cases ===

#[test]
fn non_adjacent_impl_between_struct_and_default() {
	// impl Rating { … } sits between struct and impl Default — fix must still work
	insta::assert_snapshot!(test_case(
		r#"
		struct Rating {
			rating: f64,
			deviation: f64,
		}
		impl Rating {
			pub fn is_provisional(&self) -> bool {
				self.deviation >= 110.0
			}
		}
		impl Default for Rating {
			fn default() -> Self {
				Self { rating: 1500.0, deviation: 350.0 }
			}
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[inline-default] /main.rs:1: `impl Default for Rating` can be inlined as field defaults (RFC 3681)

	# Format mode
	struct Rating {
		rating: f64 = 1500.0,
		deviation: f64 = 350.0,
	}
	impl Rating {
		pub fn is_provisional(&self) -> bool {
			self.deviation >= 110.0
		}
	}
	");
}

#[test]
fn single_field() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Yak {
			foo: i32,
		}
		impl Default for Yak {
			fn default() -> Self {
				Self { foo: 42 }
			}
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[inline-default] /main.rs:1: `impl Default for Yak` can be inlined as field defaults (RFC 3681)

	# Format mode
	struct Yak {
		foo: i32 = 42,
	}
	");
}

#[test]
fn multiple_const_fields() {
	// All fields are plain literals — safe to inline
	insta::assert_snapshot!(test_case(
		r#"
		struct Yak {
			foo: i32,
			bar: bool,
		}
		impl Default for Yak {
			fn default() -> Self {
				Self {
					foo: 0,
					bar: false,
				}
			}
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[inline-default] /main.rs:1: `impl Default for Yak` can be inlined as field defaults (RFC 3681)

	# Format mode
	struct Yak {
		foo: i32 = 0,
		bar: bool = false,
	}
	");
}
