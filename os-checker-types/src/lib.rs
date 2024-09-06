mod prelude;
pub use prelude::*;

mod toolchain;
pub use toolchain::*;

mod targets;
pub use targets::*;

#[derive(Debug, Serialize)]
pub struct JsonOutput {
    pub env: Env,
    pub cmd: Vec<Cmd>,
    pub data: Vec<Data>,
}

#[derive(Debug, Serialize)]
pub struct Env {
    pub tools: Tools,
    pub kinds: Kinds,
    pub repos: Vec<Repo>,
    pub packages: Vec<Package>,
}

#[derive(Debug, Serialize)]
pub struct Tools {
    pub rust_toolchains: RustToolchains,
    pub os_checker: ToolOsChecker,
}

#[derive(Debug, Serialize)]
pub struct ToolOsChecker {
    pub start: u64,
    pub finish: u64,
    pub duration_ms: u64,
    // FIXME: main.rs 使用 &'static
    pub git_time: String,
    // FIXME: main.rs 使用 &'static
    pub git_sha: String,
}

#[derive(Debug, Serialize)]
pub struct Repo {
    pub user: XString,
    pub repo: XString,
    /// 绝大部分情况下一个仓库要么没有设置工具链，要么设置一个，但也不排除诡异的多
    /// workspace/pkg 会设置自己的工具链。因此此数组长度可能为 0、1、甚至更多。
    pub rust_toolchain_idxs: Vec<usize>,
}

#[derive(Debug, Serialize)]
pub struct Package {
    pub name: XString,
    pub repo: PackageRepo,
    /// 这里表示仓库设置给的 pkg 设置的工具链，如果没有设置，则为 None
    pub rust_toolchain_idx: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct PackageRepo {
    pub repo_idx: usize,
    pub user: XString,
    pub repo: XString,
}

#[derive(Debug, Serialize)]
pub struct PackageCargo {
    pub targets: Vec<XString>,
    pub features: Vec<XString>,
}

#[derive(Debug, Serialize)]
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
    /// 如果仓库没有指定工具链，则使用主机工具链
    pub rust_toolchain_idx: usize,
    pub features: Vec<XString>,
    pub flags: Vec<XString>,
}

#[derive(Debug, Serialize)]
pub struct Data {
    /// idx referring to `Vec<Idx>`
    pub cmd_idx: usize,
    pub file: Utf8PathBuf,
    pub kind: Kind,
    pub raw: String,
}

/// The kind a checker reports.
#[derive(Debug, Serialize)]
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
    SemverViolation,
    #[serde(rename = "Lockbud(Probably)")]
    LockbudProbably,
    #[serde(rename = "Lockbud(Possibly)")]
    LockbudPossibly,
    Cargo,
}

#[derive(Debug, Serialize)]
pub struct Kinds {
    order: Vec<Kind>,
    mapping: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum CheckerTool {
    Fmt,
    Clippy,
    Miri,
    SemverChecks,
    Lockbud,
    /// 这是一个虚拟的检查工具，它表示 stderr 中含 `^error:` 的情况
    Cargo,
}
