use crate::{
    repo::Config,
    run_checker::{json_treenode, Repo, RepoStat},
    Result,
};
use argh::FromArgs;
use cargo_metadata::camino::Utf8PathBuf;
use rayon::prelude::*;

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

    #[argh(option, default = "Emit::AnsiTable")]
    /// emit a format containing the checking reports
    emit: Emit,
}

#[derive(Debug)]
pub enum Emit {
    /// Colorful table printed on terminal.
    AnsiTable,
    /// Used in SSG with PrimeVue and Nuxt. Print to stdout.
    Json,
    /// Used in SSG with PrimeVue and Nuxt. Print to stdout.
    JsonFile(Utf8PathBuf),
}

impl std::str::FromStr for Emit {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Emit> {
        match s.trim() {
            "ansi-table" => Ok(Emit::AnsiTable),
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

    fn repos(&self) -> Result<impl ParallelIterator<Item = Result<Repo>>> {
        Ok(self.configurations()?.into_par_iter().map(Repo::try_from))
    }

    fn statistics(&self) -> Result<Vec<RepoStat>> {
        self.repos()?.map(|repo| repo?.try_into()).collect()
    }

    pub fn run(self) -> Result<()> {
        let stats = self.statistics()?;
        debug!("Got statistics and start to run and emit output.");
        match &self.emit {
            Emit::AnsiTable => {
                for stat in &stats {
                    stat.ansi_table()?;
                }
            }
            Emit::Json => {
                let json = json_treenode(&stats);
                serde_json::to_writer(std::io::stdout(), &json)?;
            }
            Emit::JsonFile(p) => {
                let json = json_treenode(&stats);
                serde_json::to_writer(std::fs::File::create(p)?, &json)?;
            }
        }
        debug!(?self.emit, "Output emitted");
        Ok(())
    }
}
