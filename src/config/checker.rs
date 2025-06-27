use camino::Utf8Path;
use musli::{Decode, Encode};
use serde::{Deserialize, Serialize};
use CheckerTool::*;

pub const TOOLS: usize = 11; // 目前支持的检查工具数量

/// 检查工具
#[derive(
    Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash, Encode, Decode,
)]
#[serde(rename_all = "kebab-case")]
pub enum CheckerTool {
    Fmt,
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
    /// 这是一个虚拟的检查工具，它表示 stderr 中含 `^error:` 的情况
    Cargo,
}

impl CheckerTool {
    /// The checker name invoked in CLI
    pub fn name(self) -> &'static str {
        match self {
            Fmt => "fmt",
            Clippy => "clippy",
            Miri => "miri",
            SemverChecks => "semver-checks",
            Audit => "audit",
            Mirai => "mirai",
            Lockbud => "lockbud",
            Rapx => "rapx",
            Rudra => "rudra",
            Outdated => "outdated",
            Geiger => "geiger",
            Cargo => "cargo",
        }
    }

    /// To reduce outdated artifacts of other checkers,
    /// call cargo clean before some checkers start.
    pub fn cargo_clean(self, workspace_dirs: &[&Utf8Path]) {
        if matches!(self, Mirai | Rapx | Geiger) {
            let clean = &duct::cmd!("cargo", "clean");
            for dir in workspace_dirs {
                if let Err(err) = clean.clone().dir(dir).run() {
                    error!(?self, %dir, ?err, "Failed to call cargo clean.");
                }
            }
        }
    }
}
