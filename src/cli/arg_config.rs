use crate::Result;
use argh::FromArgs;
use serde_json::to_writer_pretty;
use std::io::stdout;

/// emit merged infomation based on given configs. The command will not download any artifacts,
/// unlike layout subcommand does.
///
/// NOTE: arguments except config are exclusive, because any of them will write valid JSON data
/// to stdout; for simplicity, only single argument is allowed, and thus single JSON is emitted.
/// If multiple arguments are given, only the first one of struct in source code will be used.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "config")]
pub struct ArgsConfig {
    /// a path to json configuration file. Refer to https://github.com/os-checker/os-checker/blob/main/assets/JSON-config.md
    /// for the defined format. This can be specified multiple times like
    /// `--config a.json --config b.json`, with the merge from left to right (the config in right wins).
    #[argh(option)]
    pub config: Vec<String>,

    /// emit full merged configuration JSON
    #[argh(switch)]
    pub merged: bool,

    /// emit full merged configuration JSON
    #[argh(switch)]
    pub list_repos: bool,
}

impl ArgsConfig {
    pub fn execute(&self) -> Result<()> {
        let configs = super::configurations(&self.config)?;

        if self.merged {
            to_writer_pretty(stdout(), &configs)?;
            return Ok(());
        }

        if self.list_repos {
            let repos = configs.list_repos();
            to_writer_pretty(stdout(), &repos)?;
            return Ok(());
        }

        Ok(())
    }
}
