use std::path::Path;

use clap::Args;

use crate::commands::query::run_query;
use crate::commands::{Command, Error};
use crate::schema::GildFileSparse;

#[derive(Args)]
pub struct Ask {
    alias: String,
}

impl Command for Ask {
    fn run(&self, gild_file: &GildFileSparse, _interactive: bool) -> Result<(), Error> {
        let jmespath = gild_file.queries.get(&self.alias).ok_or_else(|| {
            Error::UnknownQuery(
                self.alias.clone(),
                gild_file.queries.keys().cloned().collect(),
            )
        })?;

        run_query(jmespath, Path::new(&gild_file.changefile_directory))
    }
}
