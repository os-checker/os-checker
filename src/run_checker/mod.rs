use crate::{
    layout::Layout,
    repo::{CheckerTool, Config, Resolve},
    Result,
};
use cargo_metadata::{camino::Utf8PathBuf, diagnostic, Message};
use eyre::Context;
use serde::Deserialize;
use std::{process::Output as RawOutput, time::Instant};

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
}

pub struct Output {
    raw: RawOutput,
    parsed: OutputParsed,
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
        match self {
            OutputParsed::Fmt(v) => v.len(),
            OutputParsed::Clippy(v) => v
                .iter()
                .filter(|mes| matches!(mes, Message::CompilerMessage(_)))
                .count(),
        }
    }

    #[cfg(test)]
    fn test_diagnostics(&self) -> String {
        let mut idx = 0;
        match self {
            OutputParsed::Fmt(v) => format!("{v:?}"),
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
        let now = Instant::now();
        let out = res
            .expr
            .stderr_capture()
            .stdout_capture()
            .unchecked()
            .run()?;
        let duration = now.elapsed().as_millis() as u64;

        let stdout: &[_] = &out.stdout;
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

        let success = out.status.success();
        let count = parsed.count();
        let diagnostics = parsed.test_diagnostics();

        // let stdout = std::str::from_utf8(&out.stdout)?;
        // let stderr = std::str::from_utf8(&out.stderr)?;

        // snapshot.push(format!(
        //     "[{} with {:?} checking] {}\nstdout={stdout}\nstderr={stderr}\nparsed={parsed:?}",
        //     res.package.name, res.checker, out.status
        // ));
        snapshot.push(format!(
            "[{} with {:?} checking] success={success} count={count} diagnostics=\n{diagnostics}",
            res.package.name, res.checker
        ));

        debug!(
            "[success={success} count={count}] {} with {:?} checking in {duration}ms",
            res.package.name, res.checker
        );
    }

    let current_path = Utf8PathBuf::from(".").canonicalize_utf8()?;
    let join = snapshot
        .join("\n──────────────────────────────────────────────────────────────────────────────────\n")
        .replace(current_path.as_str(), ".");
    expect_test::expect_file!["./tests.snapshot"].assert_eq(&join);

    Ok(())
}
