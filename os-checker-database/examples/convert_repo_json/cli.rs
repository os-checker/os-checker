use crate::Result;
use std::{
    fs::File,
    path::{Path, PathBuf},
    str::FromStr,
};

use os_checker_types::out_json::file_tree::FileTreeRepo;

/// rewrites ui/repos/user/repo/*.json (except basic.json)
#[derive(argh::FromArgs, Debug)]
pub struct Cli {
    /// a path to `ui/repos/` containing `user/repo/*.json`
    /// or a path to single `ui/repos/user/repo/input.json`
    #[argh(option)]
    input: String,
    /// one the values: `stdout | inplace | path/to/output.json`
    #[argh(option, default = "Emit::Stdout")]
    emit: Emit,
}

impl Cli {
    pub fn json_files(self) -> Result<(Vec<PathBuf>, Emit)> {
        let emit = self.emit;
        let path = PathBuf::from(self.input);

        ensure!(path.exists(), "{path:?} doesn't exit");

        if path.is_file() {
            ensure!(
                path.extension() == Some("json".as_ref()),
                "{path:?} is a file, but not a JSON file"
            );
            return Ok((vec![path], emit));
        }

        ensure!(
            !matches!(emit, Emit::File(_)),
            "{emit:?} only works for input file, not for dir"
        );
        ensure!(path.is_dir(), "{path:?} is not a dir");

        let mut v = Vec::new();
        for entry in walkdir::WalkDir::new(path).sort_by_file_name() {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.into_path();
                // all json files, but not basic.json
                if path.extension() == Some("json".as_ref())
                    && path.file_stem() != Some("basic".as_ref())
                {
                    v.push(path);
                }
            }
        }

        Ok((v, emit))
    }
}

#[derive(Debug)]
pub enum Emit {
    Stdout,
    File(PathBuf),
    Inplace,
}

impl Emit {
    pub fn emit(&self, input_file: &Path, json: FileTreeRepo) -> Result<()> {
        let path = match self {
            Emit::Stdout => {
                serde_json::to_writer_pretty(std::io::stdout(), &json)?;
                return Ok(());
            }
            Emit::File(file) => file,
            Emit::Inplace => input_file,
        };
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, &json)?;
        Ok(())
    }
}

impl FromStr for Emit {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        Ok(match s {
            "stdout" => Self::Stdout,
            "inplace" => Self::Inplace,
            _ => Self::File(PathBuf::from(s)),
        })
    }
}
