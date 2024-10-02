use crate::config::CheckerTool;
use crate::layout::{Targets, Workspaces};
use crate::{Result, XString};
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CacheLayout {
    /// 仓库根目录的完整路径，可用于去除 Metadata 中的路径前缀，让路径看起来更清爽
    pub root_path: Utf8PathBuf,
    /// 所有 Cargo.toml 的路径
    ///
    /// NOTE: Cargo.toml 并不意味着对应于一个 package —— virtual workspace 布局无需定义
    ///       `[package]`，因此要获取所有 packages 的信息，应使用 [`Layout::packages`]
    pub cargo_tomls: Box<[Utf8PathBuf]>,
    /// 一个仓库可能有一个 Workspace，但也可能有多个，比如单独一些 Packages，那么它们是各自的 Workspace
    /// NOTE: workspaces 的键指向 workspace_root dir，而不是 workspace_root 的 Cargo.toml
    pub workspaces: Workspaces,
    /// The order is by pkg name and dir path.
    pub packages_info: Box<[CachePackageInfo]>,
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

#[derive(Debug, Serialize, Deserialize)]
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
