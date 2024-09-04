use crate::{cli::repos_base_dir, utils::git_clone, Result, XString};
use cargo_metadata::camino::Utf8PathBuf;
use eyre::ContextCompat;
use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;

#[derive(Debug)]
pub enum UriTag {
    Github(String),
    Url(String),
    Local(Utf8PathBuf),
}

pub struct Uri {
    /// 代码库的来源
    tag: UriTag,
    /// 代码库的作者（解析自 key）
    user: XString,
    /// 代码库的名字（解析自 key）
    repo: XString,
    /// 暂时用于临时测试存放需要下载的代码库
    #[cfg(test)]
    _local_tmp_dir: Option<tempfile::TempDir>,
    /// JSON config 中表示代码库来源的键
    key: String,
}

impl Serialize for Uri {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.key)
    }
}

impl std::fmt::Debug for Uri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.tag.fmt(f)
    }
}

/// 由于 IndexMap 需要 Eq + Ord + Hash，在合并多个配置文件时，只看 key
impl PartialEq<Self> for Uri {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl Eq for Uri {}
impl PartialOrd for Uri {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Uri {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}
impl std::hash::Hash for Uri {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl Uri {
    /// 获取该代码库的本地路径：如果指定 Github 或者 Url，则调用 git 命令下载
    pub fn local_root_path_with_git_clone(&mut self) -> Result<Utf8PathBuf> {
        let url = match &self.tag {
            UriTag::Github(user_repo) => format!("https://github.com/{user_repo}.git"),
            UriTag::Url(url) => url.clone(),
            UriTag::Local(p) => return Ok(p.clone()),
        };

        // NOTE: 测试需要 git clone 的代码库时采用临时目录，非测试则直接放入当前目录下
        #[cfg(test)]
        let target_dir = &{
            let dir = tempfile::tempdir()?;
            let target = Utf8Path::from_path(dir.path()).unwrap().join(&*self.repo);
            self._local_tmp_dir = Some(dir);
            target
        };
        #[cfg(not(test))]
        let target_dir = &{
            let mut dir = repos_base_dir();
            dir.push(self.repo.as_str());
            dir
        };

        debug!(self.key, "git clone {url} {target_dir}");
        let (_, time_elapsed_ms) = git_clone(target_dir, &url)?;
        debug!(self.key, time_elapsed_ms);

        Ok(target_dir.to_owned())
    }

    pub fn repo_name(&self) -> &str {
        &self.repo
    }

    pub fn user_name(&self) -> &str {
        &self.user
    }

    pub fn key(&self) -> &str {
        &self.key
    }
}

static USER_REPO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(.*/)*(?P<user>.*?)/(?P<repo>.*?)(\.git)?$"#).unwrap());

pub fn uri(key: String) -> Result<Uri> {
    let ((user, repo), tag) = match key.strip_prefix("file://") {
        Some(path) => {
            let path = Utf8PathBuf::from(path);
            (
                user_repo(path.canonicalize_utf8()?.as_str())?,
                UriTag::Local(path),
            )
        }
        None => {
            let tag = match key.matches('/').count() {
                0 => bail!(
                    "{key} 不是正确的代码库来源；请指定以下一种格式：\
                 `file://localpath`；github 的 `user/repo`；完整的 git 仓库地址"
                ),
                1 => UriTag::Github(key.as_str().into()),
                _ => UriTag::Url(key.as_str().into()),
            };
            (user_repo(&key)?, tag)
        }
    };
    Ok(Uri {
        tag,
        user,
        repo,
        key,
        #[cfg(test)]
        _local_tmp_dir: None,
    })
}

fn user_repo(key: &str) -> Result<(XString, XString)> {
    let f = || format!("无法从 `{key}` 中解析 user/repo");
    let cap = USER_REPO.captures(key).with_context(f)?;
    let user = cap.name("user").with_context(f)?.as_str().into();
    let repo = cap.name("repo").with_context(f)?.as_str().into();
    Ok((user, repo))
}
