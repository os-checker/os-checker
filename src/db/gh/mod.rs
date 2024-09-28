//! ```bash
//! export user=os-checker repo=os-checker
//! export branch=$(gh api repos/os-checker/os-checker --jq ".default_branch")
//!
//! export meta="user: \"$user\", repo: \"$repo\", branch: \"$branch\""
//!
//! # {
//! #   "author": {
//! #     "date": "2024-09-28T04:58:37Z",
//! #     "email": "jiping_zhou@foxmail.com",
//! #     "name": "zjp-CN"
//! #   },
//! #   "branch": "main",
//! #   "committer": {
//! #     "date": "2024-09-28T04:58:37Z",
//! #     "email": "noreply@github.com",
//! #     "name": "GitHub"
//! #   },
//! #   "mes": "Merge pull request #99 from os-checker/feat/db\n\nfeat: 使用 redb 嵌入式数据库进行检查结果缓存",
//! #   "repo": "os-checker",
//! #   "sha": "da81840e786e7e2c71329c30058347a45a3a2536",
//! #   "user": "os-checker"
//! # }
//! gh api repos/os-checker/os-checker/branches/$branch --jq \
//!   "{$meta, sha: .commit.sha, mes: .commit.commit.message, author: .commit.commit.author, committer: .commit.commit.committer}"
//! ```

use crate::{Result, XString};
use duct::cmd;
use eyre::Context;
use musli::{Decode, Encode};
use serde::Deserialize;
use std::fmt;

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

#[derive(Debug, Deserialize, Encode, Decode)]
pub struct InfoRepo {
    #[musli(with = musli::serde)]
    user: XString,
    #[musli(with = musli::serde)]
    repo: XString,
    /// default branch
    #[musli(with = musli::serde)]
    branch: XString,
    latest_commit: LatestCommit,
}

#[derive(Debug, Deserialize, Encode, Decode)]
struct LatestCommit {
    sha: String,
    mes: String,
    author: Committer,
    committer: Committer,
}

#[derive(Deserialize, Encode, Decode)]
struct Committer {
    // store as unix timestemp milli
    #[serde(deserialize_with = "deserialize_date")]
    #[serde(rename(deserialize = "date"))]
    datetime: u64,
    #[musli(with = musli::serde)]
    email: String,
    #[musli(with = musli::serde)]
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

fn info_repo(user: &str, repo: &str) -> Result<InfoRepo> {
    let branch = default_branch(user, repo)?;
    let arg = format!("repos/os-checker/os-checker/branches/{branch}");
    let jq = format!(
        "
          {{
            user: \"{user}\", repo: \"{repo}\", branch: \"{branch}\", 
            latest_commit: {{
                sha: .commit.sha,
                mes: .commit.commit.message,
                author: .commit.commit.author,
                committer: .commit.commit.committer
            }}
          }}
        "
    );
    serde_json::from_str(&gh_api(arg, jq)?).with_context(|| "无法获取仓库最新提交信息")
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
