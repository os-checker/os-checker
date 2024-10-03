use crate::{prelude::*, CheckerTool, XString};
use cargo_metadata::Metadata;
use std::fmt;

pub type Workspaces = IndexMap<Utf8PathBuf, Metadata>;

#[derive(Serialize, Deserialize, Encode, Decode, Default)]
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
    // /// 一个仓库可能有一个 Workspace，但也可能有多个，比如单独一些 Packages，那么它们是各自的 Workspace
    // /// NOTE: workspaces 的键指向 workspace_root dir，而不是 workspace_root 的 Cargo.toml
    // #[musli(with = musli::serde)]
    /// pub workspaces: Workspaces,
    /// The order is by pkg name and dir path.
    #[musli(with = musli::serde)]
    pub packages_info: Box<[CachePackageInfo]>,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub resolves: Box<[CacheResolve]>,
}

#[derive(Debug, Serialize, Deserialize, Encode, Decode)]
pub struct CacheResolve {
    pub target: String,
    /// 仅当自定义检查命令出现 --target 时为 true
    pub target_overriden: bool,
    pub channel: String,
    pub checker: CheckerTool,
    /// 完整的检查命令字符串（一定包含 --target）：
    /// 来自 os-checker 生成或者配置文件自定义
    pub cmd: String,
}
