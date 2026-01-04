use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

mod rust_checks;

#[derive(Parser)]
#[command(author, version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")"), about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Clone, Copy, ValueEnum)]
enum Language {
	Rust,
}

#[derive(Subcommand)]
enum Commands {
	/// Check for violations and exit 1 on failure
	Assert {
		/// Language to check
		#[arg(long, value_enum)]
		rust: bool,

		/// Target directory to check
		target_dir: PathBuf,
	},
	/// Attempt to fix violations automatically
	Format {
		/// Language to check
		#[arg(long, value_enum)]
		rust: bool,

		/// Target directory to check
		target_dir: PathBuf,
	},
}

fn main() {
	v_utils::clientside!();
	let cli = Cli::parse();

	let exit_code = match cli.command {
		Commands::Assert { rust, target_dir } =>
			if rust {
				rust_checks::run_assert(&target_dir)
			} else {
				eprintln!("No language specified. Use --rust");
				1
			},
		Commands::Format { rust, target_dir } =>
			if rust {
				rust_checks::run_format(&target_dir)
			} else {
				eprintln!("No language specified. Use --rust");
				1
			},
	};

	std::process::exit(exit_code);
}
