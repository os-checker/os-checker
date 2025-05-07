use super::{out::CacheLayout, CacheRepo, CacheRepoKey, CacheValue, Db};
use crate::{
    config::{RepoConfig, Uri},
    Result, XString,
};
use duct::cmd;
use eyre::Context;
use os_checker_types::db as out;
use serde::Deserialize;
use std::{cell::RefCell, fmt};

mod local;
mod type_conversion;

/// Needs CLIs like gh and jq.
/// GH_TOKEN like `GH_TOKEN: ${{ github.token }}` in Github Action.
fn gh_api(arg: String, jq: String) -> Result<String> {
    let _span = error_span!("gh_api", arg, jq).entered();
    cmd!("gh", "api", arg, "--jq", jq)
        .read()
        .with_context(|| "无法获取 Github API 数据")
}

fn default_branch(user: &str, repo: &str) -> Result<String> {
    let arg = format!("repos/{user}/{repo}");
    gh_api(arg, ".default_branch".to_owned())
}

#[derive(Debug, Clone)]
pub struct InfoKey {
    repo: CacheRepo,
    config: RepoConfig,
}

impl InfoKey {
    pub fn span(&self) -> tracing::span::EnteredSpan {
        error_span!("InfoKey", user = %self.repo.user, repo = %self.repo.repo).entered()
    }

    pub fn to_db_key(&self) -> out::InfoKey {
        self.clone().into()
    }
}

#[derive(Debug, Clone)]
pub struct Info {
    /// 该仓库的检查是否全部完成
    complete: bool,
    /// 缓存信息
    caches: Vec<CacheRepoKey>,
    /// 仓库最新提交信息
    latest_commit: LatestCommit,
}

impl Info {
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    pub fn get_cache_values(&self, db: &Db) -> Result<Vec<(&str, CacheValue)>> {
        let caches_len = self.caches.len();
        let mut v = Vec::with_capacity(caches_len);
        for key in &self.caches {
            let _span = key.span();
            match db.get_cache(&key.to_db_key())? {
                Some(cache) => v.push((key.pkg_name(), cache.into())),
                None => error!("info 存储了一个检查结果的键，但未找到对应的检查结果"),
            };
        }
        info!(caches_len);
        Ok(v)
    }

    pub fn to_db_value(&self) -> out::Info {
        self.clone().into()
    }
}

#[derive(Debug, Deserialize, Clone)]
struct LatestCommit {
    sha: String,
    mes: String,
    author: Committer,
    committer: Committer,
}

#[derive(Deserialize, Clone)]
struct Committer {
    // store as unix timestemp milli
    #[serde(deserialize_with = "deserialize_date")]
    #[serde(rename(deserialize = "date"))]
    datetime: u64,
    email: String,
    name: XString,
}

impl fmt::Debug for Committer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Committer")
            .field(
                "datetime",
                &super::parse_unix_timestamp_milli(self.datetime),
            )
            .field("email", &self.email)
            .field("name", &self.name)
            .finish()
    }
}

fn deserialize_date<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let dt = <&str>::deserialize(deserializer)?;
    Ok(parse_datetime(dt))
}

fn parse_datetime(dt: &str) -> u64 {
    const DESC: &[time::format_description::BorrowedFormatItem<'static>] =
        time::macros::format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z");
    let utc = time::PrimitiveDateTime::parse(dt, DESC)
        .unwrap()
        .assume_utc();
    let local = utc.to_offset(time::UtcOffset::from_hms(8, 0, 0).unwrap());
    super::unix_timestamp_milli(local)
}

fn info_repo(user: &str, repo: &str) -> Result<(String, LatestCommit)> {
    let branch = default_branch(user, repo)?;
    let arg = format!("repos/{user}/{repo}/branches/{branch}");
    let jq = "{
              sha: .commit.sha,
              mes: .commit.commit.message,
              author: .commit.commit.author,
              committer: .commit.commit.committer
          }";
    let last_commit = serde_json::from_str(&gh_api(arg, jq.to_owned())?)
        .with_context(|| "无法获取仓库最新提交信息")?;
    Ok((branch, last_commit))
}

/// Query latest commit sha via `gh api`, and return the key and value with empty caches.
pub fn get_info(uri: &Uri, config: RepoConfig) -> Result<InfoKeyValue> {
    let [user, repo] = [uri.user_name(), uri.repo_name()];
    let (branch, latest_commit) = if let Some(path) = uri.local_source() {
        info!(?uri, "Get info from local project");
        local::info_repo(path)?
    } else {
        info!(?uri, "Get info from Github API");
        info_repo(user, repo)?
    };
    let key = InfoKey {
        repo: CacheRepo::new_with_sha(user, repo, &latest_commit.sha, branch),
        config,
    };
    debug!(?key);
    let val = Info {
        complete: false,
        caches: vec![],
        latest_commit,
    };
    Ok(InfoKeyValue {
        key,
        val: RefCell::new(val),
    })
}

pub struct InfoKeyValue {
    key: InfoKey,
    /// 目前所有检查是单线程的，并且每个仓库是独立检查的
    val: RefCell<Info>,
}

impl InfoKeyValue {
    /// 校验远程仓库的 sha 与本地仓库的 sha 是否一致
    pub fn assert_eq_sha(&self, cache_repo: &CacheRepo) {
        self.key
            .repo
            .assert_eq_sha(cache_repo, "remote sha ≠ local sha");
    }

    pub fn get_from_db(&self, db: &Db) -> Result<Option<Info>> {
        db.get_info(&self.key.to_db_key())
            .map(|opt| opt.map(Info::from))
    }

    pub fn append_cache_key(&self, cache_key: &CacheRepoKey, db: &Db) -> Result<()> {
        let _span = self.key.span();
        let info = {
            let info = &mut self.val.borrow_mut();
            info.caches.push(cache_key.clone());
            info.to_db_value()
        };
        db.set_info(&self.key.to_db_key(), &info)
    }

    /// 所有实际检查完成，调用此函数
    pub fn set_complete(&self, db: &Db) -> Result<()> {
        let _span = self.key.span();
        let info = {
            let info = &mut self.val.borrow_mut();
            info.complete = true;
            info.to_db_value()
        };
        debug!(?self.key);
        db.set_info(&self.key.to_db_key(), &info)
    }

    pub fn set_layout_cache(&self, layout: CacheLayout, db: &Db) -> Result<()> {
        db.set_layout(&self.key.to_db_key(), &layout)
    }

    pub fn check_push_info_key(&self, db: &Db) -> Result<()> {
        db.check_push_info_key(self.key.clone().into())
    }
}

#[test]
fn github_date() {
    let dt = "2024-09-28T04:58:37Z";
    dbg!(parse_datetime(dt));
}

#[test]
fn get_default_branch() -> Result<()> {
    let user = "os-checker";
    let repo = "os-checker";
    dbg!(default_branch(user, repo)?, info_repo(user, repo)?);
    Ok(())
}
