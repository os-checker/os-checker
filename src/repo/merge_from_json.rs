//! 从只含 "user/repo" 的 JSON 数组中合并到现有的 YAML 配置。
//! 这些数组默认以 `all: true` 方式应用所有检查，但如果在 YAML
//! 中单独配置了，则完全按照 YAML 的配置应用检查。
//! 如果 JSON 中不包含 YAML 配置的仓库，则也完全按照 YAML
//! 配置应用检查到该仓库。

use super::{
    uri::{uri, Uri},
    Action, Config, RepoConfig,
};
use crate::Result;
use cargo_metadata::camino::Utf8Path;
use eyre::Context;
use indexmap::IndexMap;

impl Config {
    fn from_json(json: &str) -> Result<Vec<Config>> {
        let repos: Vec<String> = serde_json::from_str(json).with_context(
            || r#"请输入只含 "user/repo" 的 JSON 数组（比如 `["user1/repo", "user2/repo"]`"#,
        )?;

        repos
            .into_iter()
            .map(|repo| {
                Ok(Config {
                    uri: uri(repo)?,
                    config: Box::new(RepoConfig {
                        all: Some(Action::Perform(true)),
                        ..Default::default()
                    }),
                })
            })
            .collect()
    }

    pub fn from_json_path(json: &Utf8Path) -> Result<Vec<Config>> {
        let json = std::fs::read_to_string(json)
            .with_context(|| format!("从 `{json}` 读取仓库列表失败！请输入正确的 json 路径。"))?;
        Config::from_json(&json)
    }

    pub fn merge_json_and_yaml(
        configs_json: Vec<Self>,
        configs_yaml: Vec<Self>,
    ) -> Result<Vec<Self>> {
        // let json = std::fs::read_to_string(json)
        //     .with_context(|| format!("从 `{json}` 读取仓库列表失败！请输入正确的 json 路径。"))?;
        // let config_from_json = Config::from_json(json)?;
        // let config_from_yaml = Config::from_yaml(yaml)?;

        let mut merge = Merge::with_capacity(configs_json.len() + configs_yaml.len());
        for Config { uri, config } in configs_json.into_iter().chain(configs_yaml) {
            merge.push_or_update(uri, config);
        }

        Ok(merge.into_configs())
    }
}

struct Merge {
    map: IndexMap<Uri, Box<RepoConfig>>,
}

impl Merge {
    fn with_capacity(cap: usize) -> Merge {
        Merge {
            map: IndexMap::with_capacity(cap),
        }
    }

    // 先后顺序很重要：后插入的 config 完全覆盖之前已有的 config
    fn push_or_update(&mut self, uri: Uri, config: Box<RepoConfig>) {
        if let Some(repo) = self.map.get_mut(&uri) {
            *repo = config;
        } else {
            self.map.insert(uri, config);
        }
    }

    fn into_configs(mut self) -> Vec<Config> {
        self.map.sort_keys();
        self.map
            .into_iter()
            .map(|(uri, config)| Config { uri, config })
            .collect()
    }
}

#[test]
fn merge_json_and_yaml_configs() -> Result<()> {
    let json = r#"["user1/repo", "user2/repo"]"#;
    let yaml = r#"
user1/repo:
  miri: false

user3/repo:
  all: true
"#;
    let configs = Config::merge_json_and_yaml(Config::from_json(json)?, Config::from_yaml(yaml)?)?;
    expect_test::expect![[r#"
        [
            Config {
                uri: Github(
                    "user1/repo",
                ),
                config: RepoConfig {
                    miri: Perform(
                        false,
                    ),
                },
            },
            Config {
                uri: Github(
                    "user2/repo",
                ),
                config: RepoConfig {
                    all: Perform(
                        true,
                    ),
                },
            },
            Config {
                uri: Github(
                    "user3/repo",
                ),
                config: RepoConfig {
                    all: Perform(
                        true,
                    ),
                },
            },
        ]
    "#]]
    .assert_debug_eq(&configs);
    Ok(())
}
