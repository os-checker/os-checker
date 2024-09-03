use serde::{Deserialize, Serialize};
pub const TOOLS: usize = 6; // 目前支持的检查工具数量

/// 检查工具
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

impl CheckerTool {
    /// The checker name invoked in CLI
    pub fn name(self) -> &'static str {
        match self {
            CheckerTool::Fmt => "fmt",
            CheckerTool::Clippy => "clippy",
            CheckerTool::Miri => "miri",
            CheckerTool::SemverChecks => "semver-checks",
            CheckerTool::Lockbud => "lockbud",
            CheckerTool::Cargo => "cargo",
        }
    }
}
