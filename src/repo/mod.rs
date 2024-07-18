use crate::Result;
use serde::{de, Deserialize, Deserializer};
use std::{collections::BTreeMap, fmt};

#[cfg(test)]
mod tests;

/// A repo and its checker configurations.
#[derive(Debug)]
pub struct Config {
    repo: String,
    config: RepoConfig,
}

impl Config {
    pub fn from_yaml(yaml: &str) -> Result<Box<[Config]>> {
        let parsed: BTreeMap<String, RepoConfig> = marked_yaml::from_yaml(0, yaml)
            .map_err(|err| eyre!("仓库配置解析错误：{err}\n请检查 yaml 格式或者内容是否正确"))?;
        parsed
            .into_iter()
            .map(|(repo, config)| (Config { repo, config }).check_fork())
            .collect()
    }

    fn check_fork(self) -> Result<Config> {
        self.config.check_tool_action()?;
        // TODO 使用 FORK 环境变量来自动 fork 代码仓库；放置于 cfg(not(test)) 之后
        Ok(self)
    }
}

/// 检查工具
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CheckerTool {
    All,
    Fmt,
    Clippy,
    Miri,
    SemverChecks,
    Lockbud,
}

impl CheckerTool {
    pub fn name(self) -> &'static str {
        match self {
            CheckerTool::All => "all",
            CheckerTool::Fmt => "fmt",
            CheckerTool::Clippy => "clippy",
            CheckerTool::Miri => "miri",
            CheckerTool::SemverChecks => "semver-checks",
            CheckerTool::Lockbud => "lockbud",
        }
    }
}

/// Configuration for single repo.
///
/// Invalid field key will just be ignored without error.
#[derive(Deserialize)]
pub struct RepoConfig {
    all: CheckerAction,
    fmt: CheckerAction,
    clippy: CheckerAction,
    miri: CheckerAction,
    #[serde(rename(deserialize = "semver-checks"))]
    semver_checks: CheckerAction,
    lockbud: CheckerAction,
    packages: Option<BTreeMap<String, RepoConfig>>,
}

macro_rules! filter {
    ($self:ident, $val:ident: $($field:ident => $e:expr,)+) => { $(
        if let Some($val) = &$self.$field {
            $e;
        }
    )+ };
}

impl fmt::Debug for RepoConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("RepoConfig");
        filter!(self, val:
            all => s.field("all", val),
            fmt => s.field("fmt", val),
            clippy => s.field("clippy", val),
            miri => s.field("miri", val),
            semver_checks => s.field("semver-checks", val),
            lockbud => s.field("lockbud", val),
            packages => s.field("packages", val),
        );
        s.finish()
    }
}

impl RepoConfig {
    /// checker 及其操作（包括 packages 字段内的 checkers）；主要用于 check_tool_action
    fn checker_action(&self) -> Vec<(CheckerTool, &Action)> {
        use CheckerTool::*;
        let mut v = Vec::with_capacity(8);
        filter!(self, val:
            all => v.push((All, val)),
            fmt => v.push((Fmt, val)),
            clippy => v.push((Clippy, val)),
            miri => v.push((Miri, val)),
            semver_checks => v.push((SemverChecks, val)),
            lockbud => v.push((Lockbud, val)),
            packages => v.extend(val.values().flat_map(RepoConfig::checker_action)),
        );
        v
    }

    /// 检查 action（尤其是自定义命令）是否与 checker 匹配
    fn check_tool_action(&self) -> Result<()> {
        self.checker_action()
            .into_iter()
            .try_for_each(|(tool, action)| action.check(tool))
    }
}

/// An optional action for a checker.
/// If there is no checker specified, the value is None.
pub type CheckerAction = Option<Action>;

/// Action specified for a checker.
///
/// 每种检查工具具有三种操作：
/// * false 表示不运行检查工具
/// * true 表示以某种启发式的分析来运行检查工具
/// * 字符串表示指定检查工具的运行命令，如果是多行字符串，则意味着每行为一条完整的运行命令
///
/// 但是有一个特殊的 all 检查，它的 true/false 可结合其余检查工具来控制多个工具的运行，比如
///
/// ```yaml
/// user1/repo:
///   all: true # 运行除 miri 之外的检查工具（那些检查工具以 true 方式运行，除非额外指定）
///   miri: false
///
/// user2/repo:
///   all: true # 运行除 miri 之外的检查工具
///   miri: false
///   lockbud: cargo lockbud -k all -l crate1,crate2 # 但指定 lockbud 的运行命令
///
/// user3/repo:
///   all: false # 只运行 fmt 和 clippy 检查
///   fmt: true
///   clippy: true
/// ```
#[derive(Debug)]
pub enum Action {
    Perform(bool),
    Steps(Box<[String]>),
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Action;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("A boolean, string or lines of string.")
            }

            fn visit_str<E>(self, value: &str) -> Result<Action, E>
            where
                E: de::Error,
            {
                /// ignore contents starting from #
                fn no_comment(line: &str) -> Option<String> {
                    let Some(pos) = line.find('#') else {
                        return Some(line.trim().to_owned());
                    };
                    let line = line[..pos].trim();
                    (!line.is_empty()).then(|| line.to_owned())
                }

                let value = value.trim(); // 似乎 `true # comment` 自动去除了注释内容
                Ok(match value {
                    "true" => Action::Perform(true),
                    "false" => Action::Perform(false),
                    value => Action::Steps(value.lines().filter_map(no_comment).collect()),
                })
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Action {
    /// 检查指定的每一条命令是否与工具匹配
    fn check(&self, tool: CheckerTool) -> Result<()> {
        use CheckerTool::*;
        match self {
            Action::Perform(_) => Ok(()),
            Action::Steps(_) if tool == All => Ok(()),
            Action::Steps(steps) => {
                let name = tool.name();
                for step in &steps[..] {
                    ensure!(
                        step.contains(name),
                        "命令 `{step}` 与检查工具 `{name}` 不匹配"
                    );
                }
                Ok(())
            }
        }
    }
}
