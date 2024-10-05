use crate::{prelude::*, CheckerTool, XString};
use cargo_metadata::Metadata;
use std::fmt;

/// The json serialized from cargo meta_data.
/// This solves the binary encoding/decoding problem
/// when [some serde attrs like skip are not supported][serde problem].
///
/// [serde problem]: https://docs.rs/bincode/2.0.0-rc.3/bincode/serde/index.html#known-issues
#[derive(Serialize, Deserialize, Encode, Decode)]
pub struct CargoMetaData {
    pub meta_data: String,
}

impl CargoMetaData {
    pub fn meta_data(&self) -> serde_json::Result<Metadata> {
        serde_json::from_str(&self.meta_data)
    }

    pub fn from_meta_data(meta_data: &Metadata) -> serde_json::Result<Self> {
        serde_json::to_string(meta_data).map(|meta_data| Self { meta_data })
    }
}

pub type Workspaces = IndexMap<Utf8PathBuf, CargoMetaData>;

#[derive(Encode, Decode, Default)]
pub struct CacheLayout {
    /// 仓库根目录的完整路径，可用于去除 Metadata 中的路径前缀，让路径看起来更清爽
    #[musli(with = musli::serde)]
    pub root_path: Utf8PathBuf,
    /// 所有 Cargo.toml 的路径
    ///
    /// NOTE: Cargo.toml 并不意味着对应于一个 package —— virtual workspace 布局无需定义
    ///       `[package]`，因此要获取所有 packages 的信息，应使用 [`Layout::packages`]
    #[musli(with = musli::serde)]
    pub cargo_tomls: Box<[Utf8PathBuf]>,
    #[musli(with = musli::serde)]
    pub workspaces: Workspaces,
    /// The order is by pkg name and dir path.
    #[musli(with = musli::serde)]
    pub packages_info: Box<[CachePackageInfo]>,
    pub resolves: Box<[CacheResolve]>,
}

redb_value!(CacheLayout, name: "OsCheckerCacheLayout",
    read_err: "Not a valid cache layout.",
    write_err: "Cache layout can't be encoded to bytes."
);

impl fmt::Debug for CacheLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CacheLayout")
            .field("root_path", &self.root_path)
            .field("cargo_tomls", &self.cargo_tomls)
            // .field("workspaces.len", &self.workspaces.len())
            .field("packages_info", &self.packages_info)
            .finish()
    }
}

/// Refer to https://github.com/os-checker/os-checker/issues/26 for more info.
// FIXME: 把 tag 和 path 分开
// TODO: 在明确指定 targets 的情况下，还需要脚本指定的 targets 吗？(关于安装和 resolve)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TargetSource {
    RustToolchainToml(Utf8PathBuf),
    CargoConfigToml(Utf8PathBuf),
    CargoTomlDocsrsInPkgDefault(Utf8PathBuf),
    CargoTomlDocsrsInWorkspaceDefault(Utf8PathBuf),
    CargoTomlDocsrsInPkg(Utf8PathBuf),
    CargoTomlDocsrsInWorkspace(Utf8PathBuf),
    /// 非上面的方式指定，那么默认会增加一个 host target
    UnspecifiedDefaultToHostTarget,
    DetectedByPkgScripts(Utf8PathBuf),
    DetectedByRepoGithub(Utf8PathBuf),
    DetectedByRepoScripts(Utf8PathBuf),
    // OverriddenInOsCheckerJson, // 覆盖操作直接在生成 cmd 时进行，暂时不会被记录
}

impl TargetSource {
    pub fn descibe(&self) -> (&'static str, Option<&Utf8Path>) {
        match self {
            TargetSource::RustToolchainToml(p) => ("RustToolchainToml", Some(p)),
            TargetSource::CargoConfigToml(p) => ("CargoConfigToml", Some(p)),
            TargetSource::CargoTomlDocsrsInPkgDefault(p) => {
                ("CargoTomlDocsrsInPkgDefault", Some(p))
            }
            TargetSource::CargoTomlDocsrsInWorkspaceDefault(p) => {
                ("CargoTomlDocsrsInWorkspaceDefault", Some(p))
            }
            TargetSource::CargoTomlDocsrsInPkg(p) => ("CargoTomlDocsrsInPkg", Some(p)),
            TargetSource::CargoTomlDocsrsInWorkspace(p) => ("CargoTomlDocsrsInWorkspace", Some(p)),
            TargetSource::UnspecifiedDefaultToHostTarget => {
                ("UnspecifiedDefaultToHostTarget", None)
            }
            TargetSource::DetectedByPkgScripts(p) => ("DetectedByPkgScripts", Some(p)),
            TargetSource::DetectedByRepoGithub(p) => ("DetectedByRepoGithub", Some(p)),
            TargetSource::DetectedByRepoScripts(p) => ("DetectedByRepoScripts", Some(p)),
        }
    }
}

/// A list of target triples obtained from multiple sources.
/// The orders in key and value demonstrates how they shape.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Targets {
    pub map: IndexMap<String, Vec<TargetSource>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CachePackageInfo {
    pub pkg_name: XString,
    /// i.e. manifest_dir
    pub pkg_dir: Utf8PathBuf,
    pub targets: Targets,
    pub channel: String,
}

#[derive(Debug, Encode, Decode)]
pub struct CacheResolve {
    #[musli(with = musli::serde)]
    pub pkg_name: XString,
    pub target: String,
    /// 仅当自定义检查命令出现 --target 时为 true
    pub target_overridden: bool,
    pub channel: String,
    pub checker: CheckerTool,
    /// 完整的检查命令字符串（一定包含 --target）：
    /// 来自 os-checker 生成或者配置文件自定义
    pub cmd: String,
}

#[test]
fn workspaces() {
    #[derive(Encode, Decode)]
    struct Meta {
        #[musli(with=musli::serde)]
        data: Metadata,
    }

    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path("../Cargo.toml")
        .exec()
        .unwrap();

    let meta = Meta { data: metadata };
    let _bytes = musli::storage::to_vec(&meta).unwrap();

    // thread 'layout::workspaces' panicked at os-checker-types\src\layout.rs:105:54:
    // called `Result::unwrap()` on an `Err` value: Error { err: Message("Skipping is
    // not supported, expected type supported by the storage decoder") }
    // let _: Meta = musli::storage::from_slice(&bytes).unwrap();

    let metadata = meta.data;

    let meta = CargoMetaData {
        meta_data: serde_json::to_string_pretty(&metadata).unwrap(),
    };
    let bytes = musli::storage::to_vec(&meta).unwrap();
    let meta_string: CargoMetaData = musli::storage::from_slice(&bytes).unwrap();
    assert!(meta_string.meta_data().is_ok());
}
