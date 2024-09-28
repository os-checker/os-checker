use crate::{
    config::{gen_schema, Configs},
    db::Db,
    output::JsonOutput,
    run_checker::{Repo, RepoOutput},
    utils::check_or_install_checkers,
    Result,
};
use argh::FromArgs;
use cargo_metadata::camino::{Utf8Path, Utf8PathBuf};
use eyre::ContextCompat;
use rayon::prelude::*;
use serde::Serialize;
use std::{
    fs::File,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::SystemTime,
};

pub fn args() -> Args {
    let arguments = argh::from_env();
    debug!(?arguments);
    arguments
}

#[derive(FromArgs, Debug)]
/// Run a collection of checkers targeting Rust crates, and report
/// bad checking results and statistics.
pub struct Args {
    #[argh(subcommand)]
    sub_args: SubArgs,
}

impl Args {
    #[instrument(level = "trace")]
    pub fn execute(self) -> Result<()> {
        init_repos_base_dir(self.first_config());
        match self.sub_args {
            SubArgs::Layout(layout) => layout.execute()?,
            SubArgs::Run(run) => {
                run.execute()?;

                // clean repo_dir to save disk space in CI
                let repos_dir = repos_base_dir();
                debug!(%repos_dir, "正在清理所有下载的仓库目录");
                std::fs::remove_dir_all(&repos_dir)?;
                debug!(%repos_dir, "清理成功");
            }
            SubArgs::Batch(batch) => batch.execute()?,
            SubArgs::Schema(schema) => gen_schema(&schema.path)?,
        }
        Ok(())
    }

    fn first_config(&self) -> &str {
        match &self.sub_args {
            SubArgs::Run(run) => &run.config as &[_],
            SubArgs::Batch(batch) => &batch.config,
            _ => &[],
        }
        .first()
        .map(|s| &**s)
        .unwrap_or("repos")
    }
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum SubArgs {
    Layout(ArgsLayout),
    Run(ArgsRun),
    Batch(ArgsBatch),
    Schema(ArgsSchema),
}

/// Display the layouts without installing toolchains or checkers.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "layout")]
struct ArgsLayout {
    /// A path to json configuration file. Refer to https://github.com/os-checker/os-checker/blob/main/assets/JSON-config.md
    /// for the defined format. This can be specified multiple times like
    /// `--config a.json --config b.json`, with the merge from left to right (the config in right wins).
    #[argh(option)]
    config: Vec<String>,
}

/// Run checkers on all repos.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "run")]
pub struct ArgsRun {
    /// A path to json configuration file. Refer to https://github.com/os-checker/os-checker/blob/main/assets/JSON-config.md
    /// for the defined format. This can be specified multiple times like
    /// `--config a.json --config b.json`, with the merge from left to right (the config in right wins).
    #[argh(option)]
    config: Vec<String>,

    #[argh(option, default = "Emit::Json")]
    /// emit a JSON format containing the checking reports
    emit: Emit,

    /// keep the repo once the checks on it are done
    #[argh(switch)]
    keep_repo: bool,

    /// redb file path. If not specified, no cache for checking.
    #[argh(option)]
    db: Option<Utf8PathBuf>,
}

/// Merge configs and split it into batches.
///
/// `os-checker batch --config a.json --config b.json --out-dir batch --size 10`
/// will yield multiple json configs in `batch/`, each containing at most 10 repos.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "batch")]
struct ArgsBatch {
    /// A path to json configuration file. Refer to https://github.com/os-checker/os-checker/blob/main/assets/JSON-config.md
    /// for the defined format. This can be specified multiple times like
    /// `--config a.json --config b.json`, with the merge from left to right (the config in right wins).
    #[argh(option)]
    config: Vec<String>,

    /// a dir to store the generated batch json config
    #[argh(option)]
    out_dir: Utf8PathBuf,

    /// `--size n` generates at most n repos in each batch json config.
    /// `--size 0` generates a single json merged from all repos.
    #[argh(option)]
    size: usize,
}

/// Generate a JSON schema file used to validate JSON config.
/// i.e. `{{ "$schema": "./schema.json", /* write config with JSON LSP */ }}`.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "schema")]
struct ArgsSchema {
    /// path to emitted json, default to schema.json
    #[argh(option, default = r#"Utf8PathBuf::from("schema.json")"#)]
    path: Utf8PathBuf,
}

/// 见 `assets/JSON-data-format.md`
#[derive(Debug, PartialEq)]
pub enum Emit {
    /// Print to stdout.
    Json,
    /// Save as a json file.
    JsonFile(Utf8PathBuf),
}

impl Emit {
    #[instrument(level = "trace")]
    fn emit<T>(&self, json: &T) -> Result<()>
    where
        T: std::fmt::Debug + Serialize,
    {
        // trick to have stacked dyn trait objects
        let (mut writer1, mut writer2);
        let writer: &mut dyn std::io::Write = match &self {
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
}

impl std::str::FromStr for Emit {
    type Err = eyre::Error;

    #[instrument(level = "trace")]
    fn from_str(s: &str) -> Result<Emit> {
        match s.trim() {
            "json" => Ok(Emit::Json),
            p if s.ends_with(".json") => Ok(Emit::JsonFile(Utf8PathBuf::from(p))),
            _ => bail!("`{s}` is not supported; please specify one of these：ansi-table, json."),
        }
    }
}

/// 从配置文件路径中读取配置。
/// 如果指定多个配置文件，则合并成一个大的配置文件。
/// 返回值表示每个仓库的合并之后的配置信息。
#[instrument(level = "trace")]
fn configurations(configs: &[String]) -> Result<Configs> {
    const DEFAULT: &str = "repos.json";
    let config = match configs {
        [] => Configs::from_json_path(DEFAULT.into())?,
        [path] => Configs::from_json_path(path.as_str().into())?,
        paths => {
            let configs = paths
                .iter()
                .map(|path| Configs::from_json_path(path.as_str().into()))
                .collect::<Result<Vec<_>>>()?;
            configs
                .into_iter()
                .reduce(Configs::merge)
                .with_context(|| format!("无法从 {paths:?} 合并到一个 Configs"))?
        }
    };
    Ok(config)
}

/// 读取和合并配置，然后在每个仓库上执行检查。
///
/// 由于 rustup 不是并发安全的，这里的检查（尤其是安装）必须串行执行。
/// https://github.com/rust-lang/rustup/issues/2417
#[instrument(level = "trace")]
fn repos_outputs(
    configs: &[String],
    db: Option<Db>,
) -> Result<impl Iterator<Item = Result<RepoOutput>>> {
    Ok(configurations(configs)?
        .into_inner()
        .into_iter()
        .map(move |mut config| {
            config.set_db(db.clone());
            RepoOutput::try_from(config)
        }))
}

impl ArgsRun {
    #[instrument(level = "trace")]
    fn execute(&self) -> Result<()> {
        check_or_install_checkers()?;
        let db = self.db.as_deref().map(Db::new).transpose()?;
        let start = SystemTime::now();
        let outs = repos_outputs(&self.config, db)?
            .map(|out| {
                let out = out?;
                if !self.keep_repo {
                    out.clean_repo_dir()?;
                }
                Ok(out)
            })
            .collect::<Result<Vec<_>>>()?;
        let finish = SystemTime::now();
        debug!("Got statistics and start to run and emit output.");
        let mut json = JsonOutput::new(&outs);
        json.set_start_end_time(start, finish);

        self.emit.emit(&json)?;

        debug!(?self.emit, "Output emitted");
        Ok(())
    }
}

/// 生成 Repo（比如下载、解析布局、校验配置等）和工具链信息。
#[instrument(level = "trace")]
fn norun(configs: &[String]) -> Result<Vec<Repo>> {
    let repos: Vec<_> = configurations(configs)?
        .into_inner()
        .into_par_iter()
        .map(Repo::try_from)
        .collect::<Result<_>>()?;
    Ok(repos)
}

/// 是否安装工具链和检查工具；仅在 layout 子命令时为 false
static SETUP: AtomicBool = AtomicBool::new(true);

/// 是否安装工具链和检查工具；仅在 layout 子命令时为 false
pub fn is_not_layout() -> bool {
    SETUP.load(Ordering::Relaxed)
}

impl ArgsLayout {
    #[instrument(level = "trace")]
    fn execute(&self) -> Result<()> {
        SETUP.store(false, Ordering::Relaxed);
        let repos = norun(&self.config)?;
        dbg!(repos);
        Ok(())
    }
}

impl ArgsBatch {
    /// 只生成分批的配置文件
    #[instrument(level = "trace")]
    fn execute(&self) -> Result<()> {
        let configs = configurations(&self.config)?;
        configs.batch(self.size, &self.out_dir)?;
        Ok(())
    }
}

static REPOS_BASE_DIR: Mutex<Option<Utf8PathBuf>> = Mutex::new(None);

fn init_repos_base_dir(config: &str) {
    let config = Utf8Path::new(config);
    let path = Utf8PathBuf::from(config.file_stem().expect("配置文件不含 file stem"));
    // 按照 config.json 设置目录名为 config
    if !path.exists() {
        debug!(%path, "创建 REPOS_BASE_DIR");
        std::fs::create_dir(&path).unwrap();
    }
    debug!(%path, "正在初始化 REPOS_BASE_DIR");
    *REPOS_BASE_DIR.lock().unwrap() = Some(path);
    debug!("初始化 REPOS_BASE_DIR 成功");
}

/// 所有 clone 的仓库放置到该目录下
pub fn repos_base_dir() -> Utf8PathBuf {
    REPOS_BASE_DIR
        .lock()
        .expect("无法获取 REPOS_BASE_DIR")
        .as_ref()
        .expect("REPOS_BASE_DIR 尚未设置值")
        .clone()
}
