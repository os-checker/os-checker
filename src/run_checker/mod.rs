#![allow(unused)]
use crate::{
    layout::Layout,
    repo::{CheckerTool, Config, Resolve},
    Result, XString,
};
use cargo_metadata::{camino::Utf8PathBuf, diagnostic::DiagnosticLevel, Message as CargoMessage};
use eyre::Context;
use itertools::Itertools;
use regex::Regex;
use serde::Deserialize;
use std::{process::Output as RawOutput, sync::LazyLock, time::Instant};

/// 分析检查工具的结果
mod analysis;

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
}

pub struct Output {
    raw: RawOutput,
    parsed: OutputParsed,
    duration_ms: u64,
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
    let parsed = match resolve.checker {
        CheckerTool::Fmt => OutputParsed::Fmt(serde_json::from_slice(stdout)?),
        CheckerTool::Clippy => OutputParsed::Clippy(
            CargoMessage::parse_stream(stdout)
                .map(|mes| {
                    mes.map(ClippyMessage::from)
                        .with_context(|| "解析 Clippy Json 输出失败")
                })
                .collect::<Result<_>>()?,
        ),
        CheckerTool::Miri => todo!(),
        CheckerTool::SemverChecks => todo!(),
        CheckerTool::Lockbud => todo!(),
    };
    Ok(Output {
        raw,
        parsed,
        duration_ms,
        package_name: resolve.package.name.into(),
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
    // TODO: 把 count 变成字段，在初始化的时候就能计算
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

#[derive(Debug, Clone, Copy)]
pub enum ClippyTag {
    /// non-summary / detailed message
    WarnDetailed,
    /// non-summary / detailed message
    ErrorDetailed,
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
            let warn = REGEX
                .warnings
                .captures(&mes.message.message)
                .and_then(|cap| cap.name("warn")?.as_str().parse::<u32>().ok());
            let error = REGEX
                .errors
                .captures(&mes.message.message)
                .and_then(|cap| cap.name("error")?.as_str().parse::<u32>().ok());
            match (warn, error) {
                (None, None) => {
                    if matches!(mes.message.level, DiagnosticLevel::Warning) {
                        ClippyTag::WarnDetailed
                    } else {
                        ClippyTag::ErrorDetailed
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

#[test]
fn repo() -> Result<()> {
    crate::test_logger_init("assets/run_checker.log");
    let yaml = "
arceos:
  all: true
  miri: false
";
    let test_suite = Repo::new(
        "repos/os-checker-test-suite",
        &[],
        Config::from_yaml(yaml)?.pop().unwrap(),
    )?;
    let arceos = Repo::new("repos/arceos", &[], Config::from_yaml(yaml)?.pop().unwrap())?;
    let mut resolve = arceos.resolve()?;
    resolve.extend(test_suite.resolve()?);
    let mut snapshot = Vec::with_capacity(resolve.len());
    for res in resolve.iter() {
        let output = run_check(res)?;

        let success = output.raw.status.success();
        let count = output.parsed.count();
        let diagnostics = output.parsed.test_diagnostics();

        snapshot.push(format!(
            "[{} with {:?} checking] success={success} count={count} diagnostics=\n{diagnostics}",
            res.package.name, res.checker
        ));

        debug!(
            "[success={success} count={count}] {} with {:?} checking in {}ms",
            res.package.name, res.checker, output.duration_ms
        );
    }

    let current_path = Utf8PathBuf::from(".").canonicalize_utf8()?;
    let join = snapshot
        .join("\n──────────────────────────────────────────────────────────────────────────────────\n")
        .replace(current_path.as_str(), ".");
    expect_test::expect_file!["./tests.snapshot"].assert_eq(&join);

    Ok(())
}
