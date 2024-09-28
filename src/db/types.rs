use crate::{
    config::{CheckerTool, Resolve},
    output::{get_toolchain, Cmd, Data, Kind},
    Result,
};
use camino::{Utf8Path, Utf8PathBuf};
use duct::cmd;
use musli::{storage, Decode, Encode};
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

#[derive(Debug, Encode, Decode)]
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
            cmd = self.cmd.cmd.cmd
        )
        .entered()
    }
}

impl redb::Value for CacheRepoKey {
    type SelfType<'a>
        = Self
    where
        Self: 'a;

    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        storage::from_slice(data).expect("Not a valid cache key.")
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        storage::to_vec(value).expect("Cache key can't be encoded to bytes.")
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("OsCheckerCacheKey")
    }
}

impl redb::Key for CacheRepoKey {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        data1.cmp(data2)
    }
}

#[derive(Debug, Encode, Decode, Clone)]
pub struct CacheRepo {
    user: String,
    repo: String,
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

// #[derive(Debug, Encode, Decode)]
// pub struct CacheRepoValue {
//     inner: Vec<CacheValue>,
// }

// impl CacheRepoValue {
//     pub(super) fn update_unix_timestamp(&mut self) {
//         for cache in &mut self.inner {
//             cache.update_unix_timestamp();
//         }
//     }
// }

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
                &parse_now(self.unix_timestamp_milli)
                    .to_offset(time::UtcOffset::from_hms(8, 0, 0).unwrap()),
            )
            .field("diagnostics.len", &self.diagnostics.data.len())
            .finish()
    }
}

#[derive(Encode, Decode)]
pub struct OutputData {
    pub duration_ms: u64,
    pub data: Vec<OutputDataInner>,
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

impl fmt::Debug for OutputData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OutputData")
            .field("duration_ms", &self.duration_ms)
            .field("data.len", &self.data.len())
            .finish()
    }
}

impl redb::Value for CacheValue {
    type SelfType<'a>
        = Self
    where
        Self: 'a;

    type AsBytes<'a>
        = Vec<u8>
    where
        Self: 'a;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        storage::from_slice(data).expect("Not a valid cache value.")
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'a,
        Self: 'b,
    {
        storage::to_vec(value).expect("Cache value can't be encoded to bytes.")
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("OsCheckerCacheValue")
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

    /// 更新检查时间
    pub(super) fn update_unix_timestamp(&mut self) {
        self.unix_timestamp_milli = now();
    }

    /// 更新检查结果
    pub fn update_diagnostics(&mut self, f: impl FnOnce(OutputData) -> OutputData) {
        replace_with::replace_with_or_abort(&mut self.diagnostics, f);
    }
}

/// Returns the current unix timestamp in milliseconds.
pub fn now() -> u64 {
    let t = time::OffsetDateTime::from(std::time::SystemTime::now());
    let milli = t.millisecond() as u64;
    let unix_t_secs = t.unix_timestamp() as u64;
    unix_t_secs * 1000 + milli
}

pub fn parse_now(ts: u64) -> time::OffsetDateTime {
    match time::OffsetDateTime::from_unix_timestamp((ts / 1000) as i64) {
        Ok(t) => t,
        Err(err) => panic!("{ts} 无法转回时间：{err}"),
    }
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
