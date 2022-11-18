//! Module handling all the CLI arguments configuration and parsing.
//!
//! See [`parse_args`] in order to retrieve such arguments from the environment.

use clap::Parser;

/// Cargo entry point for `cargo-liner`.
///
/// This tool is meant to be called using `cargo liner`.
#[derive(clap::Parser, Debug, PartialEq, Eq)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CargoArgs {
    // The only variant: enables validating the input given by Cargo.
    Liner(LinerArgs),
}

// The actual entry point in this tool's argument parser.
#[derive(clap::Args, Debug, PartialEq, Eq)]
#[command(author, version, about, long_about)]
pub struct LinerArgs {
    #[command(subcommand)]
    pub command: Option<LinerCommands>,
}

impl LinerArgs {
    /// Parses the arguments from the environment and returns them.
    ///
    /// Although it does not return `anyhow::Result<Self>`, the function is
    /// actually fallible: it will print an error to stderr and exit the current
    /// process on an error status code if a parsing error occurs.
    pub fn parse_env() -> Self {
        match CargoArgs::parse() {
            CargoArgs::Liner(args) => args,
        }
    }
}

/// Subcommands for the main CLI.
#[derive(clap::Subcommand, Debug, PartialEq, Eq)]
pub enum LinerCommands {
    /// The default command if omitted: install and update configured packages.
    Ship,
    /// Import the `$CARGO_HOME/.crates.toml` Cargo-edited configuration file
    /// as a new Liner configuration file.
    Import(ImportArgs),
}

#[derive(clap::Args, Debug, PartialEq, Eq)]
pub struct ImportArgs {
    /// Overwrite the current configuration file if it exists.
    #[arg(short, long)]
    pub force: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_iserr() {
        assert!(CargoArgs::try_parse_from(["cargo", "liner", "--help"].into_iter()).is_err());
    }

    #[test]
    fn test_version_iserr() {
        assert!(CargoArgs::try_parse_from(["cargo", "liner", "--version"].into_iter()).is_err());
    }

    #[test]
    fn test_no_args() {
        assert_eq!(
            CargoArgs::try_parse_from(["cargo", "liner"].into_iter()).unwrap(),
            CargoArgs::Liner(LinerArgs { command: None }),
        );
    }
}
