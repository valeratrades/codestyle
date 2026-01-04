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

	/// Check that impl blocks follow type definitions [default: true]
	#[arg(long)]
	impl_follows_type: Option<bool>,

	/// Check for simple vars that should be embedded in format strings [default: true]
	#[arg(long)]
	embed_simple_vars: Option<bool>,

	/// Check that insta snapshots use inline @"" syntax [default: true]
	#[arg(long)]
	insta_inline_snapshot: Option<bool>,
}

impl From<RustCheckOptionsArgs> for RustCheckOptions {
	fn from(args: RustCheckOptionsArgs) -> Self {
		let defaults = RustCheckOptions::default();
		Self {
			instrument: args.instrument.unwrap_or(defaults.instrument),
			loops: args.loops.unwrap_or(defaults.loops),
			impl_follows_type: args.impl_follows_type.unwrap_or(defaults.impl_follows_type),
			embed_simple_vars: args.embed_simple_vars.unwrap_or(defaults.embed_simple_vars),
			insta_inline_snapshot: args.insta_inline_snapshot.unwrap_or(defaults.insta_inline_snapshot),
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
