use clap::Parser;

#[derive(Parser)]
pub struct Config {
    // Left unset, gild discovers a config file in the working directory.
    #[arg(long, env = "GILD_CONFIG_PATH")]
    pub config_path: Option<String>,
}
