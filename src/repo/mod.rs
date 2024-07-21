use crate::{layout::Package, Result};
use duct::Expression;
use serde::{de, Deserialize, Deserializer};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

mod cmd;
use cmd::*;

#[cfg(test)]
mod tests;

/// A repo and its checker configurations.
#[derive(Debug)]
pub struct Config {
    repo: String,
    config: RepoConfig,
}

impl Config {
    pub fn from_yaml(yaml: &str) -> Result<Vec<Config>> {
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

const TOOLS: usize = 4; // 目前支持的检查工具数量

/// 检查工具
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CheckerTool {
    Fmt,
    Clippy,
    Miri,
    SemverChecks,
    Lockbud,
}

impl CheckerTool {
    pub fn name(self) -> &'static str {
        match self {
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
    // FIXME: 这里需要重构
    // * 禁止嵌套：把工具放到单独的结构体 S，将 V 替换成 S 而不是现在的 RepoConfig
    // * 支持 V 为 false 的情况？（低优先级，不确定这是否必要）
    // * 如何处理不同 workspaces 的同名 package name
    // * 如何处理无意多次指定同一个 package
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
    /// 每个 package 及其对应的检查命令
    pub fn pkg_checker_action<'p>(
        &self,
        pkgs: &[Package<'p>],
    ) -> Result<Vec<(Package<'p>, Expression)>> {
        let all = matches!(self.all, Some(Action::Perform(true)));

        let v = match &self.packages {
            Some(map) => {
                // check validity of packages names
                let layout: BTreeSet<_> = pkgs.iter().map(|p| p.name).collect();
                let input: BTreeSet<_> = map.keys().map(|s| s.as_str()).collect();
                let invalid: BTreeSet<_> = input.difference(&layout).copied().collect();
                let rest: BTreeSet<_> = layout.difference(&input).copied().collect();
                ensure!(
                    invalid.is_empty(),
                    "yaml 配置中不存在如下 packages：{invalid:?}；\n\
                     该仓库有如下 package names：{layout:?}；\n\
                     已经设置的 packages 有：{input:?}\n；\n\
                     你应该从剩余的 packages 中指定：{rest:?}",
                );

                let mut v = Vec::with_capacity(pkgs.len() * TOOLS);
                let layout: BTreeMap<_, _> = pkgs.iter().map(|&p| (p.name, p)).collect();

                // 指定的 packages
                for (name, config) in map {
                    let pkg = *layout.get(name.as_str()).unwrap(); // already checked
                    let inner_all = match config.all {
                        Some(Action::Perform(false)) => false,
                        _ => all,
                    };
                    v.extend(config.pkg_cmd(inner_all, &[pkg])?);
                }
                // 未指定的 packages
                for name in rest {
                    let pkg = *layout.get(name).unwrap(); // already checked
                    v.extend(self.pkg_cmd(all, &[pkg])?);
                }
                v
            }
            None => self.pkg_cmd(all, pkgs)?, // for all pkgs
        };

        // TODO: fix by package name and tool name
        Ok(v)
    }

    /// TODO: 暂时应用 fmt 和 clippy，其他工具待完成
    fn pkg_cmd<'p>(
        &self,
        all: bool,
        pkgs: &[Package<'p>],
    ) -> Result<Vec<(Package<'p>, Expression)>> {
        let mut v = Vec::with_capacity(pkgs.len() * TOOLS);

        match &self.fmt {
            Some(Action::Perform(true)) => {
                for &p in pkgs {
                    v.push((p, cargo_fmt(p.cargo_toml)));
                }
            }
            None if all => {
                for &p in pkgs {
                    v.push((p, cargo_fmt(p.cargo_toml)));
                }
            }
            Some(Action::Lines(lines)) => {
                for &p in pkgs {
                    for line in lines {
                        v.push((p, custom(line, p.cargo_toml)?));
                    }
                }
            }
            _ => (),
        }
        match &self.clippy {
            Some(Action::Perform(true)) => {
                for &p in pkgs {
                    v.push((p, cargo_clippy(p.cargo_toml)));
                }
            }
            None if all => {
                for &p in pkgs {
                    v.push((p, cargo_clippy(p.cargo_toml)));
                }
            }
            Some(Action::Lines(lines)) => {
                for &p in pkgs {
                    for line in lines {
                        v.push((p, custom(line, p.cargo_toml)?));
                    }
                }
            }
            _ => (),
        }

        Ok(v)
    }

    /// checker 及其操作（包括 packages 字段内的 checkers）；主要用于 check_tool_action
    fn checker_action(&self) -> Result<Vec<(CheckerTool, &Action)>> {
        use CheckerTool::*;
        let mut v = Vec::with_capacity(8);
        filter!(self, val:
            all => if let Action::Lines(lines) = val {
                bail!("暂不支持在 all 上指定任何命令，请删除 {lines:?} ");
            },
            fmt => v.push((Fmt, val)),
            clippy => v.push((Clippy, val)),
            miri => v.push((Miri, val)),
            semver_checks => v.push((SemverChecks, val)),
            lockbud => v.push((Lockbud, val)),
            packages => for config in val.values() {
                v.extend(config.checker_action()?);
            },
        );
        Ok(v)
    }

    /// 检查 action（尤其是自定义命令）是否与 checker 匹配
    fn check_tool_action(&self) -> Result<()> {
        self.checker_action()?
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
    Lines(Box<[String]>),
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
                    value => Action::Lines(value.lines().filter_map(no_comment).collect()),
                })
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl Action {
    /// 检查指定的每一条命令是否与工具匹配
    fn check(&self, tool: CheckerTool) -> Result<()> {
        match self {
            Action::Perform(_) => Ok(()),
            Action::Lines(lines) => {
                let name = tool.name();
                for line in &lines[..] {
                    ensure!(
                        line.contains(name),
                        "命令 `{line}` 与检查工具 `{name}` 不匹配"
                    );
                }
                Ok(())
            }
        }
    }
}
