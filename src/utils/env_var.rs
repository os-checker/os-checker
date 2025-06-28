use crate::config::CheckerTool;
use std::{env::var, sync::LazyLock};

const FORCE_REPO_CHECK: &str = "FORCE_REPO_CHECK";
const FORCE_RUN_CHECK: &str = "FORCE_RUN_CHECK";

struct Global {
    force_repo_check: bool,
    force_run_check: ForceRunCheck,
}

fn var_bool(env: &str) -> Option<bool> {
    var(env).map(|val| matches!(&*val, "true" | "1")).ok()
}

static GLOBAL: LazyLock<Global> = LazyLock::new(|| Global {
    force_repo_check: var_bool(FORCE_REPO_CHECK).unwrap_or(false),
    force_run_check: ForceRunCheck::new(),
});

/// 当 os-checker 内部支持新检查时，将这个值设置为 true，
/// 来强制运行仓库检查（不影响已有的检查缓存结果）。
/// NOTE: cargo error 的检查结果总是在强制运行仓库检查时更新。
pub fn force_repo_check() -> bool {
    GLOBAL.force_repo_check
}

pub enum ForceRunCheck {
    False,
    All,
    Partial(Vec<CheckerTool>),
}

impl ForceRunCheck {
    /// * FORCE_RUN_CHECK=tool or tool,tool2,... => Partial
    /// * FORCE_RUN_CHECK=true or all => All
    /// * FORCE_RUN_CHECK=anything-else or unset => False
    fn new() -> Self {
        match var(FORCE_REPO_CHECK).map(|s| s.trim().to_ascii_lowercase()) {
            Ok(var) if !var.is_empty() => {
                let sep = ',';
                if var.contains(sep) {
                    let v: Vec<_> = var
                        .split(',')
                        .map(|s| {
                            CheckerTool::from_str(s).unwrap_or_else(|| {
                                panic!("{s:?} in {var:?} isn't a valid CheckerTool")
                            })
                        })
                        .collect();
                    assert!(!v.is_empty());
                    ForceRunCheck::Partial(v)
                } else if let Some(tool) = CheckerTool::from_str(&var) {
                    ForceRunCheck::Partial(vec![tool])
                } else if var == "true" || var == "all" {
                    ForceRunCheck::All
                } else {
                    ForceRunCheck::False
                }
            }
            _ => ForceRunCheck::False,
        }
    }
}

/// 当运行到 run_check 时，是否强制运行检查，不检查是否有缓存。
/// force_repo_check 会控制是否运行 run_check，而 force_run_check 会控制是否照顾缓存。
pub fn force_run_check() -> &'static ForceRunCheck {
    &GLOBAL.force_run_check
}
