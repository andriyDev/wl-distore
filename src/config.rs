use std::path::PathBuf;

use clap::{Parser, Subcommand};
use thiserror::Error;

pub struct Args {
    pub layouts: PathBuf,
    pub save_and_exit: bool,
}

impl Args {
    /// Collects the arguments to the binary using flags and config files.
    pub fn collect() -> Result<Self, CollectArgsError> {
        let flags = Flags::parse();
        // Sanity check that the layouts path is meant to be a path to a file.
        if flags.layouts.ends_with("/") {
            return Err(CollectArgsError::LayoutsPathIsDirectory(flags.layouts));
        }
        let layouts = match expanduser::expanduser(&flags.layouts) {
            Ok(path) => path,
            Err(err) => {
                return Err(CollectArgsError::CouldNotExpandUserForLayouts(
                    flags.layouts,
                    err,
                ));
            }
        };
        Ok(Args {
            layouts,
            save_and_exit: matches!(flags.command, Some(Command::SaveCurrent)),
        })
    }
}

#[derive(Debug, Error)]
pub enum CollectArgsError {
    #[error("The layouts path \"{0}\" ends in a slash, so is interpreted as a directory")]
    LayoutsPathIsDirectory(String),
    #[error("Could not expand the user for the layouts path \"{0}\": {1}")]
    CouldNotExpandUserForLayouts(String, std::io::Error),
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Flags {
    /// The file to save and load layout data to/from.
    #[arg(long, default_value = "~/.local/state/wl-distore/layouts.json")]
    layouts: String,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Saves the current layout and exits. This can be used to fix a broken config, or otherwise
    /// adjust configuration without needing to have wl-distore watching.
    SaveCurrent,
}
