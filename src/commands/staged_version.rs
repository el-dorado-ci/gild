use std::path::Path;

use clap::Args;

use crate::commands::{Command, Error};
use crate::schema::GildFileSparse;
use crate::utils;
use crate::version;

#[derive(Args)]
pub struct StagedVersion;

impl Command for StagedVersion {
    fn run(&self, gild_file: &GildFileSparse, _interactive: bool) -> Result<(), Error> {
        let changefiles = utils::read_changefiles(Path::new(&gild_file.changefile_directory))?;
        let current = version::current(&gild_file.version_source)?;

        let next = match version::pending_bump(&changefiles) {
            Some(bump) => version::next(&current, bump)?,
            None => current,
        };

        println!("{next}");

        Ok(())
    }
}
