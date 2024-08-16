use crate::{
    layout::Layout,
    output::JsonOutput,
    repo::{CheckerTool, Config, Resolve},
    Result, XString,
};
use cargo_metadata::{camino::Utf8PathBuf, diagnostic::DiagnosticLevel, Message as CargoMessage};
use eyre::{Context, ContextCompat};
use itertools::Itertools;
use owo_colors::OwoColorize;
use regex::Regex;
use serde::Deserialize;
use std::{
    io::{self, Write},
    process::Output as RawOutput,
    sync::LazyLock,
    time::Instant,
};

/// 分析检查工具的结果
mod analysis;
pub use analysis::{RawReportsOnFile, Statistics, TreeNode};

/// 把获得的输出转化成 JSON 所需的输出
mod utils;

#[cfg(test)]
mod tests;

pub struct RepoStat {
    repo: Repo,
    stat: Vec<Statistics>,
}

impl RepoStat {
    pub fn ansi_table(&self) -> Result<()> {
        let stdout = io::stdout();
        let repo_path = self.repo.layout.root_path();
        let repo_name = self.repo.config.repo_name();
        writeln!(
            &stdout,
            "The result of checking {} | src: {repo_path}",
            repo_name.bold().black().on_bright_blue()
        )?;

        for stat in self.stat.iter().filter(|s| !s.check_fine()) {
            writeln!(
                &stdout,
                "{}\n{}",
                stat.table_of_count_of_kind(),
                stat.table_of_count_of_file()
            )?;
        }

        Ok(())
    }

    /// Node = { key: string, data: any, children: Node[] }
    pub fn json(
        &self,
        key: &mut usize,
        raw_reports: &mut Vec<(usize, RawReportsOnFile)>,
    ) -> TreeNode {
        let user = XString::new(self.repo.config.user_name());
        let repo = XString::new(self.repo.config.repo_name());
        TreeNode::json_node(&self.stat, key, user, repo, raw_reports)
    }

    pub fn with_json_output(&self, json: &mut JsonOutput) {
        use crate::output::*;
        let user = XString::new(self.repo.config.user_name());
        let repo = XString::new(self.repo.config.repo_name());
        let repo_idx = json.env.repos.len();
        for stat in &self.stat {
            let pkg_idx = json.env.packages.len();
            json.env.packages.push(Package {
                name: stat.pkg_name(),
                repo: PackageRepo {
                    idx: repo_idx,
                    user: user.clone(),
                    repo: repo.clone(),
                },
            });

            let raw_outputs = stat.raw_outputs();
            // 预留足够的空间
            // TODO: 应该可以在初始化 json.data 的时候就一次性预留空间
            json.data.reserve(raw_outputs.iter().map(|r| r.count).sum());
            for raw in raw_outputs {
                utils::push_idx_and_data(pkg_idx, raw, &mut json.idx, &mut json.data);
            }
        }
        json.env.repos.push(Repo { user, repo });
    }
}

pub fn json_treenode(stats: &[RepoStat]) -> (Vec<TreeNode>, Vec<RawReportsOnFile>) {
    let key = &mut 0;
    let mut raw_reports = Vec::with_capacity(32);
    let tree = stats
        .iter()
        .map(|s| s.json(key, &mut raw_reports))
        .collect();
    raw_reports.sort_unstable_by_key(|(key, _)| *key);
    // TODO: 如何处理 raw_reports 的汇总？显然直接重复 raw_reports 是浪费存储的。
    // 这是一个微优化，需要更紧凑的数据组织方式（比如通过索引和数组来统一汇总与详情）。
    (tree, raw_reports.into_iter().map(|val| val.1).collect_vec())
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

#[allow(dead_code)]
pub struct Output {
    raw: RawOutput,
    parsed: OutputParsed,
    /// 该检查工具报告的总数量；与最后 os-checker 提供原始输出计算的数量应该一致
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

    trace!(
        ?resolve,
        success = raw.status.success(),
        stderr = %String::from_utf8_lossy(&raw.stderr),
    );

    let stdout: &[_] = &raw.stdout;
    let parsed = match resolve.checker {
        CheckerTool::Fmt => {
            let fmt = serde_json::from_slice(stdout).with_context(|| {
                format!(
                    "无法解析 rustfmt 的标准输出：stdout={:?}",
                    String::from_utf8_lossy(stdout),
                )
            })?;
            OutputParsed::Fmt(fmt)
        }
        CheckerTool::Clippy => OutputParsed::Clippy(
            CargoMessage::parse_stream(stdout)
                .map(|mes| {
                    mes.map(ClippyMessage::from).with_context(|| {
                        format!(
                            "解析 Clippy Json 输出失败：stdout={:?}",
                            String::from_utf8_lossy(stdout),
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
            OutputParsed::Clippy(v) => v
                .iter()
                .filter_map(|mes| match &mes.tag {
                    ClippyTag::WarnDetailed(p) | ClippyTag::ErrorDetailed(p) => {
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
                        ClippyTag::WarnDetailed(paths)
                    } else {
                        ClippyTag::ErrorDetailed(paths)
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
