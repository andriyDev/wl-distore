use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::{Parser, Subcommand};
use serde::Deserialize;
use thiserror::Error;

pub struct Args {
    pub layouts: PathBuf,
    pub apply_command: Option<Arc<str>>,
    pub save_and_exit: bool,
}

impl Args {
    /// Collects the arguments to the binary using flags and config files.
    pub fn collect() -> Result<Self, CollectArgsError> {
        let mut flags = Flags::parse();
        let flag_config = Config::take_from_flags(&mut flags);

        let config_path = flags
            .config
            .as_ref()
            .map(String::as_str)
            .unwrap_or("~/.config/wl-distore/config.toml");

        let config_path = match expanduser::expanduser(&config_path) {
            Ok(path) => path,
            Err(err) => {
                return Err(CollectArgsError::CouldNotExpandUser(
                    config_path.to_string(),
                    err,
                ));
            }
        };
        let file_config = load_config_from_file(&config_path)?;

        let mut config = Config::create_default();
        config.override_with(file_config);
        config.override_with(flag_config);

        let layouts = config.layouts.unwrap();
        // Sanity check that the layouts path is meant to be a path to a file.
        if layouts.ends_with("/") {
            return Err(CollectArgsError::LayoutsPathIsDirectory(layouts));
        }
        let layouts = match expanduser::expanduser(&layouts) {
            Ok(path) => path,
            Err(err) => {
                return Err(CollectArgsError::CouldNotExpandUser(layouts, err));
            }
        };
        Ok(Args {
            layouts,
            apply_command: config.apply_command.map(|s| s.into()),
            save_and_exit: matches!(flags.command, Some(Command::SaveCurrent)),
        })
    }
}

#[derive(Debug, Error)]
pub enum CollectArgsError {
    #[error("Failed to read the config file: {0}")]
    FailedToReadConfigFile(std::io::Error),
    #[error("Failed to parse the config file: {0}")]
    FailedToParseConfigFile(toml::de::Error),
    #[error("The layouts path \"{0}\" ends in a slash, so is interpreted as a directory")]
    LayoutsPathIsDirectory(String),
    #[error("Could not expand the user for path \"{0}\": {1}")]
    CouldNotExpandUser(String, std::io::Error),
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Flags {
    /// The config file to read from. [default=~/.config/wl-distore/config.toml]
    #[arg(long)]
    config: Option<String>,
    /// The file to save and load layout data to/from. [default=~/.local/state/wl-distore/layouts.json]
    #[arg(long)]
    layouts: Option<String>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Saves the current layout and exits. This can be used to fix a broken config, or otherwise
    /// adjust configuration without needing to have wl-distore watching.
    SaveCurrent,
}

#[derive(Deserialize, Default)]
struct Config {
    /// The file to save and load layout data to/from.
    layouts: Option<String>,
    /// The command to run after applying a layout.
    apply_command: Option<String>,
}

impl Config {
    /// Creates a default config which all fields fall back to.
    fn create_default() -> Self {
        Self {
            layouts: Some("~/.local/state/wl-distore/layouts.json".into()),
            apply_command: None,
        }
    }

    /// Takes the relevant fields from `flags` and creates a [`Config`].
    fn take_from_flags(flags: &mut Flags) -> Self {
        Self {
            layouts: flags.layouts.take(),
            apply_command: None,
        }
    }

    /// Overrides any fields in `self` with any non-[`None`] values in `overrides`.
    fn override_with(&mut self, overrides: Self) {
        self.layouts = overrides.layouts.or(self.layouts.take());
        self.apply_command = overrides.apply_command.or(self.apply_command.take());
    }
}

/// Loads a config from `path`.
fn load_config_from_file(path: &Path) -> Result<Config, CollectArgsError> {
    let config = match std::fs::read_to_string(path) {
        Ok(config) => config,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Config::default()),
        Err(err) => return Err(CollectArgsError::FailedToReadConfigFile(err)),
    };

    toml::from_str(&config).map_err(|err| CollectArgsError::FailedToParseConfigFile(err))
}
