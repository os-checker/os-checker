use crate::{config::CheckerTool, run_checker::RepoOutput, XString};
use cargo_metadata::camino::Utf8PathBuf;
use musli::{Decode, Encode};
use serde::Serialize;
use std::time::SystemTime;

mod toolchain;
pub use toolchain::{
    get_toolchain, host_target_triple, host_toolchain, init_toolchain_info, install_toolchain_idx,
    push_toolchain, uninstall_toolchains, RustToolchains,
};

#[derive(Debug, Serialize)]
pub struct JsonOutput {
    pub env: Env,
    pub cmd: Vec<Cmd>,
    pub data: Vec<Data>,
}

impl JsonOutput {
    pub fn new(outputs: &[RepoOutput]) -> Self {
        let mut json = Self {
            env: Env {
                tools: Tools::new(),
                kinds: Kinds::new(),
                repos: vec![],
                packages: vec![],
            },
            cmd: vec![],
            data: vec![],
        };
        outputs.iter().for_each(|s| s.with_json_output(&mut json));
        json
    }

    /// 设置 os-checker 开始运行检查和完成所有检查（得到所有结果，但不包含转换成
    /// JSON 格式）的时间
    pub fn set_start_end_time(&mut self, start: SystemTime, finish: SystemTime) {
        self.env.tools.os_checker.start = unix_timestamp(start);
        self.env.tools.os_checker.finish = unix_timestamp(finish);
        self.env.tools.os_checker.duration_ms =
            finish.duration_since(start).unwrap().as_millis() as u64;
    }
}

#[derive(Debug, Serialize)]
pub struct Env {
    tools: Tools,
    kinds: Kinds,
    pub repos: Vec<Repo>,
    pub packages: Vec<Package>,
}

#[derive(Debug, Serialize)]
pub struct Tools {
    rust_toolchains: RustToolchains,
    os_checker: ToolOsChecker,
}

impl Tools {
    pub fn new() -> Self {
        Self {
            rust_toolchains: RustToolchains::new(),
            os_checker: ToolOsChecker::new_without_duration(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ToolOsChecker {
    start: u64,
    finish: u64,
    duration_ms: u64,
    git_time: &'static str,
    git_sha: &'static str,
}

impl ToolOsChecker {
    fn new_without_duration() -> Self {
        let [start, finish, duration_ms] = [0; 3];
        let [git_time, git_sha] = [env!("OS_CHECKER_GIT_TIME"), env!("OS_CHECKER_GIT_SHA")];
        Self {
            start,
            finish,
            duration_ms,
            git_time,
            git_sha,
        }
    }
}

// Get current unix timestamp in ms which is handled in WebUI for example.
fn unix_timestamp(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[derive(Debug, Serialize)]
pub struct Repo {
    pub user: XString,
    pub repo: XString,
    // /// 绝大部分情况下一个仓库要么没有设置工具链，要么设置一个，但也不排除诡异的多
    // /// workspace/pkg 会设置自己的工具链。因此此数组长度可能为 0、1、甚至更多。
    // pub rust_toolchain_idxs: Vec<usize>,
}

#[derive(Debug, Serialize)]
pub struct Package {
    pub name: XString,
    pub repo: PackageRepo,
    // 这里表示仓库设置给的 pkg 设置的工具链，如果没有设置，则为 None
    // pub rust_toolchain_idx: Option<usize>,
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
    // /// 如果仓库没有指定工具链，则使用主机工具链（这对缓存不友好，暂时放弃）
    // pub rust_toolchain_idx: usize,
    /// channel 名
    pub rust_toolchain: String,
    pub features: Vec<String>,
    pub flags: Vec<String>,
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
#[derive(Debug, Serialize, Decode, Encode, Clone, Copy)]
#[allow(dead_code)]
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
    Mirai,
    #[serde(rename = "Lockbud(Probably)")]
    LockbudProbably,
    #[serde(rename = "Lockbud(Possibly)")]
    LockbudPossibly,
    Cargo,
}

#[derive(Debug, Serialize)]
struct Kinds {
    order: Vec<Kind>,
    mapping: serde_json::Value,
}

impl Kinds {
    fn new() -> Kinds {
        use Kind::*;
        // 工具名小写的 snake_case，但类别名为 PascalCase
        Kinds {
            order: vec![
                Cargo,
                ClippyError,
                ClippyWarn,
                Mirai,
                LockbudProbably,
                LockbudPossibly,
                Unformatted,
            ],
            mapping: serde_json::json!({
                "cargo": [Cargo],
                "clippy": [ClippyError, ClippyWarn],
                "mirai": [Mirai],
                "lockbud": [LockbudProbably, LockbudPossibly],
                "fmt": [Unformatted]
            }),
        }
    }
}
