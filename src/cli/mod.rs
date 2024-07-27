use crate::{
    repo::Config,
    run_checker::{Repo, RepoStat},
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
/// Reach new heights.
pub struct Args {
    /// an optional height
    #[argh(option, default = r#"Utf8PathBuf::from("repos.yaml")"#)]
    config: Utf8PathBuf,
}

impl Args {
    fn configurations(&self) -> Result<Vec<Config>> {
        Config::from_path(&*self.config)
    }

    fn repos(&self) -> Result<impl ParallelIterator<Item = Result<Repo>>> {
        Ok(self.configurations()?.into_par_iter().map(Repo::try_from))
    }

    pub fn statistics(&self) -> Result<Vec<RepoStat>> {
        self.repos()?.map(|repo| repo?.try_into()).collect()
    }
}
