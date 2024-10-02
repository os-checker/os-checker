use crate::prelude::*;

// 由于我们想对每个检查出了结果时缓存，而不是在仓库所有检查完成时缓存，这里需要重复数据。
// 减少数据重复，需要新定义一个结构，在缓存和 PackagesOutputs 上。
#[derive(Debug, Encode, Decode, Clone)]
pub struct CacheRepoKeyCmd {
    #[musli(with = musli::serde)]
    pub pkg_name: XString,
    pub checker: CacheChecker,
    pub cmd: CacheCmd,
}

#[derive(Debug, Encode, Decode, Clone)]
pub struct CacheRepoKey {
    pub repo: CacheRepo,
    pub cmd: CacheRepoKeyCmd,
}

redb_value!(@key CacheRepoKey, name: "OsCheckerCacheKey",
    read_err: "Not a valid cache key.",
    write_err: "Cache key can't be encoded to bytes."
);

#[derive(Debug, Encode, Decode, Clone)]
pub struct CacheRepo {
    #[musli(with = musli::serde)]
    pub user: XString,
    #[musli(with = musli::serde)]
    pub repo: XString,
    pub sha: String,
    #[musli(with = musli::serde)]
    pub branch: XString,
}

#[derive(Debug, Encode, Decode, Clone)]
struct CacheChecker {
    pub checker: crate::CheckerTool,
    // If we don't care about the version, use None.
    pub version: Option<String>,
    pub sha: Option<String>,
}

#[derive(Debug, Encode, Decode, Clone)]
struct CacheCmd {
    pub cmd: String,
    pub target: String,
    /// FIXME: channel 转换回 RustToolchain 会丢失额外的信息
    pub channel: String,
    // Below is not necessary, and currently not implemented.
    #[musli(with = musli::serde)]
    pub features: Vec<XString>,
    /// rustcflags
    #[musli(with = musli::serde)]
    pub flags: Vec<XString>,
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
    pub file: Utf8PathBuf,
    pub kind: crate::Kind,
    pub raw: String,
}

#[derive(Encode, Decode)]
pub struct CacheValue {
    pub unix_timestamp_milli: u64,
    pub cmd: CacheRepoKeyCmd,
    pub diagnostics: OutputData,
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
