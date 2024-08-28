use crate::{
    layout::{Packages, Pkg},
    Result,
};
use cargo_metadata::camino::{Utf8Path, Utf8PathBuf};
use eyre::Context;
use indexmap::{IndexMap, IndexSet};
use serde::{de, Deserialize, Deserializer, Serialize};
use std::fmt;

mod cmd;
use cmd::*;

mod uri;
mod validate;
pub use validate::Resolve;

#[cfg(test)]
mod tests;

/// A repo and its checker configurations.
#[derive(Debug)]
pub struct Config {
    uri: uri::Uri,
    config: Box<RepoConfig>,
}

impl Config {
    /// 解析 yaml 配置文件
    pub fn from_yaml(yaml: &str) -> Result<Vec<Config>> {
        let mut parsed: IndexMap<String, Box<RepoConfig>> = marked_yaml::from_yaml(0, yaml)
            .with_context(|| "仓库配置解析错误，请检查 yaml 格式或者内容是否正确")?;
        // 按仓库名排序
        parsed.sort_unstable_keys();
        for val in parsed.values_mut() {
            if let Some(pkgs) = &mut val.packages {
                // 按 package 名排序
                pkgs.sort_unstable_keys();
            }
        }
        parsed
            .into_iter()
            .map(|(key, config)| {
                (Config {
                    uri: uri::uri(key)?,
                    config,
                })
                .check()
            })
            .collect()
    }

    pub fn from_path<'a>(path: impl Into<&'a Utf8Path>) -> Result<Vec<Config>> {
        let path = path.into();
        let yaml = std::fs::read_to_string(path)
            .with_context(|| format!("从 `{path}` 读取配置内容失败！请输入正确的 yaml 路径。"))?;
        Config::from_yaml(&yaml)
    }

    /// 检查命令与工具是否匹配
    ///
    /// TODO: 检查自定义命令中的 target、features 和 RUSTFLAGS（需要与 layout 信息进行覆盖或者合并）
    fn check(self) -> Result<Config> {
        self.config.check_tool_action()?;
        Ok(self)
    }

    /// 获取该代码库的本地路径：如果指定 Github 或者 Url，则调用 git clone 命令下载
    pub fn local_root_path_with_git_clone(&mut self) -> Result<Utf8PathBuf> {
        self.uri.local_root_path_with_git_clone()
    }

    pub fn repo_name(&self) -> &str {
        self.uri.repo_name()
    }

    pub fn user_name(&self) -> &str {
        self.uri.user_name()
    }

    /// 解析该仓库所有 package 的检查执行命令
    pub fn resolve(&self, pkgs: &Packages) -> Result<Vec<Resolve>> {
        self.config
            .pkg_checker_action(pkgs)
            .with_context(|| format!("解析 `{:?}` 仓库的检查命令出错", self.uri))
    }
}

const TOOLS: usize = 4; // 目前支持的检查工具数量

/// 检查工具
#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
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
    packages: Option<IndexMap<String, RepoConfig>>,
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
    fn pkg_checker_action(&self, pkgs: &Packages) -> Result<Vec<Resolve>> {
        let all = matches!(self.all, Some(Action::Perform(true)));

        let mut v = match &self.packages {
            Some(map) => {
                // check validity of packages names
                let layout = pkgs.package_set();
                let mut input: IndexSet<_> = map.keys().map(|s| s.as_str()).collect();
                input.sort_unstable();
                let invalid: IndexSet<_> = input.difference(&layout).copied().collect();
                let rest: IndexSet<_> = layout.difference(&input).copied().collect();
                ensure!(
                    invalid.is_empty(),
                    "yaml 配置中不存在如下 packages：{invalid:?}；\n\
                     该仓库有如下 package names：{layout:?}；\n\
                     已经设置的 packages 有：{input:?}\n；\n\
                     你应该从剩余的 packages 中指定：{rest:?}",
                );

                let mut v = Vec::with_capacity(pkgs.len() * TOOLS);
                // let layout: BTreeMap<_, _> = pkgs.iter().map(|&p| (p.name, p)).collect();

                // 指定的 packages
                for (name, config) in map {
                    let inner_all = match config.all {
                        Some(Action::Perform(false)) => false,
                        _ => all,
                    };
                    v.extend(config.pkg_cmd(inner_all, &pkgs.single_vec_of_pkg(name))?);
                }
                // 未指定的 packages
                for name in rest {
                    v.extend(self.pkg_cmd(all, &pkgs.single_vec_of_pkg(name))?);
                }
                v
            }
            None => self.pkg_cmd(all, &pkgs.all_vec_of_pkg())?, // for all pkgs
        };

        v.sort_unstable_by(|a, b| (&a.pkg_name, a.checker).cmp(&(&b.pkg_name, b.checker)));
        Ok(v)
    }

    /// TODO: 暂时应用 fmt 和 clippy，其他工具待完成
    fn pkg_cmd(&self, all: bool, pkgs: &[Pkg]) -> Result<Vec<Resolve>> {
        use CheckerTool::*;
        let mut v = Vec::with_capacity(pkgs.len() * TOOLS);

        match &self.fmt {
            Some(Action::Perform(true)) => Resolve::fmt(pkgs, &mut v),
            None if all => Resolve::fmt(pkgs, &mut v),
            Some(Action::Lines(lines)) => Resolve::custom(pkgs, lines, Fmt, &mut v)?,
            _ => (),
        }
        match &self.clippy {
            Some(Action::Perform(true)) => Resolve::clippy(pkgs, &mut v),
            None if all => Resolve::clippy(pkgs, &mut v),
            Some(Action::Lines(lines)) => Resolve::custom(pkgs, lines, Clippy, &mut v)?,
            _ => (),
        }
        match &self.lockbud {
            Some(Action::Perform(true)) => Resolve::lockbud(pkgs, &mut v),
            None if all => Resolve::lockbud(pkgs, &mut v),
            Some(Action::Lines(lines)) => Resolve::custom(pkgs, lines, Lockbud, &mut v)?,
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
