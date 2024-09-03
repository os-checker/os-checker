use super::{uri::Uri, Config, Configs, RepoConfig};
use crate::Result;
use cargo_metadata::camino::Utf8Path;
use eyre::Context;
use indexmap::IndexMap;

#[allow(dead_code)]
impl Config {
    pub fn from_json(json: &str) -> Result<Config> {
        Ok(serde_json::from_str(json)?)
    }

    /// 序列化一个仓库配置
    pub fn from_json_path(json: &Utf8Path) -> Result<Config> {
        let json = std::fs::read_to_string(json)
            .with_context(|| format!("从 `{json}` 读取仓库列表失败！请输入正确的 json 路径。"))?;
        Config::from_json(&json)
    }
}

impl Configs {
    // b 覆盖 a
    pub fn merge(Configs(a): Self, Configs(b): Self) -> Self {
        let mut merge = Merge::with_capacity(a.len() + b.len());
        for Config { uri, config } in a.into_iter().chain(b) {
            merge.push_or_update(uri, config);
        }
        Configs(merge.into_configs())
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
