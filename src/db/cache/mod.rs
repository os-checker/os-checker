use crate::{
    config::{CheckerTool, Resolve},
    output::{get_channel, Cmd, Data, Kind},
    Result, XString,
};
use camino::{Utf8Path, Utf8PathBuf};
use duct::cmd;
use os_checker_types::db as out;
use std::fmt;

mod type_conversion;

// 由于我们想对每个检查出了结果时缓存，而不是在仓库所有检查完成时缓存，这里需要重复数据。
// 减少数据重复，需要新定义一个结构，在缓存和 PackagesOutputs 上。
#[derive(Debug, Clone)]
pub struct CacheRepoKeyCmd {
    pkg_name: XString,
    checker: CacheChecker,
    cmd: CacheCmd,
}

impl CacheRepoKeyCmd {
    pub fn new(resolve: &Resolve) -> Self {
        Self {
            pkg_name: resolve.pkg_name.clone(),
            checker: CacheChecker {
                checker: resolve.checker,
                version: None,
                sha: None,
            },
            cmd: CacheCmd {
                cmd: resolve.cmd.clone(),
                target: resolve.target.clone(),
                channel: get_channel(resolve.toolchain.unwrap_or(0)),
                // TODO: 待支持
                features: vec![],
                flags: vec![],
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheRepoKey {
    repo: CacheRepo,
    cmd: CacheRepoKeyCmd,
}

impl CacheRepoKey {
    pub fn new(repo: &CacheRepo, resolve: &Resolve) -> Self {
        CacheRepoKey {
            repo: repo.clone(),
            cmd: CacheRepoKeyCmd::new(resolve),
        }
    }

    pub fn span(&self) -> tracing::span::EnteredSpan {
        error_span!(
            "CacheRepoKey",
            // user = self.repo.user,
            // repo = self.repo.repo,
            pkg = %self.cmd.pkg_name,
            cmd = self.cmd.cmd.cmd
        )
        .entered()
    }

    pub fn pkg_name(&self) -> &str {
        &self.cmd.pkg_name
    }

    pub fn to_db_key(&self) -> out::CacheRepoKey {
        self.clone().into()
    }
}

#[derive(Debug, Clone)]
pub struct CacheRepo {
    pub user: XString,
    pub repo: XString,
    sha: String,
    branch: XString,
}

impl CacheRepo {
    pub fn new(user: &str, repo: &str, root: &Utf8Path) -> Result<Self> {
        let sha = cmd!("git", "rev-parse", "HEAD").dir(root).read()?;
        let branch = cmd!("git", "branch", "--show-current").dir(root).read()?;
        Ok(Self {
            user: user.into(),
            repo: repo.into(),
            sha: sha.trim().to_owned(),
            branch: branch.trim().into(),
        })
    }

    pub fn new_with_sha(user: &str, repo: &str, sha: &str, branch: String) -> Self {
        Self {
            user: user.into(),
            repo: repo.into(),
            sha: sha.to_owned(),
            branch: branch.into(),
        }
    }

    pub fn assert_eq_sha(&self, other: &Self, err: &str) {
        assert_eq!(self.sha, other.sha, "{err}");
    }
}

#[derive(Debug, Clone)]
struct CacheChecker {
    checker: CheckerTool,
    // If we don't care about the version, use None.
    version: Option<String>,
    sha: Option<String>,
}

#[derive(Debug, Clone)]
struct CacheCmd {
    cmd: String,
    target: String,
    /// FIXME: channel 转换回 RustToolchain 会丢失额外的信息
    channel: String,
    // Below is not necessary, and currently not implemented.
    features: Vec<XString>,
    /// rustcflags
    flags: Vec<XString>,
}

#[derive(Clone)]
pub struct OutputData {
    pub duration_ms: u64,
    pub data: Vec<OutputDataInner>,
}

impl fmt::Debug for OutputData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutputData")
            .field("duration_ms", &self.duration_ms)
            .field("data.len", &self.data.len())
            .finish()
    }
}

#[derive(Clone)]
pub struct OutputDataInner {
    file: Utf8PathBuf,
    kind: Kind,
    raw: String,
}

impl OutputDataInner {
    pub fn new(file: Utf8PathBuf, kind: Kind, raw: String) -> Self {
        Self { file, kind, raw }
    }
}

#[derive(Clone)]
pub struct CacheValue {
    unix_timestamp_milli: u64,
    cmd: CacheRepoKeyCmd,
    diagnostics: OutputData,
}

impl fmt::Debug for CacheValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CacheValue")
            .field(
                "unix_timestamp_milli",
                &super::parse_unix_timestamp_milli(self.unix_timestamp_milli),
            )
            .field("diagnostics.len", &self.diagnostics.data.len())
            .finish()
    }
}

impl CacheValue {
    pub fn new(resolve: &Resolve, duration_ms: u64, data: Vec<OutputDataInner>) -> Self {
        CacheValue {
            unix_timestamp_milli: now(),
            cmd: CacheRepoKeyCmd::new(resolve),
            diagnostics: OutputData { duration_ms, data },
        }
    }

    pub fn append_to_data(&self, cmd_idx: usize, data: &mut Vec<Data>) {
        data.extend(self.diagnostics.data.iter().map(|d| Data {
            cmd_idx,
            file: d.file.clone(),
            kind: d.kind,
            raw: d.raw.clone(),
        }));
    }

    pub fn to_cmd(&self, package_idx: usize) -> Cmd {
        let cmd = &self.cmd;
        Cmd {
            package_idx,
            tool: cmd.checker.checker,
            count: self.count(),
            duration_ms: self.diagnostics.duration_ms,
            cmd: cmd.cmd.cmd.clone(),
            arch: cmd
                .cmd
                .target
                .split_once("-")
                .map(|(arch, _)| arch.into())
                .unwrap_or_default(),
            target_triple: cmd.cmd.target.clone(),
            rust_toolchain: cmd.cmd.channel.clone(),
            features: cmd.cmd.features.clone(),
            flags: cmd.cmd.flags.clone(),
        }
    }

    pub fn checker(&self) -> CheckerTool {
        self.cmd.checker.checker
    }

    pub fn count(&self) -> usize {
        self.diagnostics.data.len()
    }

    pub fn to_db_value(&self) -> out::CacheValue {
        self.clone().into()
    }
}

/// Returns the current unix timestamp in milliseconds.
pub fn now() -> u64 {
    let t = time::OffsetDateTime::from(std::time::SystemTime::now());
    super::unix_timestamp_milli(t)
}
