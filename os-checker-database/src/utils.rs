use os_checker_types::JsonOutput;
use serde::{Deserialize, Serialize};

// pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
pub use eyre::Result;
pub type IndexMap<K, V> = indexmap::IndexMap<K, V, ahash::RandomState>;

pub fn new_map_with_cap<K, V>(cap: usize) -> IndexMap<K, V> {
    IndexMap::<_, _>::with_capacity_and_hasher(cap, ahash::RandomState::new())
}

pub fn group_by<K, V, I, F>(iter: I, f: F) -> std::collections::HashMap<K, Vec<V>>
where
    K: std::hash::Hash + Eq,
    I: IntoIterator<Item = V>,
    F: FnMut(&V) -> K,
{
    itertools::Itertools::into_group_map_by(iter.into_iter(), f)
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct UserRepo<'a> {
    pub user: &'a str,
    pub repo: &'a str,
}

pub fn repo_pkgidx(json: &JsonOutput, pkg_idx: usize) -> UserRepo {
    let repo = &json.env.packages[pkg_idx].repo;
    UserRepo {
        user: &repo.user,
        repo: &repo.repo,
    }
}

pub fn repo_cmdidx(json: &JsonOutput, cmd_idx: usize) -> UserRepo {
    let pkg_idx = json.cmd[cmd_idx].package_idx;
    repo_pkgidx(json, pkg_idx)
}

pub fn pkg_cmdidx(json: &JsonOutput, cmd_idx: usize) -> UserRepoPkg {
    let pkg_idx = json.cmd[cmd_idx].package_idx;
    let package_repo = &json.env.packages[pkg_idx];
    let repo = &package_repo.repo;
    UserRepoPkg {
        user: &repo.user,
        repo: &repo.repo,
        pkg: &package_repo.name,
    }
}

pub fn target_cmdidx(json: &JsonOutput, cmd_idx: usize) -> &str {
    &json.cmd[cmd_idx].target_triple
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct UserRepoPkg<'a> {
    pub user: &'a str,
    pub repo: &'a str,
    pub pkg: &'a str,
}

impl<'a> UserRepoPkg<'a> {
    pub fn into_repo(self) -> UserRepo<'a> {
        let Self { user, repo, .. } = self;
        UserRepo { user, repo }
    }
}

#[cfg(test)]
pub fn ui_json() -> JsonOutput {
    let file = std::fs::File::open("ui.json").unwrap();
    serde_json::from_reader(std::io::BufReader::new(file)).unwrap()
}
