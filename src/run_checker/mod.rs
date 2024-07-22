use crate::{
    layout::Layout,
    repo::{CheckerTool, Config, Resolve},
    Result,
};
use cargo_metadata::{camino::Utf8PathBuf, Message};
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
    crate::logger_init();
    let yaml = "
arceos:
  all: true
  miri: false
";
    let repo = Repo::new(
        "repos/os-checker-test-suite",
        &[],
        Config::from_yaml(yaml)?.pop().unwrap(),
    )?;
    // let repo = Repo::new("repos/arceos", &[], Config::from_yaml(yaml)?.pop().unwrap())?;
    let resolve = repo.resolve()?;
    let mut snapshot = Vec::with_capacity(resolve.len());
    for res in resolve.iter().take(4) {
        let now = Instant::now();
        let out = res
            .expr
            .stderr_capture()
            .stdout_capture()
            .unchecked()
            .run()?;
        let duration = now.elapsed().as_millis() as u64;
        debug!(
            "{} with {:?} checking in {duration}ms",
            res.package.name, res.checker
        );

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

        let stdout = std::str::from_utf8(&out.stdout)?;
        let stderr = std::str::from_utf8(&out.stderr)?;

        snapshot.push(format!(
            "[{} with {:?} checking] {}\nstdout={stdout}\nstderr={stderr}\nparsed={parsed:?}",
            res.package.name, res.checker, out.status
        ));
    }

    let current_path = Utf8PathBuf::from(".").canonicalize_utf8()?;
    let join = snapshot.join("\n\n").replace(current_path.as_str(), ".");
    expect_test::expect_file!["./tests.snapshot"].assert_eq(&join);

    Ok(())
}
