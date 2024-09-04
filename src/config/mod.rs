use crate::{layout::Packages, Result};
use cargo_metadata::camino::{Utf8Path, Utf8PathBuf};
use eyre::Context;
use itertools::Itertools;
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};

mod cmd;
use cmd::*;

mod uri;
mod validate;
use serde_json::Value;
pub use validate::Resolve;

mod merge_from_json;

mod checker;
pub use checker::{CheckerTool, TOOLS};

mod deserialization;
pub use deserialization::RepoConfig;

#[cfg(test)]
mod tests;

/// A repo and its checker configurations.
#[derive(Debug, Deserialize)]
#[serde(try_from = "serde_json::Value")]
pub struct Config {
    uri: uri::Uri,
    config: Box<RepoConfig>,
}

impl Config {
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
            .resolve(self.uri.key(), pkgs)
            .with_context(|| format!("解析 `{:?}` 仓库的检查命令出错", self.uri))
    }
}

impl TryFrom<Value> for Config {
    type Error = eyre::Error;

    fn try_from(value: Value) -> Result<Self> {
        if let Value::Object(obj) = value {
            // assert_eq!(config.len(), 1);
            if let Some((repo, deserializer)) = obj.into_iter().next() {
                if let Ok(config) = RepoConfig::deserialize(deserializer) {
                    config.validate_checker_name(&repo)?;
                    return Ok(Config {
                        uri: uri::uri(repo)?,
                        config: Box::new(config),
                    });
                }
            }
        }
        bail!("{PARSE_JSON_ERROR}")
    }
}

impl Serialize for Config {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&self.uri, &*self.config)?;
        map.end()
    }
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "serde_json::Value")]
pub struct Configs(Vec<Config>);

impl Configs {
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// 序列化一个仓库配置
    pub fn from_json_path<'p>(path: impl Into<&'p Utf8Path>) -> Result<Self> {
        let path = path.into();
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("从 `{path}` 读取仓库列表失败！请输入正确的 json 路径。"))?;
        // FIXME: json not json array
        Self::from_json(&json)
    }

    pub fn into_inner(self) -> Vec<Config> {
        self.0
    }

    fn chunk(self, size: usize) -> Vec<Self> {
        if size == 0 {
            return vec![self];
        }
        self.0
            .into_iter()
            .chunks(size)
            .into_iter()
            .map(|chunk| Self(chunk.collect()))
            .collect()
    }

    pub fn batch(self, size: usize, dir: &Utf8Path) -> Result<()> {
        use std::fmt::Write;
        let mut path = Utf8PathBuf::from(dir);

        if !path.exists() {
            std::fs::create_dir_all(&mut path)?;
            trace!(%path, "successfully created the batch directory");
        }

        let mut file_name = String::new();
        let chunks = self.chunk(size);

        for (idx, configs) in chunks.into_iter().enumerate() {
            file_name.clear();
            write!(&mut file_name, "batch_{}.json", idx + 1).unwrap();
            path.push(&file_name);

            let writer = std::fs::File::create(&path)?;
            serde_json::to_writer_pretty(writer, &configs)?;
            trace!(%path, "successfully wrote a batch json config");
            path.pop();
        }

        Ok(())
    }
}

impl Serialize for Configs {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let v = &self.0;
        let mut map = serializer.serialize_map(Some(v.len()))?;
        for config in v {
            map.serialize_entry(config.uri.key(), &*config.config)?;
        }
        map.end()
    }
}

impl TryFrom<Value> for Configs {
    type Error = eyre::Error;

    fn try_from(value: Value) -> Result<Self> {
        if let Value::Object(obj) = value {
            // assert_eq!(config.len(), 1);
            let mut v = obj
                .into_iter()
                .map(|(repo, deserializer)| {
                    let config =
                        RepoConfig::deserialize(deserializer).with_context(|| PARSE_JSON_ERROR)?;
                    config.validate_checker_name(&repo)?;
                    Ok(Config {
                        uri: uri::uri(repo)?,
                        config: Box::new(config),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            v.sort_by(|a, b| a.uri.cmp(&b.uri));
            Ok(Configs(v))
        } else {
            bail!("{PARSE_JSON_ERROR}")
        }
    }
}

const PARSE_JSON_ERROR: &str = r#"Should be an object like `{"user/repo": {...}}`"#;