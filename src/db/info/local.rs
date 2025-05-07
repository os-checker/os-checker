//! Get info for local projects.
use super::{Committer, LatestCommit};
use crate::Result;
use duct::cmd;
use eyre::{Context, ContextCompat};
use os_checker_types::unix_timestamp_milli;
use time::format_description::well_known::Rfc2822;

pub fn info_repo(path: &str) -> Result<(String, LatestCommit)> {
    let branch = current_branch(path)?;
    let commit = latest_commit(path)?;
    Ok((branch, commit))
}

fn current_branch(path: &str) -> Result<String> {
    let branch = cmd!("git", "branch", "--show-current").dir(path).read()?;
    Ok(branch.trim().to_owned())
}

fn latest_commit(path: &str) -> Result<LatestCommit> {
    // git log -1 --pretty=tformat:"SHA: %H%nAuthor: %an <%ae>%nCommitter: %cn <%ce>%nCommit Header: %s"
    let output = cmd!(
        "git",
        "log",
        "-1",
        "--pretty=tformat:SHA: %H%nCommit Header: %s%nAuthor: %an%nAuthor Email: %ae%n\
            Author Date: %ad%nCommitter: %cn%nCommitter Email: %ce%nCommitter Date: %cd",
        "--date=rfc"
    )
    .dir(path)
    .read()?;

    // $ git log -1 --pretty=tformat:"SHA: %H%nCommit Header: %s%nAuthor: %an%nAuthor Email: %ae%nAuthor Date: %ad%nCommitter: %cn%nCommitter Email: %ce%nCommitter Date: %cd"
    // SHA: 4c603baeb747801449a7d20fff4a5be08624c4db
    // Commit Header: chore: remove verbose logs
    // Author: zjp
    // Author Email: jiping_zhou@foxmail.com
    // Author Date: Tue, 6 May 2025 22:44:25 +0800
    // Committer: zjp
    // Committer Email: jiping_zhou@foxmail.com
    // Committer Date: Tue, 6 May 2025 22:44:25 +0800
    let lines: Vec<_> = output.trim().lines().collect();
    ensure!(lines.len() == 8, "lines={lines:?} doesn't have 8 lines");
    let get = |idx: usize, prefix: &str| {
        lines[idx]
            .strip_prefix(prefix)
            .with_context(|| format!("{:?} doesn't have prefix {prefix:?}", lines[idx]))
            .map(String::from)
    };
    Ok(LatestCommit {
        sha: get(0, "SHA: ")?,
        mes: get(1, "Commit Header: ")?,
        author: Committer {
            name: get(2, "Author: ")?.into(),
            email: get(3, "Author Email: ")?,
            datetime: {
                let date = get(4, "Author Date: ")?;
                unix_timestamp_milli(
                    time::OffsetDateTime::parse(&date, &Rfc2822)
                        .with_context(|| format!("{date:?} is not parsed"))?,
                )
            },
        },
        committer: Committer {
            name: get(5, "Committer: ")?.into(),
            email: get(6, "Committer Email: ")?,
            datetime: unix_timestamp_milli(time::OffsetDateTime::parse(
                &get(7, "Committer Date: ")?,
                &Rfc2822,
            )?),
        },
    })
}

#[test]
fn local_info_repo() {
    let (branch, commit) = info_repo(".").unwrap();
    dbg!(branch, commit);
}
