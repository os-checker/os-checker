#![allow(unused)]
use crate::{
    layout::Layout,
    repo::{CheckerTool, Config, Resolve},
    Result, XString,
};
use cargo_metadata::{camino::Utf8PathBuf, diagnostic::DiagnosticLevel, Message as CargoMessage};
use eyre::{Context, ContextCompat};
use itertools::Itertools;
use owo_colors::OwoColorize;
use regex::Regex;
use serde::Deserialize;
use std::{process::Output as RawOutput, sync::LazyLock, time::Instant};

/// 分析检查工具的结果
mod analysis;
pub use analysis::Statistics;

#[cfg(test)]
mod tests;

pub struct RepoStat {
    repo: Repo,
    stat: Vec<Statistics>,
}

impl RepoStat {
    pub fn print(&self) {
        let repo_path = self.repo.layout.root_path();
        let repo_name = self.repo.config.repo_name();
        println!(
            "The result of checking {} | src: {repo_path}",
            repo_name.bold().black().on_bright_blue()
        );

        for stat in self.stat.iter().filter(|s| !s.check_fine()) {
            println!(
                "{}\n{}",
                stat.table_of_count_of_kind(),
                stat.table_of_count_of_file()
            );
        }
    }
}

impl TryFrom<Config> for RepoStat {
    type Error = eyre::Error;

    fn try_from(config: Config) -> Result<Self> {
        let repo = Repo::try_from(config)?;
        Ok(RepoStat {
            stat: repo.outputs_and_statistics()?,
            repo,
        })
    }
}

impl TryFrom<Repo> for RepoStat {
    type Error = eyre::Error;

    fn try_from(repo: Repo) -> Result<Self> {
        Ok(RepoStat {
            stat: repo.outputs_and_statistics()?,
            repo,
        })
    }
}

#[derive(Debug)]
pub struct Repo {
    layout: Layout,
    config: Config,
}

impl Repo {
    pub fn new(repo_root: &str, dirs_excluded: &[&str], config: Config) -> Result<Repo> {
        let layout = Layout::parse(repo_root, dirs_excluded)
            .with_context(|| eyre!("无法解析 `{repo_root}` 内的 Rust 项目布局"))?;
        Ok(Self { layout, config })
    }

    pub fn resolve(&self) -> Result<Vec<Resolve>> {
        self.config.resolve(self.layout.packages())
    }

    pub fn run_check(&self) -> Result<Vec<Output>> {
        let v: Vec<_> = self.resolve()?.iter().map(run_check).try_collect()?;
        // 由于已经按顺序执行，这里其实无需排序；如果以后引入并发，则需要排序
        // v.sort_unstable_by(|a, b| (&a.package_name, a.checker).cmp(&(&b.package_name, b.checker)));
        Ok(v)
    }

    pub fn outputs_and_statistics(&self) -> Result<Vec<Statistics>> {
        self.run_check().map(Statistics::new)
    }
}

impl TryFrom<Config> for Repo {
    type Error = eyre::Error;

    fn try_from(mut config: Config) -> Result<Repo> {
        let repo_root = config.local_root_path()?;
        Repo::new(repo_root.as_str(), &[], config)
    }
}

pub struct Output {
    raw: RawOutput,
    parsed: OutputParsed,
    count: usize,
    duration_ms: u64,
    package_root: Utf8PathBuf,
    package_name: XString,
    checker: CheckerTool,
}

/// 以子进程方式执行检查
fn run_check(resolve: &Resolve) -> Result<Output> {
    let now = Instant::now();
    let raw = resolve
        .expr
        .stderr_capture()
        .stdout_capture()
        .unchecked()
        .run()?;
    let duration_ms = now.elapsed().as_millis() as u64;
    let stdout: &[_] = &raw.stdout;
    let stderr: &[_] = &raw.stderr;
    let parsed = match resolve.checker {
        CheckerTool::Fmt => {
            let fmt = if raw.status.success() {
                Box::default()
            } else {
                serde_json::from_slice(stdout).with_context(|| {
                    format!(
                        "无法解析 rustfmt 的标准输出：stdout=\n{:?}\nstderr={}\nresolve=\n{resolve:?}",
                        String::from_utf8_lossy(stdout),
                        String::from_utf8_lossy(stderr),
                    )
                })?
            };
            OutputParsed::Fmt(fmt)
        }
        CheckerTool::Clippy => OutputParsed::Clippy(
            CargoMessage::parse_stream(stdout)
                .map(|mes| {
                    mes.map(ClippyMessage::from).with_context(|| {
                        format!(
                            "解析 Clippy Json 输出失败：stdout=\n{:?}\nstderr={}\nresolve=\n{resolve:?}",
                            String::from_utf8_lossy(stdout),
                            String::from_utf8_lossy(stderr),
                        )
                    })
                })
                .collect::<Result<_>>()?,
        ),
        CheckerTool::Miri => todo!(),
        CheckerTool::SemverChecks => todo!(),
        CheckerTool::Lockbud => todo!(),
    };
    let count = parsed.count();
    let package_root = resolve
        .package
        .cargo_toml
        .parent()
        .map(Into::into)
        .with_context(|| format!("{} 无父目录", resolve.package.cargo_toml))?;
    let package_name = XString::from(resolve.package.name);
    trace!(%package_root, %package_name);
    Ok(Output {
        raw,
        parsed,
        count,
        duration_ms,
        package_root,
        package_name,
        checker: resolve.checker,
    })
}

#[derive(Debug)]
pub enum OutputParsed {
    Fmt(Box<[FmtMessage]>),
    Clippy(Box<[ClippyMessage]>),
}

impl OutputParsed {
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
    //
    // 注意：如果使用正则表达式， warning 和 error 之类的名词是单复数感知的。
    //
    // BTW bacon 采用解析 stdout 的内容而不是 JSON 来计算 count:
    // https://github.com/Canop/bacon/blob/main/src/line_analysis.rs
    fn count(&self) -> usize {
        match self {
            OutputParsed::Fmt(v) => v.len(), // 需要 fmt 的文件数量
            OutputParsed::Clippy(v) => v
                .iter()
                .filter_map(|mes| match mes.tag {
                    ClippyTag::Error(n) | ClippyTag::Warn(n) => Some(n as usize),
                    ClippyTag::WarnAndError(w, e) => Some(w as usize + e as usize),
                    _ => None,
                })
                .sum(),
        }
    }

    #[cfg(test)]
    fn test_diagnostics(&self) -> String {
        use std::fmt::Write;

        let mut idx = 0;
        match self {
            OutputParsed::Fmt(v) => {
                let mut buf = String::with_capacity(1024);
                let add = "+";
                let minus = "-";
                for mes in v.iter() {
                    for mis in &mes.mismatches {
                        idx += 1;
                        _ = writeln!(
                            &mut buf,
                            "\n[{idx}] file: {} (original lines from {} to {})",
                            mes.name, mis.original_begin_line, mis.original_end_line
                        );
                        for diff in prettydiff::diff_lines(&mis.original, &mis.expected).diff() {
                            match diff {
                                prettydiff::basic::DiffOp::Insert(s) => {
                                    for line in s {
                                        _ = writeln!(&mut buf, "{add}{line}");
                                    }
                                }
                                prettydiff::basic::DiffOp::Replace(a, b) => {
                                    for line in a {
                                        _ = writeln!(&mut buf, "{minus}{line}");
                                    }
                                    for line in b {
                                        _ = writeln!(&mut buf, "{add}{line}");
                                    }
                                    // println!("~{a:?}#{b:?}")
                                }
                                prettydiff::basic::DiffOp::Remove(s) => {
                                    for line in s {
                                        _ = writeln!(&mut buf, "{minus}{line}");
                                    }
                                }
                                prettydiff::basic::DiffOp::Equal(s) => {
                                    for line in s {
                                        _ = writeln!(&mut buf, " {line}");
                                    }
                                }
                            }
                        }
                    }
                }
                buf
            }
            OutputParsed::Clippy(v) => v
                .iter()
                .filter_map(|mes| {
                    if let CargoMessage::CompilerMessage(mes) = &mes.inner {
                        idx += 1;
                        Some(format!("[{idx}] {}", mes.message))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(""),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct FmtMessage {
    name: Utf8PathBuf,
    mismatches: Box<[FmtMismatch]>,
}

#[derive(Debug, Deserialize)]
pub struct FmtMismatch {
    original_begin_line: u32,
    original_end_line: u32,
    expected_begin_line: u32,
    expected_end_line: u32,
    original: Box<str>,
    expected: Box<str>,
}

#[derive(Debug)]
pub enum ClippyTag {
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
fn extract_cargo_message(mes: &CargoMessage) -> ClippyTag {
    struct ClippySummary {
        warnings: Regex,
        errors: Regex,
    }

    static REGEX: LazyLock<ClippySummary> = LazyLock::new(|| ClippySummary {
        warnings: Regex::new(r#"(?P<warn>\d+) warnings?"#).unwrap(),
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
                    let path = spans
                        .filter_map(|span| {
                            if span.is_primary {
                                Some(Utf8PathBuf::from(&span.file_name))
                            } else {
                                None
                            }
                        })
                        .collect();
                    if matches!(mes.message.level, DiagnosticLevel::Warning) {
                        ClippyTag::WarnDetailed(path)
                    } else {
                        ClippyTag::ErrorDetailed(path)
                    }
                }
                (None, Some(e)) => ClippyTag::Error(e),
                (Some(w), None) => ClippyTag::Warn(w),
                (Some(w), Some(e)) => ClippyTag::WarnAndError(w, e),
            }
        }
        _ => ClippyTag::None,
    }
}

#[derive(Debug)]
pub struct ClippyMessage {
    inner: CargoMessage,
    tag: ClippyTag,
}

impl From<CargoMessage> for ClippyMessage {
    fn from(inner: CargoMessage) -> Self {
        ClippyMessage {
            tag: extract_cargo_message(&inner),
            inner,
        }
    }
}
