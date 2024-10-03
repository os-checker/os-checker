use os_checker_types::db as out;

impl From<super::InfoKey> for out::InfoKey {
    fn from(value: super::InfoKey) -> Self {
        let super::InfoKey { repo, config } = value;
        out::InfoKey {
            repo: repo.into(),
            config: config.into(),
        }
    }
}

impl From<super::Info> for out::Info {
    fn from(value: super::Info) -> Self {
        let super::Info {
            complete,
            caches,
            latest_commit,
        } = value;
        out::Info {
            complete,
            caches: caches.into_iter().map(|c| c.into()).collect(),
            latest_commit: latest_commit.into(),
        }
    }
}

impl From<super::LatestCommit> for out::LatestCommit {
    fn from(value: super::LatestCommit) -> Self {
        let super::LatestCommit {
            sha,
            mes,
            author,
            committer,
        } = value;
        out::LatestCommit {
            sha,
            mes,
            author: author.into(),
            committer: committer.into(),
        }
    }
}

impl From<super::Committer> for out::Committer {
    fn from(value: super::Committer) -> Self {
        let super::Committer {
            datetime,
            email,
            name,
        } = value;
        out::Committer {
            datetime,
            email,
            name,
        }
    }
}
