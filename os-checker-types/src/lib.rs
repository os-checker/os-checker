#[macro_use]
extern crate tracing;
#[macro_use]
extern crate eyre;

#[macro_use]
mod prelude;
pub use prelude::*;

mod toolchain;
pub use toolchain::*;

pub mod cache;
pub mod checks;
pub mod config;
pub mod info;
pub mod layout;
pub mod table;

pub mod db {
    pub use crate::cache::*;
    pub use crate::checks::*;
    pub use crate::config::RepoConfig;
    pub use crate::info::*;
    pub use crate::layout::*;
    pub use crate::table::*;
}

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonOutput {
    pub env: Env,
    pub cmd: Vec<Cmd>,
    pub data: Vec<Data>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Env {
    pub tools: Tools,
    pub kinds: Kinds,
    pub repos: Vec<Repo>,
    pub packages: Vec<Package>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tools {
    pub rust_toolchains: RustToolchains,
    pub os_checker: ToolOsChecker,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolOsChecker {
    pub start: u64,
    pub finish: u64,
    pub duration_ms: u64,
    // FIXME: main.rs 使用 &'static
    pub git_time: String,
    // FIXME: main.rs 使用 &'static
    pub git_sha: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Repo {
    pub user: XString,
    pub repo: XString,
    // /// 绝大部分情况下一个仓库要么没有设置工具链，要么设置一个，但也不排除诡异的多
    // /// workspace/pkg 会设置自己的工具链。因此此数组长度可能为 0、1、甚至更多。
    // pub rust_toolchain_idxs: Vec<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Package {
    pub name: XString,
    pub repo: PackageRepo,
    // 这里表示仓库设置给的 pkg 设置的工具链，如果没有设置，则为 None
    // pub rust_toolchain_idx: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageRepo {
    pub repo_idx: usize,
    pub user: XString,
    pub repo: XString,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageCargo {
    pub targets: Vec<XString>,
    pub features: Vec<XString>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cmd {
    pub package_idx: usize,
    pub tool: CheckerTool,
    pub cmd: String,
    pub count: usize,
    pub duration_ms: u64,
    /// FIXME: 替换成 target_idx 之后，该字段应该被删除
    pub arch: XString,
    /// FIXME: 替换成 target_idx
    pub target_triple: String,
    // 如果仓库没有指定工具链，则使用主机工具链
    // pub rust_toolchain_idx: usize,
    pub rust_toolchain: String,
    pub features: Vec<XString>,
    pub flags: Vec<XString>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Data {
    /// idx referring to `Vec<Idx>`
    pub cmd_idx: usize,
    pub file: Utf8PathBuf,
    pub kind: Kind,
    pub raw: String,
}

/// The kind a checker reports.
#[derive(Debug, Serialize, Deserialize, Encode, Decode, PartialEq, Eq, Clone, Copy, Hash)]
#[musli(name_all = "name")]
pub enum Kind {
    /// fmt
    Unformatted,
    // FIXME: 带括号的键存在诸多不变，为了编程方便，使用 camel-case；
    // 面向 UI 时，前端会转换成所需的文字。
    #[serde(rename = "Clippy(Warn)")]
    ClippyWarn,
    #[serde(rename = "Clippy(Error)")]
    ClippyError,
    /// miri
    Miri,
    /// semver-checks
    #[serde(rename = "Semver Violation")]
    SemverViolation,
    Audit,
    Mirai,
    #[serde(rename = "Lockbud(Probably)")]
    LockbudProbably,
    #[serde(rename = "Lockbud(Possibly)")]
    LockbudPossibly,
    Rapx,
    Rudra,
    Outdated,
    Geiger,
    Cargo,
}

impl Kind {
    /// should be the same as serde's kebab-case
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Unformatted => "Unformatted",
            Kind::ClippyWarn => "Clippy(Warn)",
            Kind::ClippyError => "Clippy(Error)",
            Kind::Miri => "Miri",
            Kind::SemverViolation => "Semver Violation",
            Kind::Audit => "Audit",
            Kind::Mirai => "Mirai",
            Kind::LockbudProbably => "Lockbud(Probably)",
            Kind::LockbudPossibly => "Lockbud(Possibly)",
            Kind::Rapx => "Rapx",
            Kind::Rudra => "Rudra",
            Kind::Outdated => "Outdated",
            Kind::Geiger => "Geiger",
            Kind::Cargo => "Cargo",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Kinds {
    pub order: Vec<Kind>,
    pub mapping: serde_json::Value,
}

#[derive(
    Debug, Serialize, Deserialize, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash,
)]
#[musli(name_all = "name")]
#[serde(rename_all = "kebab-case")]
pub enum CheckerTool {
    /// 这是一个虚拟的检查工具，它表示 stderr 中含 `^error:` 的情况
    Cargo,
    Clippy,
    Miri,
    SemverChecks,
    Audit,
    Mirai,
    Lockbud,
    Rapx,
    Rudra,
    Outdated,
    Geiger,
    Fmt,
}

impl CheckerTool {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cargo => "Cargo",
            Self::Clippy => "Clippy",
            Self::Miri => "Miri",
            Self::SemverChecks => "SemverChecks",
            Self::Audit => "Audit",
            Self::Mirai => "Mirai",
            Self::Lockbud => "Lockbud",
            Self::Rapx => "Rapx",
            Self::Rudra => "Rudra",
            Self::Outdated => "Outdated",
            Self::Geiger => "Geiger",
            Self::Fmt => "Fmt",
        }
    }
}
