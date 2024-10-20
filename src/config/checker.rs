use musli::{Decode, Encode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub const TOOLS: usize = 6; // 目前支持的检查工具数量

/// 检查工具
#[derive(
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Clone,
    Copy,
    PartialOrd,
    Ord,
    Hash,
    JsonSchema,
    Encode,
    Decode,
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
    Rap,
    Outdated,
    /// 这是一个虚拟的检查工具，它表示 stderr 中含 `^error:` 的情况
    Cargo,
}

impl CheckerTool {
    /// The checker name invoked in CLI
    pub fn name(self) -> &'static str {
        match self {
            CheckerTool::Fmt => "fmt",
            CheckerTool::Clippy => "clippy",
            CheckerTool::Miri => "miri",
            CheckerTool::SemverChecks => "semver-checks",
            CheckerTool::Audit => "audit",
            CheckerTool::Mirai => "mirai",
            CheckerTool::Lockbud => "lockbud",
            CheckerTool::Rap => "rap",
            CheckerTool::Outdated => "outdated",
            CheckerTool::Cargo => "cargo",
        }
    }
}
