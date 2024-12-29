use crate::{
    config::{gen_schema, Configs},
    db::Db,
    output::JsonOutput,
    run_checker::{FullOrFastOutputs, Repo, RepoOutput},
    Result,
};
use argh::FromArgs;
use cargo_metadata::camino::{Utf8Path, Utf8PathBuf};
use either::Either;
use eyre::ContextCompat;
use itertools::Itertools;
use serde::Serialize;
use std::{
    fs, io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::SystemTime,
};

mod arg_config;

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
    pub fn execute(mut self) -> Result<()> {
        self.set_configs()?;
        init_repos_base_dir(self.base_dir());

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
            SubArgs::Config(config) => config.execute()?,
            SubArgs::Schema(schema) => gen_schema(&schema.path)?,
            SubArgs::Db(db) => db.execute()?,
        }
        Ok(())
    }

    fn base_dir(&self) -> Utf8PathBuf {
        const BASE_DIR: &str = "repos";

        let file_stem = |config: &str| {
            let config = Utf8Path::new(config);
            Utf8PathBuf::from(config.file_stem().expect("配置文件不含 file stem"))
        };

        match &self.sub_args {
            SubArgs::Run(run) => file_stem(&run.config[0]),
            SubArgs::Batch(batch) => file_stem(&batch.config[0]),
            SubArgs::Layout(layout) => layout.base_dir.clone().unwrap_or_else(|| BASE_DIR.into()),
            _ => BASE_DIR.into(),
        }
    }

    /// Try reading `OS_CHECKER_CONFIGS` env var if no config is given.
    fn set_configs(&mut self) -> Result<()> {
        const OS_CHECKER_CONFIGS: &str = "OS_CHECKER_CONFIGS";

        let mut_config = match &mut self.sub_args {
            SubArgs::Layout(layout) => &mut layout.config,
            SubArgs::Run(run) => &mut run.config,
            SubArgs::Batch(batch) => &mut batch.config,
            SubArgs::Config(config) => &mut config.config,
            SubArgs::Schema(_) => return Ok(()),
            SubArgs::Db(_) => return Ok(()),
        };
        if mut_config.is_empty() {
            if let Ok(configs) = std::env::var(OS_CHECKER_CONFIGS) {
                info!("Set {OS_CHECKER_CONFIGS} as --config arguments.");
                mut_config.extend(configs.trim().split(" ").map(|c| c.trim().to_owned()));
            } else {
                bail!(
                    "Neither {OS_CHECKER_CONFIGS} nor --config exists. Please provide one of them."
                )
            }
        }
        Ok(())
    }
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum SubArgs {
    Layout(ArgsLayout),
    Run(ArgsRun),
    Batch(ArgsBatch),
    Config(arg_config::ArgsConfig),
    Schema(ArgsSchema),
    Db(ArgsDb),
}

/// Display the layouts without installing toolchains or checkers.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "layout")]
struct ArgsLayout {
    /// a path to json configuration file. Refer to https://github.com/os-checker/os-checker/blob/main/assets/JSON-config.md
    /// for the defined format. This can be specified multiple times like
    /// `--config a.json --config b.json`, with the merge from left to right (the config in right wins).
    #[argh(option)]
    config: Vec<String>,
    /// base folder in which a repo locates. e.g. `--base-dir /tmp` means `user/repo` will locate in `/tmp/user/repo`.
    #[argh(option)]
    base_dir: Option<Utf8PathBuf>,
    /// display targets of packages for a given repos. The packages are filterred out as specified
    /// in the config.
    ///
    /// The argument should be a list of `user/repo` separated with comma like `a/b,c/d`. Empty
    /// string means all repos.
    ///
    /// Repos not in the given list will not be downloaded and parsed.
    #[argh(option)]
    list_targets: Option<String>,
    /// json output file path. Default to layout.txt.
    #[argh(option, default = "Utf8PathBuf::from(\"layout.txt\")")]
    out: Utf8PathBuf,
}

/// Run checkers on all repos.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "run")]
pub struct ArgsRun {
    /// a path to json configuration file. Refer to https://github.com/os-checker/os-checker/blob/main/assets/JSON-config.md
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
    /// a path to json configuration file. Refer to https://github.com/os-checker/os-checker/blob/main/assets/JSON-config.md
    /// for the defined format. This can be specified multiple times like
    /// `--config a.json --config b.json`, with the merge from left to right (the config in right wins).
    #[argh(option)]
    config: Vec<String>,

    /// a dir to store the generated batch json config
    #[argh(option)]
    out_dir: Utf8PathBuf,

    /// the argument:
    /// `--size n` generates at most n repos in each batch json config;
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
    fn emit<T>(&self, json: &T) -> Result<()>
    where
        T: std::fmt::Debug + Serialize,
    {
        // trick to have stacked dyn trait objects
        let (mut writer1, mut writer2);
        let writer: &mut dyn io::Write = match &self {
            Emit::Json => {
                writer1 = io::stdout();
                &mut writer1
            }
            Emit::JsonFile(p) => {
                let _span = error_span!("emit", ?p).entered();
                if let Some(parent) = p.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent)?;
                    }
                }
                writer2 = fs::File::create(p)?;
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
    Ok(match configs {
        [] => bail!("No configuration JSON is given."),
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
    })
}

/// 读取和合并配置，然后在每个仓库上执行检查。
///
/// 由于 rustup 不是并发安全的，这里的检查（尤其是安装）必须串行执行。
/// https://github.com/rust-lang/rustup/issues/2417
#[instrument(level = "trace")]
fn repos_outputs(
    configs: &[String],
    db: Option<Db>,
) -> Result<impl Iterator<Item = Result<FullOrFastOutputs>>> {
    Ok(configurations(configs)?
        .into_inner()
        .into_iter()
        .map(move |mut config| {
            config.set_db(db.clone());
            RepoOutput::try_new(config)
        }))
}

impl ArgsRun {
    #[instrument(level = "trace")]
    fn execute(&self) -> Result<()> {
        let db = self.db.as_deref().map(Db::new).transpose()?;
        let start = SystemTime::now();
        let outs = repos_outputs(&self.config, db.clone())?
            .map(|out| {
                let out = out?;
                if let Either::Left(out) = &out {
                    if !self.keep_repo {
                        out.clean_repo_dir()?;
                    }
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

        // 丢弃其他数据库句柄
        drop(outs);
        // 压缩缓存数据库文件
        // FIXME: this moves to db --done
        if let Some(db) = db {
            db.compact();
        }

        Ok(())
    }
}

/// 生成 Repo（比如下载、解析布局、校验配置等）和工具链信息。
#[instrument(level = "trace")]
fn norun(configs: &[String]) -> Result<Vec<Repo>> {
    let repos: Vec<_> = configurations(configs)?
        .into_inner()
        .into_iter()
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

        // FIXME: 我们需要支持 repos 为 None 的情况吗？它代表所有仓库，有意义，但没有需求。
        if self.list_targets.is_some() {
            self.list_targets()?;
        } else {
            let repos = norun(&self.config)?;
            dbg!(repos);
        }

        Ok(())
    }

    fn list_targets(&self) -> Result<()> {
        let list_targets = self.list_targets.as_deref().unwrap();
        let repos = list_targets.split(',').collect::<Vec<_>>();

        let configs = configurations(&self.config)?;
        configs.check_given_repos(&repos)?;

        let targets: Vec<_> = configs
            .into_inner()
            .into_iter()
            .filter(|config| config.is_in_repos(&repos))
            .map(|config| Repo::try_from(config)?.list_targets())
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .sorted_unstable_by(|a, b| (&a.user, &a.repo, &a.pkg).cmp(&(&b.user, &b.repo, &b.pkg)))
            .collect();

        let file = fs::File::create(&self.out)?;
        serde_json::to_writer_pretty(file, &targets)?;

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

fn init_repos_base_dir(path: Utf8PathBuf) {
    // 按照 config.json 设置目录名为 config
    if !path.exists() {
        debug!(%path, "创建 REPOS_BASE_DIR");
        fs::create_dir_all(&path).unwrap();
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

/// Cache redb manipulation.
#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "db")]
struct ArgsDb {
    /// this should be called before all checks start
    #[argh(switch)]
    start: bool,
    /// this should be called after all checks finish
    #[argh(switch)]
    done: bool,
    /// redb file path; this will be created if not exists
    #[argh(positional)]
    db: Utf8PathBuf,
}

impl ArgsDb {
    fn execute(&self) -> Result<()> {
        let db = Db::new(&self.db)?;
        if self.start {
            db.new_check()
        } else if self.done {
            db.check_set_complete()
        } else {
            Ok(())
        }
    }
}
