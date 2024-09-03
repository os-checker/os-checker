use crate::{
    config::{Config, Configs},
    output::{JsonOutput, Norun},
    run_checker::{Repo, RepoOutput},
    Result,
};
use argh::FromArgs;
use cargo_metadata::camino::Utf8PathBuf;
use eyre::ContextCompat;
use rayon::prelude::*;
use serde::Serialize;
use std::{fs::File, time::SystemTime};

pub fn args() -> Args {
    let arguments = argh::from_env();
    trace!(?arguments);
    arguments
}

#[derive(FromArgs, Debug)]
/// Run a collection of checkers targeting Rust crates, and report
/// bad checking results and statistics.
pub struct Args {
    /// A path to json configuration file. Refer to https://github.com/os-checker/os-checker/blob/main/assets/JSON-config.md
    /// for the defined format.
    #[argh(option)]
    config: Vec<String>,

    #[argh(option, default = "Emit::Json")]
    /// emit a JSON format containing the checking reports
    emit: Emit,

    /// `--norun  --emit a.json` means emitting information like targets without running real checkers
    #[argh(switch)]
    norun: bool,

    /// works with `--norun` to set up all rust-toolchains and checkers
    #[argh(switch)]
    setup: bool,
}

/// 见 `../../assets/JSON-data-format.md`
#[derive(Debug)]
pub enum Emit {
    /// Print to stdout.
    Json,
    /// Save as a json file.
    JsonFile(Utf8PathBuf),
}

impl std::str::FromStr for Emit {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Emit> {
        match s.trim() {
            "json" => Ok(Emit::Json),
            p if s.ends_with(".json") => Ok(Emit::JsonFile(Utf8PathBuf::from(p))),
            _ => bail!("`{s}` is not supported; please specify one of these：ansi-table, json."),
        }
    }
}

impl Args {
    fn configurations(&self) -> Result<Vec<Config>> {
        const DEFAULT: &str = "repos.json";
        let config = match &self.config[..] {
            [] => Configs::from_json_path(DEFAULT)?,
            [path] => Configs::from_json_path(path.as_str())?,
            paths => {
                let configs = paths
                    .iter()
                    .map(|path| Configs::from_json_path(path.as_str()))
                    .collect::<Result<Vec<_>>>()?;
                configs
                    .into_iter()
                    .reduce(Configs::merge)
                    .with_context(|| format!("无法从 {paths:?} 合并到一个 Configs"))?
            }
        };
        Ok(config.into_inner())
    }

    fn repos_outputs(&self) -> Result<impl ParallelIterator<Item = Result<RepoOutput>>> {
        Ok(self
            .configurations()?
            .into_par_iter()
            .map(RepoOutput::try_from))
    }

    fn run(&self) -> Result<()> {
        let start = SystemTime::now();
        let outs = self.repos_outputs()?.collect::<Result<Vec<_>>>()?;
        let finish = SystemTime::now();
        debug!("Got statistics and start to run and emit output.");
        let mut json = JsonOutput::new(&outs);
        json.set_start_end_time(start, finish);

        self.emit(&json)?;

        debug!(?self.emit, "Output emitted");
        Ok(())
    }

    fn emit(&self, json: &impl Serialize) -> Result<()> {
        // trick to have stacked dyn trait objects
        let (mut writer1, mut writer2);
        let writer: &mut dyn std::io::Write = match &self.emit {
            Emit::Json => {
                writer1 = std::io::stdout();
                &mut writer1
            }
            Emit::JsonFile(p) => {
                writer2 = File::create(p)?;
                &mut writer2
            }
        };
        serde_json::to_writer_pretty(writer, json)?;

        Ok(())
    }

    fn norun(&self) -> Result<()> {
        let repos: Vec<_> = self
            .configurations()?
            .into_par_iter()
            .map(Repo::try_from)
            .collect::<Result<_>>()?;
        let mut norun = Norun::new();
        for repo in &repos {
            repo.norun(&mut norun);
        }
        self.emit(&norun)?;
        if self.setup {
            norun.setup()?;
        }
        Ok(())
    }

    pub fn execute(self) -> Result<()> {
        if self.norun {
            self.norun()
        } else {
            self.run()
        }
    }
}
