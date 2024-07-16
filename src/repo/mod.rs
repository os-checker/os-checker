use serde::{de, Deserialize};
use std::collections::HashMap;

/// Configuration for single repo.
#[derive(Debug, Deserialize)]
pub struct RepoConfig {
    fmt: CheckerAction,
    clippy: CheckerAction,
    miri: CheckerAction,
    lockbud: CheckerAction,
    all: CheckerAction,
}

/// An optional action for a checker.
/// If there is no checker specified, the value is None.
pub type CheckerAction = Option<Action>;

/// Action specified for a checker.
// #[derive(Debug, Deserialize)]
// #[serde(untagged)]
// pub enum Action {
//     Perform(bool),
//     Steps(Steps),
// }
//
#[derive(Debug)]
pub enum Action {
    Perform(bool),
    Steps(Box<[String]>),
}

impl<'de> Deserialize<'de> for Action {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Action;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("A boolean, string or lines of string.")
            }

            fn visit_str<E>(self, value: &str) -> Result<Action, E>
            where
                E: de::Error,
            {
                let value = value.trim();
                Ok(match value {
                    "true" => Action::Perform(true),
                    "false" => Action::Perform(false),
                    value => Action::Steps(value.lines().map(String::from).collect()),
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
    cargo miri --test a
  semver-checks: false

user/repo: 
  all: true
";
    let parsed: HashMap<String, RepoConfig> = marked_yaml::from_yaml(0, yaml).unwrap();
    let expected = expect_test::expect![[r#"
        {
            "user/repo": RepoConfig {
                fmt: None,
                clippy: None,
                miri: None,
                lockbud: None,
                all: Some(
                    Perform(
                        true,
                    ),
                ),
            },
            "os-checker/os-checker": RepoConfig {
                fmt: Some(
                    Perform(
                        true,
                    ),
                ),
                clippy: Some(
                    Steps(
                        [
                            "cargo clippy -F a,b,c",
                        ],
                    ),
                ),
                miri: Some(
                    Steps(
                        [
                            "cargo miri --test a",
                        ],
                    ),
                ),
                lockbud: None,
                all: None,
            },
        }
    "#]];
    expected.assert_debug_eq(&parsed);
}
