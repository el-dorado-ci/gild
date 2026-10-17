use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct Config {
    // Left unset, gild discovers a config file in the working directory.
    #[arg(long, env = "GILD_CONFIG_PATH")]
    pub config_path: Option<String>,

    #[arg(long, global = true, env = "GILD_NON_INTERACTIVE")]
    pub non_interactive: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Add,
    Generate,
}
