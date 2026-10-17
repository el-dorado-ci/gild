use clap::{Parser, Subcommand};

use crate::commands::add::Add;
use crate::commands::generate::Generate;
use crate::commands::query::Query;

#[derive(Parser)]
pub struct Config {
    #[arg(long, env = "GILD_CONFIG_PATH")]
    pub config_path: Option<String>,

    #[arg(long, global = true, env = "GILD_NON_INTERACTIVE")]
    pub non_interactive: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Add(Add),
    Generate(Generate),
    Query(Query),
}
