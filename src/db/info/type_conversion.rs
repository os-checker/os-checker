use super::{Committer, Info, InfoKey, LatestCommit};
use os_checker_types::db as out;

// ********** CLI => os_checker_types **********

impl From<InfoKey> for out::InfoKey {
    fn from(value: InfoKey) -> Self {
        let InfoKey { repo, config } = value;
        Self {
            repo: repo.into(),
            config: config.into(),
        }
    }
}

impl From<Info> for out::Info {
    fn from(value: Info) -> Self {
        let Info {
            complete,
            caches,
            latest_commit,
        } = value;
        Self {
            complete,
            caches: caches.into_iter().map(|c| c.into()).collect(),
            latest_commit: latest_commit.into(),
        }
    }
}

impl From<LatestCommit> for out::LatestCommit {
    fn from(value: LatestCommit) -> Self {
        let LatestCommit {
            sha,
            mes,
            author,
            committer,
        } = value;
        Self {
            sha,
            mes,
            author: author.into(),
            committer: committer.into(),
        }
    }
}

impl From<Committer> for out::Committer {
    fn from(value: Committer) -> Self {
        let Committer {
            datetime,
            email,
            name,
        } = value;
        Self {
            datetime,
            email,
            name,
        }
    }
}

// ********** os_checker_types => CLI **********

impl From<out::InfoKey> for InfoKey {
    fn from(value: out::InfoKey) -> Self {
        let out::InfoKey { repo, config } = value;
        Self {
            repo: repo.into(),
            config: config.into(),
        }
    }
}

impl From<out::Info> for Info {
    fn from(value: out::Info) -> Self {
        let out::Info {
            complete,
            caches,
            latest_commit,
        } = value;
        Self {
            complete,
            caches: caches.into_iter().map(|c| c.into()).collect(),
            latest_commit: latest_commit.into(),
        }
    }
}

impl From<out::LatestCommit> for LatestCommit {
    fn from(value: out::LatestCommit) -> Self {
        let out::LatestCommit {
            sha,
            mes,
            author,
            committer,
        } = value;
        Self {
            sha,
            mes,
            author: author.into(),
            committer: committer.into(),
        }
    }
}

impl From<out::Committer> for Committer {
    fn from(value: out::Committer) -> Self {
        let out::Committer {
            datetime,
            email,
            name,
        } = value;
        Self {
            datetime,
            email,
            name,
        }
    }
}
