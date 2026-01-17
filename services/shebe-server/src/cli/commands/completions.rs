//! Completions command - generate shell completion scripts

use crate::cli::Cli;
use clap::{Args, CommandFactory};
use clap_complete::{generate, Shell};
use std::io;

/// Arguments for the completions command
#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: Shell,
}

/// Execute the completions command
pub fn execute(args: CompletionsArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(args.shell, &mut cmd, name, &mut io::stdout());
    Ok(())
}
