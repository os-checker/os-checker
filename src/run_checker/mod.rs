use crate::{
    config::{CheckerTool, Config, Resolve},
    layout::Layout,
    output::{JsonOutput, Norun},
    Result, XString,
};
use cargo_metadata::{
    camino::{Utf8Path, Utf8PathBuf},
    diagnostic::DiagnosticLevel,
    Message as CargoMessage,
};
use either::Either;
use eyre::Context;
use regex::Regex;
use serde::Deserialize;
use std::{process::Output as RawOutput, sync::LazyLock};

mod lockbud;
/// 把获得的输出转化成 JSON 所需的输出
mod utils;

mod packages_outputs;
use packages_outputs::PackagesOutputs;

#[cfg(test)]
mod tests;

pub struct RepoOutput {
    repo: Repo,
    outputs: PackagesOutputs,
}

impl std::fmt::Debug for RepoOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RepoOutput")
            .field("repo", &self.repo)
            // .field("outputs", &self.outputs)
            .finish()
    }
}

impl RepoOutput {
    pub fn with_json_output(&self, json: &mut JsonOutput) {
        use crate::output::*;
        let user = XString::new(self.repo.config.user_name());
        let repo = XString::new(self.repo.config.repo_name());
        let repo_idx = json.env.repos.len();

        let outputs = &self.outputs;
        // 预留足够的空间
        // TODO: 应该可以在初始化 json.data 的时候就一次性预留空间
        json.data.reserve(outputs.count());

        for (pkg_name, v) in &**outputs {
            let pkg_idx = json.env.packages.len();
            json.env.packages.push(Package {
                name: pkg_name.into(),
                repo: PackageRepo {
                    repo_idx,
                    user: user.clone(),
                    repo: repo.clone(),
                },
                rust_toolchain_idx: v.as_slice().first().and_then(|o| o.resolve.toolchain),
            });
            for o in v.as_slice() {
                utils::push_idx_and_data(pkg_idx, o, &mut json.cmd, &mut json.data);
            }
        }

        let rust_toolchain_idxs = self.repo.layout.rust_toolchain_idxs();
        json.env.repos.push(Repo {
            user,
            repo,
            rust_toolchain_idxs,
        });
    }

    /// 提前删除仓库目录
    #[instrument(level = "trace")]
    pub fn clean_repo_dir(&self) -> Result<()> {
        self.repo.config.clean_repo_dir()
    }
}

#[derive(Debug)]
pub struct Repo {
    layout: Layout,
    config: Config,
}

impl Repo {
    fn new_or_empty(repo_root: &str, dirs_excluded: &[&str], config: Config) -> Repo {
        let layout = Layout::parse(repo_root, dirs_excluded)
            .with_context(|| eyre!("无法解析 `{repo_root}` 内的 Rust 项目布局"));
        match layout {
            Ok(layout) => Self { layout, config },
            Err(err) => Self {
                layout: Layout::empty(repo_root, err),
                config,
            },
        }
    }

    #[instrument(level = "trace")]
    fn resolve(&self) -> Result<Either<Vec<Resolve>, &str>> {
        match self.layout.get_parse_error() {
            Some(err) => Ok(Either::Right(err)),
            None => Ok(Either::Left(self.config.resolve(&self.layout.packages()?)?)),
        }
    }

    #[instrument(level = "trace")]
    fn run_check(&self) -> Result<PackagesOutputs> {
        let mut outputs = PackagesOutputs::new();
        let err_or_resolve = self.resolve()?;
        match err_or_resolve {
            Either::Left(resolve) => {
                for resolve in resolve {
                    run_check(resolve, &mut outputs)?;
                }
            }
            Either::Right(err) => {
                // NOTE: 无法从 repo 中知道 pkg 信息，因此为空
                let pkg_name = String::new();
                let repo_root = self.layout.repo_root();
                let output = Output::new_cargo_from_layout_parse_error(&pkg_name, repo_root, err);
                outputs.push_cargo_layout_parse_error(pkg_name, output);
            }
        }
        Ok(outputs)
    }

    /// 提前删除仓库目录
    #[instrument(level = "trace")]
    pub fn clean_repo_dir(&self) -> Result<()> {
        self.config.clean_repo_dir()
    }

    #[instrument(level = "trace")]
    pub fn norun(&self, norun: &mut Norun) -> Result<()> {
        self.layout.norun(norun);
        // validate pkgs and checkers in cmds
        debug!("resolve = {:#?}", self.resolve()?);
        Ok(())
    }
}

impl TryFrom<Config> for Repo {
    type Error = eyre::Error;

    #[instrument(level = "trace")]
    fn try_from(mut config: Config) -> Result<Repo> {
        let repo_root = config.local_root_path_with_git_clone()?;
        Ok(Repo::new_or_empty(repo_root.as_str(), &[], config))
    }
}

impl TryFrom<Config> for RepoOutput {
    type Error = eyre::Error;

    #[instrument(level = "trace")]
    fn try_from(config: Config) -> Result<RepoOutput> {
        let repo = Repo::try_from(config)?;
        // TODO: 确保工具链安装成功
        let mut outputs = repo.run_check()?;
        outputs.sort_by_name_and_checkers();
        Ok(RepoOutput { repo, outputs })
    }
}

pub struct Output {
    raw: RawOutput,
    parsed: OutputParsed,
    /// 该检查工具报告的总数量；与最后 os-checker 提供原始输出计算的数量应该一致
    count: usize,
    duration_ms: u64,
    resolve: Resolve,
}

impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Output")
            // .field("raw", &self.raw)
            // .field("parsed", &self.parsed)
            .field("count", &self.count)
            .field("duration_ms", &self.duration_ms)
            .field("resolve", &self.resolve)
            .finish()
    }
}

impl Output {
    /// NOTE: &self 应该为非 Cargo checker，即来自实际检查工具的输出
    fn new_cargo_from_checker(&self, stderr_parsed: String) -> Self {
        Output {
            raw: RawOutput {
                // 这个不太重要
                status: self.raw.status,
                stdout: Vec::new(),
                // NOTE: 为了保持 stderr 和 parsed 一致，这里应该为
                // stderr_stripped，但考虑到可能需要合并 parsed，以及
                // stderr string 将来添加 CheckerTool 信息，保持一致需要更多代码，
                // 可实际上有了 parsed，暂时不太需要 RawOutput，因此这里简单为空。
                stderr: Vec::new(),
            },
            parsed: OutputParsed::Cargo {
                source: CargoSource::Checker(self.resolve.checker),
                stderr: stderr_parsed,
            },
            count: 1, // 因为每个 checker 最多产生一个 cargo 诊断的方法
            duration_ms: self.duration_ms,
            resolve: self.resolve.new_cargo(),
        }
    }

    fn new_cargo_from_layout_parse_error(pkg_name: &str, repo_root: &Utf8Path, err: &str) -> Self {
        let (status, stdout, stderr) = Default::default();
        let raw = RawOutput {
            status,
            stdout,
            stderr,
        };
        let parsed = OutputParsed::Cargo {
            source: CargoSource::LayoutParseError(repo_root.into()),
            stderr: err.to_owned(),
        };
        Output {
            raw,
            parsed,
            count: 1,
            duration_ms: 0,
            resolve: Resolve::new_cargo_layout_parse_error(pkg_name, repo_root.into()),
        }
    }
}

/// 由于 Cargo 检查是虚拟的，它代表某种编译错误，来自运行 Cargo
/// 命令或者运行其他检查工具的过程。
#[derive(Debug)]
enum CargoSource {
    Checker(CheckerTool),
    /// repo_root
    LayoutParseError(Box<Utf8Path>),
}

/// 以子进程方式执行检查
#[instrument(level = "trace")]
fn run_check(resolve: Resolve, outputs: &mut PackagesOutputs) -> Result<()> {
    let expr = resolve.expr.clone();
    let (duration_ms, raw) = crate::utils::execution_time_ms(|| {
        expr.stderr_capture().stdout_capture().unchecked().run()
    });
    let raw = raw?;

    let stdout: &[_] = &raw.stdout;
    let stderr: &[_] = &raw.stderr;
    let parsed = match resolve.checker {
        CheckerTool::Fmt => {
            let fmt = serde_json::from_slice(stdout).with_context(|| {
                format!(
                    "无法解析 rustfmt 的标准输出：stdout={}\n原始命令为：\
                    `{}`（即 `{:?}`）\ntoolchain={}\nstderr={}",
                    String::from_utf8_lossy(stdout),
                    resolve.cmd,
                    resolve.expr,
                    resolve.toolchain(),
                    String::from_utf8_lossy(stderr),
                )
            })?;
            OutputParsed::Fmt(fmt)
        }
        CheckerTool::Clippy => OutputParsed::Clippy(
            CargoMessage::parse_stream(stdout)
                .map(|mes| {
                    mes.map(RustcMessage::from).with_context(|| {
                        format!(
                            "解析 Clippy Json 输出失败：stdout={}\n原始命令为：\
                            `{}`（即 `{:?}`）\ntoolchain={}\nstderr={}",
                            String::from_utf8_lossy(stdout),
                            resolve.cmd,
                            resolve.expr,
                            resolve.toolchain(),
                            String::from_utf8_lossy(stderr),
                        )
                    })
                })
                .collect::<Result<_>>()?,
        ),
        CheckerTool::Mirai => OutputParsed::Mirai(
            CargoMessage::parse_stream(stdout)
                .map(|mes| {
                    mes.map(RustcMessage::from).with_context(|| {
                        format!(
                            "解析 Mirai Json 输出失败：stdout={}\n原始命令为：\
                            `{}`（即 `{:?}`）\ntoolchain={}\nstderr={}",
                            String::from_utf8_lossy(stdout),
                            resolve.cmd,
                            resolve.expr,
                            resolve.toolchain(),
                            String::from_utf8_lossy(stderr),
                        )
                    })
                })
                .collect::<Result<_>>()?,
        ),
        CheckerTool::Lockbud => OutputParsed::Lockbud(lockbud::parse_lockbud_result(&raw.stderr)),
        CheckerTool::Miri => todo!(),
        CheckerTool::SemverChecks => todo!(),
        // 由于 run_check 只输出单个 Ouput，而其他检查工具可能会利用 cargo，因此导致发出两类诊断
        CheckerTool::Cargo => panic!("Don't specify cargo as a checker. It's a virtual one."),
    };
    let count = parsed.count();
    let output = Output {
        raw,
        parsed,
        count,
        duration_ms,
        resolve,
    };

    outputs.push_output_with_cargo(output);

    Ok(())
}

#[derive(Debug)]
enum OutputParsed {
    Fmt(Box<[FmtMessage]>),
    Clippy(Box<[RustcMessage]>),
    Mirai(Box<[RustcMessage]>),
    Lockbud(String),
    Cargo { source: CargoSource, stderr: String },
}

impl OutputParsed {
    /// 这里计算的逻辑应该与原始输出的逻辑一致：统计检查工具报告的问题数量（而不是文件数量之类的)
    // 对于 clippy 和 miri?，最后可能有汇总，这应该在计算 count 时排除, e.g.
    // * warning: 10 warnings emitted （结尾）
    //   有时甚至在中途：
    //   [3] warning: 2 warnings emitted
    //   [4] warning: you should consider adding a `Default` implementation for `CFScheduler<T>`
    // * error: aborting due to 7 previous errors; 1 warning emitted （结尾，之后无内容）
    //   有时却依然会追加内容：
    //   [9] error: aborting due to 7 previous errors; 1 warning emitted
    //   [10] Some errors have detailed explanations: E0425, E0432, E0433, E0599.
    //   [11] For more information about an error, try `rustc --explain E0425`.
    fn count(&self) -> usize {
        match self {
            // 一个文件可能含有多处未格式化的报告
            OutputParsed::Fmt(v) => v.iter().map(|f| f.mismatches.len()).sum(),
            OutputParsed::Clippy(v) | OutputParsed::Mirai(v) => v
                .iter()
                .filter_map(|mes| match &mes.tag {
                    RustcTag::WarnDetailed(p) | RustcTag::ErrorDetailed(p) => {
                        // os-checker 根据每个可渲染内容的文件路径来发出原始输出
                        match &mes.inner {
                            CargoMessage::CompilerMessage(cmes)
                                if cmes.message.rendered.is_some() =>
                            {
                                Some(p.len())
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                })
                .sum(),
            OutputParsed::Lockbud(s) => {
                if s.is_empty() {
                    0
                } else {
                    1
                }
            }
            // NOTE: 这个计数不准确，但也不能调用 Vec:::len，因为它包含的 Vec 是动态的，
            // 而最终输出到 JSON 的计数并不调用此方法，因此这里简单的设置为 0，
            // 虽然从最终计数看，cargo 的诊断数量应为 Vec::len。
            OutputParsed::Cargo { .. } => 0,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct FmtMessage {
    name: Utf8PathBuf,
    mismatches: Box<[FmtMismatch]>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct FmtMismatch {
    original_begin_line: u32,
    original_end_line: u32,
    expected_begin_line: u32,
    expected_end_line: u32,
    original: Box<str>,
    expected: Box<str>,
}

// FIXME: 利用 summary 来检验数量
#[allow(dead_code)]
#[derive(Debug)]
pub enum RustcTag {
    /// non-summary / detailed message with primary span paths
    WarnDetailed(Box<[Utf8PathBuf]>),
    /// non-summary / detailed message with primary span paths
    ErrorDetailed(Box<[Utf8PathBuf]>),
    /// warn count in summary
    Warn(u32),
    /// error count in summary
    Error(u32),
    /// warn and error counts in summary
    WarnAndError(u32, u32),
    /// not interested
    None,
}

/// 提取/分类 clippy 的有效信息
fn extract_cargo_message(mes: &CargoMessage) -> RustcTag {
    struct ClippySummary {
        warnings: Regex,
        errors: Regex,
    }

    static REGEX: LazyLock<ClippySummary> = LazyLock::new(|| ClippySummary {
        warnings: Regex::new(r#"(?P<warn>\d+) warnings?"#).unwrap(),
        // FIXME: 貌似有时候输出为 \d+( previous)? errors?
        errors: Regex::new(r#"(?P<error>\d+)( \w+)? errors?"#).unwrap(),
    });

    match mes {
        CargoMessage::CompilerMessage(mes)
            if matches!(
                mes.message.level,
                DiagnosticLevel::Warning | DiagnosticLevel::Error
            ) =>
        {
            let haystack = &mes.message.message;
            let warn = REGEX
                .warnings
                .captures(haystack)
                .and_then(|cap| cap.name("warn")?.as_str().parse::<u32>().ok());
            let error = REGEX
                .errors
                .captures(haystack)
                .and_then(|cap| cap.name("error")?.as_str().parse::<u32>().ok());
            // FIXME: 似乎可以不需要正则表达式来区分是否是 summary，比如 spans 为空?
            match (warn, error) {
                (None, None) => {
                    let spans = mes.message.spans.iter();
                    let mut paths: Box<_> = spans
                        .filter_map(|span| {
                            if span.is_primary {
                                Some(Utf8PathBuf::from(&span.file_name))
                            } else {
                                None
                            }
                        })
                        .collect();
                    // NOTE: path 有可能是空，但该信息依然可能很重要（比如来自 rustc
                    // 报告的错误，只不过没有指向具体的文件路径)
                    if paths.is_empty() {
                        paths = Box::new(["unkonwn-but-maybe-important".into()]);
                    }
                    if matches!(mes.message.level, DiagnosticLevel::Warning) {
                        RustcTag::WarnDetailed(paths)
                    } else {
                        RustcTag::ErrorDetailed(paths)
                    }
                }
                (None, Some(e)) => RustcTag::Error(e),
                (Some(w), None) => RustcTag::Warn(w),
                (Some(w), Some(e)) => RustcTag::WarnAndError(w, e),
            }
        }
        _ => RustcTag::None,
    }
}

#[derive(Debug)]
pub struct RustcMessage {
    inner: CargoMessage,
    tag: RustcTag,
}

impl From<CargoMessage> for RustcMessage {
    fn from(inner: CargoMessage) -> Self {
        RustcMessage {
            tag: extract_cargo_message(&inner),
            inner,
        }
    }
}
