use crate::{
    db::{get_info, Db, InfoKeyValue},
    layout::Packages,
    Result,
};
use cargo_metadata::camino::{Utf8Path, Utf8PathBuf};
use eyre::Context;
use indexmap::IndexSet;
use itertools::Itertools;
use os_checker_types::db::ListTargets;
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use serde_json::Value;

pub mod cmd;

mod resolve;
pub use resolve::Resolve;

mod merge_from_json;
mod uri;

mod checker;
pub use checker::{CheckerTool, TOOLS};

mod deserialization;
pub use deserialization::{gen_schema, Features, RepoConfig, TargetEnv, TargetsSpecifed};

#[cfg(test)]
mod tests;

/// A repo and its checker configurations.
#[derive(Debug, Deserialize)]
#[serde(try_from = "serde_json::Value")]
pub struct Config {
    uri: uri::Uri,
    config: Box<RepoConfig>,
    db: Option<Db>,
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

    pub fn is_in_repos(&self, repos: &[&str]) -> bool {
        let key = self.uri.key();
        for &repo in repos {
            if key == repo {
                return true;
            }
        }
        // the config doesn't belong to any repo in the list, or the list is none
        false
    }

    pub fn set_db(&mut self, db: Option<Db>) {
        self.db = db;
    }

    pub fn db(&self) -> Option<&Db> {
        self.db.as_ref()
    }

    pub fn new_info(&self) -> Result<Box<InfoKeyValue>> {
        let user = self.user_name();
        let repo = self.repo_name();
        let config = &*self.config;
        get_info(user, repo, config.clone()).map(Box::new)
    }

    /// 解析该仓库所有 package 的检查执行命令
    pub fn resolve(&self, pkgs: &Packages) -> Result<Vec<Resolve>> {
        self.config
            .resolve(self.uri.key(), pkgs)
            .with_context(|| format!("解析 `{:?}` 仓库的检查命令出错", self.uri))
    }

    pub fn targets_specified(&self) -> TargetsSpecifed {
        self.config.targets_specified()
    }

    pub fn clean_repo_dir(&self) -> Result<()> {
        self.uri.clean_repo_dir()
    }

    pub fn list_targets(&self, pkgs: &Packages) -> Result<Vec<ListTargets>> {
        Ok(self
            .config
            .selected_pkgs(pkgs)?
            .into_iter()
            .map(|(pkg, info)| ListTargets {
                user: self.user_name().into(),
                repo: self.repo_name().into(),
                pkg: pkg.into(),
                targets: info.targets(),
            })
            .collect())
    }
}

impl TryFrom<Value> for Config {
    type Error = eyre::Error;

    #[instrument(level = "trace")]
    fn try_from(value: Value) -> Result<Self> {
        if let Value::Object(obj) = value {
            // assert_eq!(config.len(), 1);
            if let Some((repo, deserializer)) = obj.into_iter().next() {
                if let Ok(mut config) = RepoConfig::deserialize(deserializer) {
                    config.validate_checker_name(&repo)?;
                    config.sort_packages();
                    return Ok(Config {
                        uri: uri::uri(repo)?,
                        config: Box::new(config),
                        db: None,
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
    #[instrument(level = "trace")]
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// 序列化一个仓库配置
    #[instrument(level = "trace")]
    pub fn from_json_path(path: &Utf8Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("从 `{path}` 读取仓库列表失败！请输入正确的 json 路径。"))?;
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

    #[instrument(level = "trace")]
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

    pub fn check_given_repos(&self, repos: &[&str]) -> Result<()> {
        let mut set: IndexSet<_> = self.0.iter().map(|c| c.uri.key()).collect();
        set.sort_unstable();
        for repo in repos {
            ensure!(
                set.contains(repo),
                "{repo} is not in config repos:\n{set:?}"
            );
        }
        Ok(())
    }

    pub fn list_repos(&self) -> Vec<&str> {
        self.0.iter().map(|config| config.uri.key()).collect()
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

    #[instrument(level = "trace")]
    fn try_from(value: Value) -> Result<Self> {
        if let Value::Object(obj) = value {
            // assert_eq!(config.len(), 1);
            let mut v = obj
                .into_iter()
                .map(|(repo, deserializer)| {
                    let config =
                        RepoConfig::deserialize(deserializer).with_context(|| PARSE_JSON_ERROR)?;
                    config.validate_checker_name(&repo)?;
                    config.validate_skip_pkg_dir_globs(&repo)?;
                    debug!(?config);
                    Ok(Config {
                        uri: uri::uri(repo)?,
                        config: Box::new(config),
                        db: None,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            v.sort_by(|a, b| a.uri.cmp(&b.uri));
            return Ok(Configs(v));
        }
        bail!("{PARSE_JSON_ERROR}")
    }
}

const PARSE_JSON_ERROR: &str = r#"Should be an object like `{"user/repo": {...}}`"#;

#[test]
fn de_features() -> Result<()> {
    let json = r#"
{
  "guoweikang/osl": {
    "packages": {
      "osl": {
        "features": [
          "arceos",
          ""
        ]
      }
    }
  }
}"#;
    let config: Config = serde_json::from_str(json)?;
    dbg!(&config);

    Ok(())
}
