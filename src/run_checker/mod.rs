use crate::{
    layout::Layout,
    repo::{CheckerTool, Config, Resolve},
    Result,
};
use cargo_metadata::{camino::Utf8PathBuf, diagnostic::DiagnosticLevel, Message};
use eyre::Context;
use regex::Regex;
use serde::Deserialize;
use std::{process::Output as RawOutput, sync::LazyLock, time::Instant};

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
        self.resolve()?.iter().map(run_check).collect()
    }
}

pub struct Output {
    raw: RawOutput,
    parsed: OutputParsed,
    duration_ms: u64,
}

/// 以子进程方式执行检查
fn run_check(res: &Resolve) -> Result<Output> {
    let now = Instant::now();
    let raw = res
        .expr
        .stderr_capture()
        .stdout_capture()
        .unchecked()
        .run()?;
    let duration_ms = now.elapsed().as_millis() as u64;
    let stdout: &[_] = &raw.stdout;
    let parsed = match res.checker {
        CheckerTool::Fmt => OutputParsed::Fmt(serde_json::from_slice(stdout)?),
        CheckerTool::Clippy => OutputParsed::Clippy(
            Message::parse_stream(stdout)
                .map(|mes| mes.with_context(|| "解析 Clippy Json 输出失败"))
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
    })
}

#[derive(Debug)]
pub enum OutputParsed {
    Fmt(Box<[FmtMessage]>),
    Clippy(Box<[Message]>),
}

impl OutputParsed {
    // FIXME: 对于 clippy 和 miri?，最后可能有汇总，这应该在计算 count 时排除, e.g.
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
        struct ClippySummary {
            warnings: Regex,
            errors: Regex,
        }
        static REGEX: LazyLock<ClippySummary> = LazyLock::new(|| ClippySummary {
            warnings: Regex::new(r#"(?P<warn>\d+) warnings?"#).unwrap(),
            errors: Regex::new(r#"(?P<error>\d+)( \w+)? errors?"#).unwrap(),
        });

        match self {
            OutputParsed::Fmt(v) => v.len(),
            OutputParsed::Clippy(v) => v
                .iter()
                .filter_map(|mes| match mes {
                    Message::CompilerMessage(mes)
                        if matches!(
                            mes.message.level,
                            DiagnosticLevel::Warning | DiagnosticLevel::Error
                        ) =>
                    {
                        let warn = REGEX
                            .warnings
                            .captures(&mes.message.message)
                            .and_then(|cap| cap.name("warn")?.as_str().parse::<usize>().ok());
                        let error = REGEX
                            .errors
                            .captures(&mes.message.message)
                            .and_then(|cap| cap.name("error")?.as_str().parse::<usize>().ok());
                        match (warn, error) {
                            (None, None) => None,
                            (None, Some(e)) => Some(e),
                            (Some(w), None) => Some(w),
                            (Some(w), Some(e)) => Some(w + e),
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
                    if let Message::CompilerMessage(mes) = mes {
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
