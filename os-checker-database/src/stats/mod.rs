use crate::{utils::IndexMap, write_to_file, Result, ALL_TARGETS};
use ahash::{AHashMap, AHashSet};
use os_checker_types::JsonOutput;
use serde::{Deserialize, Serialize};

use crate::utils::{group_by, repo_cmdidx};

#[derive(Debug, Serialize, Deserialize)]
struct PassCountRepo {
    /// 无诊断的仓库数量
    pass: usize,
    /// 总仓库数量
    total: usize,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Key<'a> {
    repo: &'a str,
    target: &'a str,
}

pub struct PassCountRepos<'a> {
    map: AHashMap<Key<'a>, usize>,
}

impl<'a> PassCountRepos<'a> {
    pub fn new() -> Self {
        PassCountRepos {
            map: AHashMap::new(),
        }
    }

    pub fn update(&mut self, json: &'a JsonOutput) {
        // 预分配空间
        self.map.reserve(json.env.repos.len() * 4);

        // cmd 中包含所有仓库及其 target，无论是否有诊断
        for cmd in &json.cmd {
            let repo = &json.env.packages[cmd.package_idx].repo.repo;
            let target = &cmd.target_triple;
            let key = Key { repo, target };
            let count = cmd.count;
            self.map
                .entry(key)
                .and_modify(|c| *c += count)
                .or_insert(count);
        }
    }

    fn pass_count_repo(&self) -> Vec<(&'a str, PassCountRepo)> {
        // pass count for targets
        let mut v = Vec::with_capacity(16);

        let mut count_set = CountRepoSet::new();

        // All-Targets: total count for each repo
        let repo_count = self.map.iter().map(|(k, &c)| (k.repo, c)).fold(
            AHashMap::new(),
            |mut acc, (repo, c)| {
                acc.entry(repo).and_modify(|e| *e += c).or_insert(c);
                acc
            },
        );
        v.push((ALL_TARGETS, count_set.counting(repo_count.into_iter())));

        // specific targets：这里的总计数全部来自该 target，而不是所有被检查仓库数量
        let targets = group_by(&self.map, |(k, _)| k.target);
        for (target, repos) in targets {
            let iter = repos.iter().map(|(k, c)| (k.repo, **c));
            v.push((target, count_set.counting(iter)));
        }

        v
    }

    /// 只在获取所有数据之后调用此函数。
    pub fn write_to_file(&self) -> Result<()> {
        let mut pass_count_repo = self.pass_count_repo();
        pass_count_repo.sort_unstable_by(|a, b| {
            // 先按照 total 和 ratio 降序，然后按照名称升序
            let b_ratio = (b.1.pass as f32 / b.1.total as f32 * 1000.0) as u16;
            let a_ratio = (a.1.pass as f32 / a.1.total as f32 * 1000.0) as u16;
            (b.1.total, b_ratio, a.0).cmp(&(a.1.total, a_ratio, b.0))
        });

        // 写入每个 target 上通过数量
        for (target, pass_count) in &pass_count_repo {
            write_to_file("pass_count_repo", target, pass_count)?;
            info!(pass_count_repo = ?pass_count, "写入 pass_count_repo/{target}.json 成功");
        }

        // 写入一个包含所有 target 的数量文件（减少网络调用）
        let map = IndexMap::from_iter(pass_count_repo);
        write_to_file("pass_count_repo", "_targets_", &map)?;

        Ok(())
    }
}

struct CountRepoSet<'a> {
    /// zero count repos
    pass: AHashSet<&'a str>,
    /// all repos
    total: AHashSet<&'a str>,
}

impl<'a> CountRepoSet<'a> {
    fn new() -> Self {
        CountRepoSet {
            pass: AHashSet::with_capacity(128),
            total: AHashSet::with_capacity(128),
        }
    }

    fn counting(&mut self, iter: impl Iterator<Item = (&'a str, usize)>) -> PassCountRepo {
        self.pass.clear();
        self.total.clear();
        for (repo, count) in iter {
            self.total.insert(repo);
            if count == 0 {
                self.pass.insert(repo);
            }
        }
        PassCountRepo {
            pass: self.pass.len(),
            total: self.total.len(),
        }
    }
}
