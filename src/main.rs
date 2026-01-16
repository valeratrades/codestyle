use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

mod rust_checks;

use rust_checks::RustCheckOptions;

#[derive(Parser)]
#[command(author, version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")"), about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Run Rust code style checks
	Rust {
		#[command(subcommand)]
		mode: RustMode,

		#[command(flatten)]
		options: RustCheckOptionsArgs,
	},
}

#[derive(Subcommand)]
enum RustMode {
	/// Check for violations and exit 1 on failure
	Assert {
		/// Target directory to check
		target_dir: PathBuf,
	},
	/// Attempt to fix violations automatically
	Format {
		/// Target directory to check
		target_dir: PathBuf,
	},
}

#[derive(Args)]
struct RustCheckOptionsArgs {
	/// Check for #[instrument] on async functions [default: false]
	#[arg(long)]
	instrument: Option<bool>,

	/// Check for //LOOP comment on endless loops [default: true]
	#[arg(long)]
	loops: Option<bool>,

	/// Join split impl blocks for the same type [default: true]
	#[arg(long)]
	join_split_impls: Option<bool>,

	/// Wrap impl blocks with vim 1-fold markers [default: false]
	#[arg(long)]
	impl_folds: Option<bool>,

	/// Check that impl blocks follow type definitions [default: true]
	#[arg(long)]
	impl_follows_type: Option<bool>,

	/// Check for simple vars that should be embedded in format strings [default: true]
	#[arg(long)]
	embed_simple_vars: Option<bool>,

	/// Check that insta snapshots use inline @"" syntax [default: true]
	#[arg(long)]
	insta_inline_snapshot: Option<bool>,

	/// Disallow usage of chrono crate (use jiff instead) [default: true]
	#[arg(long)]
	no_chrono: Option<bool>,

	/// Disallow usage of tokio::spawn [default: true]
	#[arg(long)]
	no_tokio_spawn: Option<bool>,

	/// Replace `return Err(eyre!(...))` with `bail!(...)` [default: true]
	#[arg(long)]
	use_bail: Option<bool>,

	/// Check that test functions don't have redundant `test_` prefix [default: false]
	#[arg(long)]
	test_fn_prefix: Option<bool>,
}

impl From<RustCheckOptionsArgs> for RustCheckOptions {
	fn from(args: RustCheckOptionsArgs) -> Self {
		let defaults = RustCheckOptions::default();
		Self {
			instrument: args.instrument.unwrap_or(defaults.instrument),
			loops: args.loops.unwrap_or(defaults.loops),
			join_split_impls: args.join_split_impls.unwrap_or(defaults.join_split_impls),
			impl_folds: args.impl_folds.unwrap_or(defaults.impl_folds),
			impl_follows_type: args.impl_follows_type.unwrap_or(defaults.impl_follows_type),
			embed_simple_vars: args.embed_simple_vars.unwrap_or(defaults.embed_simple_vars),
			insta_inline_snapshot: args.insta_inline_snapshot.unwrap_or(defaults.insta_inline_snapshot),
			no_chrono: args.no_chrono.unwrap_or(defaults.no_chrono),
			no_tokio_spawn: args.no_tokio_spawn.unwrap_or(defaults.no_tokio_spawn),
			use_bail: args.use_bail.unwrap_or(defaults.use_bail),
			test_fn_prefix: args.test_fn_prefix.unwrap_or(defaults.test_fn_prefix),
		}
	}
}

fn main() {
	v_utils::clientside!();
	let cli = Cli::parse();

	let exit_code = match cli.command {
		Commands::Rust { mode, options } => {
			let opts: RustCheckOptions = options.into();
			match mode {
				RustMode::Assert { target_dir } => rust_checks::run_assert(&target_dir, &opts),
				RustMode::Format { target_dir } => rust_checks::run_format(&target_dir, &opts),
			}
		}
	};

	std::process::exit(exit_code);
}
