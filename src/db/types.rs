use crate::{
    config::{CheckerTool, Resolve},
    output::{get_toolchain, Cmd, Data, Kind},
    Result,
};
use camino::{Utf8Path, Utf8PathBuf};
use duct::cmd;
use musli::{Decode, Encode};
use std::fmt;

// 由于我们想对每个检查出了结果时缓存，而不是在仓库所有检查完成时缓存，这里需要重复数据。
// 减少数据重复，需要新定义一个结构，在缓存和 PackagesOutputs 上。
#[derive(Debug, Encode, Decode, Clone)]
pub struct CacheRepoKeyCmd {
    pkg_name: String,
    checker: CacheChecker,
    cmd: CacheCmd,
}

impl CacheRepoKeyCmd {
    pub fn new(resolve: &Resolve) -> Self {
        Self {
            pkg_name: resolve.pkg_name.as_str().to_owned(),
            checker: CacheChecker {
                checker: resolve.checker,
                version: None,
                sha: None,
            },
            cmd: CacheCmd {
                cmd: resolve.cmd.clone(),
                target: resolve.target.clone(),
                channel: {
                    let mut channel = String::new();
                    get_toolchain(resolve.toolchain.unwrap_or(0), |t| {
                        channel = t.channel.clone()
                    });
                    channel
                },
                // TODO: 待支持
                features: vec![],
                flags: vec![],
            },
        }
    }
}

#[derive(Debug, Encode, Decode, Clone)]
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
            user = self.repo.user,
            repo = self.repo.repo,
            pkg = self.cmd.pkg_name,
            cmd = self.cmd.cmd.cmd
        )
        .entered()
    }

    pub fn pkg_name(&self) -> &str {
        &self.cmd.pkg_name
    }
}

redb_value!(@key CacheRepoKey, name: "OsCheckerCacheKey",
    read_err: "Not a valid cache key.",
    write_err: "Cache key can't be encoded to bytes."
);

#[derive(Debug, Encode, Decode, Clone)]
pub struct CacheRepo {
    pub user: String,
    pub repo: String,
    sha: String,
    branch: String,
}

impl CacheRepo {
    pub fn new(user: &str, repo: &str, root: &Utf8Path) -> Result<Self> {
        let sha = cmd!("git", "rev-parse", "HEAD").dir(root).read()?;
        let branch = cmd!("git", "branch", "--show-current").dir(root).read()?;
        Ok(Self {
            user: user.to_owned(),
            repo: repo.to_owned(),
            sha: sha.trim().to_owned(),
            branch: branch.trim().to_owned(),
        })
    }

    pub fn new_with_sha(user: &str, repo: &str, sha: &str, branch: String) -> Self {
        Self {
            user: user.to_owned(),
            repo: repo.to_owned(),
            sha: sha.to_owned(),
            branch,
        }
    }

    pub fn assert_eq_sha(&self, other: &Self, err: &str) {
        assert_eq!(self.sha, other.sha, "{err}");
    }
}

#[derive(Debug, Encode, Decode, Clone)]
struct CacheChecker {
    checker: CheckerTool,
    // If we don't care about the version, use None.
    version: Option<String>,
    sha: Option<String>,
}

#[derive(Debug, Encode, Decode, Clone)]
struct CacheCmd {
    cmd: String,
    target: String,
    /// FIXME: channel 转换回 RustToolchain 会丢失额外的信息
    channel: String,
    // Below is not necessary, and currently not implemented.
    features: Vec<String>,
    /// rustcflags
    flags: Vec<String>,
}

#[derive(Encode, Decode)]
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

#[derive(Encode, Decode)]
pub struct OutputDataInner {
    #[musli(with = musli::serde)]
    file: Utf8PathBuf,
    kind: Kind,
    raw: String,
}

impl OutputDataInner {
    pub fn new(file: Utf8PathBuf, kind: Kind, raw: String) -> Self {
        Self { file, kind, raw }
    }
}

#[derive(Encode, Decode)]
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

redb_value!(CacheValue, name: "OsCheckerCacheValue",
    read_err: "Not a valid cache value.",
    write_err: "Cache value can't be encoded to bytes."
);

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

    /// 更新检查时间
    #[cfg(test)]
    pub fn update_unix_timestamp(&mut self) {
        self.unix_timestamp_milli = now();
    }

    // /// 更新检查结果
    // pub fn update_diagnostics(&mut self, f: impl FnOnce(OutputData) -> OutputData) {
    //     replace_with::replace_with_or_abort(&mut self.diagnostics, f);
    // }
}

/// Returns the current unix timestamp in milliseconds.
pub fn now() -> u64 {
    let t = time::OffsetDateTime::from(std::time::SystemTime::now());
    super::unix_timestamp_milli(t)
}

#[cfg(test)]
pub fn new_cache() -> (CacheRepoKey, CacheValue) {
    let cmd = CacheRepoKeyCmd {
        pkg_name: "pkg".to_owned(),
        checker: CacheChecker {
            checker: CheckerTool::Clippy,
            version: None,
            sha: None,
        },
        cmd: CacheCmd {
            cmd: "cargo clippy".to_owned(),
            target: "x86".to_owned(),
            channel: "nightly".to_owned(),
            features: vec![],
            flags: vec![],
        },
    };

    let data = OutputData {
        duration_ms: 0,
        data: vec![OutputDataInner {
            file: Default::default(),
            kind: Kind::ClippyError,
            raw: "warning: xxx".to_owned(),
        }],
    };
    let value = CacheValue {
        unix_timestamp_milli: now(),
        cmd: cmd.clone(),
        diagnostics: data,
    };

    let key = CacheRepoKey {
        repo: CacheRepo {
            user: "user".to_owned(),
            repo: "repo".to_owned(),
            sha: "abc".to_owned(),
            branch: "main".to_owned(),
        },
        cmd,
    };

    (key, value)
}
