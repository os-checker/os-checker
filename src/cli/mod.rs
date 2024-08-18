use crate::{output::JsonOutput, repo::Config, run_checker::RepoOutput, Result};
use argh::FromArgs;
use cargo_metadata::camino::Utf8PathBuf;
use rayon::prelude::*;
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
    /// A path to yaml configuration file. Refer to https://github.com/os-checker/os-checker/issues/5
    /// for the defined format.
    #[argh(option, default = r#"Utf8PathBuf::from("repos.yaml")"#)]
    config: Utf8PathBuf,

    #[argh(option, default = "Emit::Json")]
    /// emit a JSON format containing the checking reports
    emit: Emit,
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
        Config::from_path(&*self.config)
    }

    fn repos_outputs(&self) -> Result<impl ParallelIterator<Item = Result<RepoOutput>>> {
        Ok(self
            .configurations()?
            .into_par_iter()
            .map(RepoOutput::try_from))
    }

    pub fn run(self) -> Result<()> {
        let start = SystemTime::now();
        let outs = self.repos_outputs()?.collect::<Result<Vec<_>>>()?;
        let end = SystemTime::now();
        debug!("Got statistics and start to run and emit output.");
        match &self.emit {
            Emit::Json => {
                let mut json = JsonOutput::new(&outs);
                json.set_start_end_time(start, end);
                serde_json::to_writer_pretty(std::io::stdout(), &json)?;
            }
            Emit::JsonFile(p) => {
                let mut json = JsonOutput::new(&outs);
                json.set_start_end_time(start, end);
                serde_json::to_writer_pretty(File::create(p)?, &json)?;
            }
        }
        debug!(?self.emit, "Output emitted");
        Ok(())
    }
}
