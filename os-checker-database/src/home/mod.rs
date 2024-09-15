// [
//   {
//     "key": 0,
//     "data": {
//       "user": "kern-crates",
//       "repo": "ByteOS",
//       "total_count": 18,
//       "Cargo": 2,
//       "Clippy(Error)": 1,
//       "Clippy(Warn)": 15
//     },
//     "children": [
//       {
//         "key": 1,
//         "data": {
//           "user": "kern-crates",
//           "repo": "ByteOS",
//           "package": "kernel",
//           "total_count": 18,
//           "Cargo": 2,
//           "Clippy(Error)": 1,
//           "Clippy(Warn)": 15
//         }
//       }
//     ]
//   }
// ]

use crate::{
    utils::{
        group_by, new_map_with_cap, pkg_cmdidx, repo_cmdidx, target_cmdidx, IndexMap, UserRepo,
        UserRepoPkg,
    },
    Result,
};
use camino::Utf8Path;
use os_checker_types::{Data as RawData, JsonOutput, Kind};
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

pub fn all_targets(json: &JsonOutput) -> Vec<NodeRepo> {
    let data: Vec<_> = json.data.iter().collect();
    inner(json, &data)
}

pub fn split_by_target(json: &JsonOutput) -> Vec<(&str, Vec<NodeRepo>)> {
    let group_by_target = group_by(&json.data, |d| target_cmdidx(json, d.cmd_idx));
    let mut v = Vec::with_capacity(group_by_target.len());

    for (target, data) in group_by_target {
        v.push((target, inner(json, &data)));
    }
    v
}

fn inner<'a>(json: &'a JsonOutput, data: &[&RawData]) -> Vec<NodeRepo<'a>> {
    // 按照 repo 分组
    let group_by_repo = group_by(data, |d| repo_cmdidx(json, d.cmd_idx));
    let mut nodes = Vec::with_capacity(group_by_repo.len());

    for (repo, data_repo) in group_by_repo {
        // 按照 pkg 分组
        let group_by_pkg = group_by(data_repo, |d| pkg_cmdidx(json, d.cmd_idx));
        let mut children = Vec::with_capacity(group_by_pkg.len());

        for (pkg, data) in group_by_pkg {
            // 按照 kind 分组
            let group_by_kind = group_by(data, |d| d.kind);
            let mut map = new_map_with_cap(group_by_kind.len());
            map.extend(group_by_kind.into_iter().map(|(kind, v)| (kind, v.len())));
            // 按照 count 降序排序
            map.sort_unstable_by(|_, a, _, b| b.cmp(a));
            let count = Count::new(map);
            let total_count = count.total_count();
            let node_pkg = NodePkg {
                key: 0,
                data: NodePkgData {
                    pkg,
                    total_count,
                    count,
                },
            };
            children.push(node_pkg);
        }

        // children 按照计数降序、pkg 升序排列（我们知道这里的 user 和 repo 是一定相同的）
        children.sort_unstable_by(|a, b| {
            (b.data.total_count, a.data.pkg.pkg).cmp(&(a.data.total_count, b.data.pkg.pkg))
        });

        let mut count = Count::empty();
        count.update(children.iter().map(|c| &c.data.count));
        let total_count = count.total_count();
        let node = NodeRepo {
            key: 0,
            data: NodeRepoData {
                repo,
                total_count,
                count,
            },
            children,
        };
        nodes.push(node);
    }

    sort_by_count(&mut nodes);
    update_key(&mut nodes);

    nodes
}

// 仓库按照计数降序、user/repo 升序排列。
// 此函数适用于单个 ui.json，也适用于合并 batch。
fn sort_by_count(nodes: &mut [NodeRepo]) {
    nodes.sort_unstable_by(|a, b| {
        (b.data.total_count, a.data.repo).cmp(&(a.data.total_count, b.data.repo))
    });
}

/// 设置 repo 和 pkg 的 key。
/// 这个函数是必要的，因为重新按照 count 排序导致无法在创建节点实例的时候按顺序确定 key；
/// 此外，在合并 batch 的时候，key 需要重新生成。
fn update_key(nodes: &mut [NodeRepo]) {
    let mut key = 0;
    for repo in nodes {
        repo.key = key;
        key += 1;

        for pkg in &mut repo.children {
            pkg.key = key;
            key += 1;
        }
    }
}

/// 读取 src_dir 的所有 JSON，合并成一个新的 JSON，并写到 target_dir。
/// 新 JSON 的文件名取自 src_dir 的目录名。
#[instrument(level = "trace")]
pub fn write_batch(src_dir: &Utf8Path, target_dir: &Utf8Path) -> Result<()> {
    let vec_bytes = crate::json_paths(src_dir.as_str())?
        .into_iter()
        .map(std::fs::read)
        .collect::<Result<Vec<_>, _>>()?;

    let mut batch_nodes = Vec::<NodeRepo>::with_capacity(128);
    for bytes in &vec_bytes {
        batch_nodes.extend(serde_json::from_slice::<Vec<NodeRepo>>(bytes)?);
    }

    sort_by_count(&mut batch_nodes);
    update_key(&mut batch_nodes);

    let name = src_dir.file_name().unwrap();
    let path = target_dir.join(format!("{name}.json"));
    let file = std::fs::File::create(&path)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer(writer, &batch_nodes)?;

    info!("成功把 batch home 合并: src_dir={src_dir} merged={path}");
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeRepo<'a> {
    key: usize,
    #[serde(borrow)]
    data: NodeRepoData<'a>,
    children: Vec<NodePkg<'a>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NodeRepoData<'a> {
    #[serde(flatten)]
    #[serde(borrow)]
    repo: UserRepo<'a>,
    total_count: usize,
    #[serde(flatten)]
    count: Count,
}

#[derive(Debug, Serialize, Deserialize)]
struct NodePkg<'a> {
    key: usize,
    #[serde(borrow)]
    data: NodePkgData<'a>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NodePkgData<'a> {
    #[serde(flatten)]
    #[serde(borrow)]
    pkg: UserRepoPkg<'a>,
    total_count: usize,
    #[serde(flatten)]
    count: Count,
}

type CountInner = IndexMap<Kind, usize>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct Count {
    map: CountInner,
}

impl Count {
    fn empty() -> Self {
        // FIXME: 在 os_checker_types 中定义这个数量
        Count {
            map: new_map_with_cap(10),
        }
    }

    fn new(map: CountInner) -> Self {
        Self { map }
    }

    fn merge(&mut self, other: &Self) {
        for (&kind, &count) in &other.map {
            self.map
                .entry(kind)
                .and_modify(|c| *c += count)
                .or_insert(count);
        }
    }

    fn update<'a, Iter>(&mut self, iter: Iter)
    where
        Iter: Iterator<Item = &'a Self>,
    {
        iter.for_each(|c| self.merge(c));
    }

    fn total_count(&self) -> usize {
        self.map.values().sum()
    }
}
