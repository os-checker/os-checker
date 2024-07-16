#![allow(unused)]
use serde::{de, Deserialize, Deserializer};
use std::{collections::BTreeMap, fmt};

#[derive(Debug)]
pub struct Config {
    repo: String,
    config: RepoConfig,
}

/// Configuration for single repo.
#[derive(Deserialize)]
pub struct RepoConfig {
    all: CheckerAction,
    fmt: CheckerAction,
    clippy: CheckerAction,
    miri: CheckerAction,
    lockbud: CheckerAction,
}

impl fmt::Debug for RepoConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("RepoConfig");
        if let Some(val) = &self.all {
            s.field("all", val);
        }
        if let Some(val) = &self.fmt {
            s.field("fmt", val);
        }
        if let Some(val) = &self.clippy {
            s.field("clippy", val);
        }
        if let Some(val) = &self.miri {
            s.field("miri", val);
        }
        if let Some(val) = &self.lockbud {
            s.field("lockbud", val);
        }
        s.finish()
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
                let value = value.trim();
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

#[test]
fn test_parse() {
    let yaml = "
os-checker/os-checker:
  fmt: true
  clippy: cargo clippy -F a,b,c
  miri: |
    # this is a comment line
    cargo miri run # a comment
    cargo miri test -- a_test_fn
  semver-checks: false

user/repo: 
  all: true
";
    let parsed: BTreeMap<String, RepoConfig> = marked_yaml::from_yaml(0, yaml).unwrap();
    let parsed: Vec<_> = parsed
        .into_iter()
        .map(|(repo, config)| Config { repo, config })
        .collect();
    let expected = expect_test::expect![[r#"
        [
            Config {
                repo: "os-checker/os-checker",
                config: RepoConfig {
                    fmt: Perform(
                        true,
                    ),
                    clippy: Steps(
                        [
                            "cargo clippy -F a,b,c",
                        ],
                    ),
                    miri: Steps(
                        [
                            "cargo miri run",
                            "cargo miri test -- a_test_fn",
                        ],
                    ),
                },
            },
            Config {
                repo: "user/repo",
                config: RepoConfig {
                    all: Perform(
                        true,
                    ),
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&parsed);
}
