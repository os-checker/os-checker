//! 启发式了解项目的 Rust packages 组织结构。

use crate::{output::Norun, utils::walk_dir, Result, XString};
use cargo_metadata::{
    camino::{Utf8Path, Utf8PathBuf},
    Metadata, MetadataCommand,
};
use indexmap::{IndexMap, IndexSet};
use std::{collections::BTreeMap, fmt};

#[cfg(test)]
mod tests;

/// Target triple list and cargo check diagnostics.
mod targets;
use targets::PackageInfo;

mod detect_targets;
pub use detect_targets::RustToolchain;

/// 寻找仓库内所有 Cargo.toml 所在的路径
fn find_all_cargo_toml_paths(repo_root: &str, dirs_excluded: &[&str]) -> Vec<Utf8PathBuf> {
    let mut cargo_tomls = walk_dir(repo_root, 10, dirs_excluded, |file_path| {
        let file_name = file_path.file_name()?;
        // 只搜索 Cargo.toml 文件
        if file_name == "Cargo.toml" {
            Some(file_path)
        } else {
            None
        }
    });

    cargo_tomls.sort_unstable();
    cargo_tomls
}

type Workspaces = BTreeMap<Utf8PathBuf, Metadata>;

/// 解析所有 Cargo.toml 所在的 Package 的 metadata 来获取仓库所有的 Workspaces
fn parse(cargo_tomls: &[Utf8PathBuf]) -> Result<Workspaces> {
    let mut map = BTreeMap::new();
    for cargo_toml in cargo_tomls {
        // 暂时不解析依赖的原因：
        // * 不需要依赖信息
        // * 加快的解析速度
        // * 如何处理 features? features 会影响依赖吗？（待确认）
        //
        // 需要解析依赖的原因：
        // * 从 `[target.'cfg(...)'.*dependencies]` 中搜索 target：注意，如果这一条会比较难，因为
        //   有可能它为 target_os 或者 target_family 之类宽泛的平台名称，与我们所需的三元组不直接相关。
        //
        // [`DepKindInfo`]: https://docs.rs/cargo_metadata/0.18.1/cargo_metadata/struct.DepKindInfo.html#structfield.target
        let metadata = MetadataCommand::new()
            .manifest_path(cargo_toml)
            .no_deps()
            .exec()
            .map_err(|err| eyre!("无法读取 cargo metadata 的结果：{err}"))?;
        let root = &metadata.workspace_root;
        // 每个 member package 解析的 workspace_root 和 members 是一样的
        if !map.contains_key(root) {
            map.insert(root.clone(), metadata);
        }
    }
    Ok(map)
}

/// 去除与机器相关的根目录；为了简洁和方便在不同机器上测试，将规范路径缩短
fn strip_base_path(target: &Utf8Path, base: &Utf8Path) -> Option<Utf8PathBuf> {
    target
        .strip_prefix(base)
        .map(|p| Utf8PathBuf::from(".").join(p))
        .ok()
}

pub struct Layout {
    /// 仓库根目录的完整路径，可用于去除 Metadata 中的路径前缀，让路径看起来更清爽
    root_path: Utf8PathBuf,
    /// 所有 Cargo.toml 的路径
    ///
    /// NOTE: Cargo.toml 并不意味着对应于一个 package —— virtual workspace 布局无需定义
    ///       `[package]`，因此要获取所有 packages 的信息，应使用 [`Layout::packages`]
    cargo_tomls: Vec<Utf8PathBuf>,
    /// 一个仓库可能有一个 Workspace，但也可能有多个，比如单独一些 Packages，那么它们是各自的 Workspace
    /// NOTE: workspaces 的键指向 workspace_root dir，而不是 workspace_root 的 Cargo.toml
    workspaces: Workspaces,
    /// The order is by pkg name and dir path.
    packages_info: Box<[PackageInfo]>,
}

impl fmt::Debug for Layout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct WorkspacesDebug<'a>(&'a Workspaces, &'a Utf8PathBuf);
        impl fmt::Debug for WorkspacesDebug<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut s = f.debug_struct("Workspaces");
                for (idx, (root, meta)) in self.0.iter().enumerate() {
                    let pkg_root = strip_base_path(root, self.1);
                    let mut members: Vec<_> = meta
                        .workspace_packages()
                        .iter()
                        .map(|p| p.name.as_str())
                        .collect();
                    members.sort_unstable();
                    s.field(&format!("[{idx}] root"), pkg_root.as_ref().unwrap_or(root))
                        .field(&format!("[{idx}] root.members"), &members);
                }
                s.finish()
            }
        }

        let root = &self.root_path;
        let canonicalize_root = root.canonicalize_utf8();
        let root_full = canonicalize_root.as_ref().unwrap_or(root);
        f.debug_struct("Layout")
            .field("repo_root", root)
            .field("cargo_tomls", &self.cargo_tomls)
            .field("workspaces", &WorkspacesDebug(&self.workspaces, root_full))
            .field("packages_info", &self.packages_info)
            .finish()
    }
}

impl Layout {
    pub fn parse(repo_root: &str, dirs_excluded: &[&str]) -> Result<Layout> {
        let root_path = Utf8PathBuf::from(repo_root);

        let cargo_tomls = find_all_cargo_toml_paths(repo_root, dirs_excluded);
        ensure!(
            !cargo_tomls.is_empty(),
            "repo_root `{repo_root}` (规范路径为 `{}`) 不是 Rust 项目，因为不包含任何 Cargo.toml",
            root_path.canonicalize_utf8()?
        );
        debug!(?cargo_tomls);

        let workspaces = parse(&cargo_tomls)?;

        let repo_targets = detect_targets::scripts_and_github_dir_in_repo(&root_path)?;
        debug!(?repo_targets);

        let cargo_tomls_len = cargo_tomls.len();
        let mut pkg_info = Vec::with_capacity(cargo_tomls_len);
        for ws in workspaces.values() {
            let ws_targets = detect_targets::WorkspaceTargetTriples::new(&root_path, ws);
            for pkg in ws_targets.packages {
                pkg_info.push(PackageInfo::new(pkg, &repo_targets)?);
            }
        }
        debug!(cargo_tomls_len, pkg_len = pkg_info.len());
        // sort by pkg_name and pkg_dir
        pkg_info.sort_unstable_by(|a, b| (&a.pkg_name, &a.pkg_dir).cmp(&(&b.pkg_name, &b.pkg_dir)));

        let layout = Layout {
            workspaces,
            cargo_tomls,
            root_path,
            packages_info: pkg_info.into_boxed_slice(),
        };
        debug!("layout={layout:#?}");
        Ok(layout)
    }

    pub fn packages(&self) -> Result<Packages> {
        // FIXME: 这里开始假设一个仓库不存在同名 package；这其实不正确：
        // 如果具有多个 workspaces，那么可能存在同名 package。
        // 但如果要支持同名 package，还需要修改 RepoConfig。
        // 目前没有计划支持这么做，因为出现同名 package 的情况并不常见。
        // 从根本上解决这个问题，必须不允许同名 package，比如统一成
        // 路径，或者对同名 package 进行检查，必须包含额外的路径。
        // 无论如何，这都带来复杂性，目前来看并不值得。
        let map: IndexMap<_, _> = self
            .packages_info
            .iter()
            .map(|info| {
                (
                    info.pkg_name.clone(),
                    PackageInfoShared {
                        pkg_dir: info.pkg_dir.clone(),
                        targets: info.targets.keys().cloned().collect(),
                    },
                )
            })
            .collect();
        if map.len() != self.packages_info.len() {
            let mut count = IndexMap::with_capacity(map.len());
            for name in self.packages_info.iter().map(|info| &*info.pkg_name) {
                count.entry(name).and_modify(|c| *c += 1).or_insert(1);
            }
            let duplicates: Vec<_> = count.iter().filter(|(_, c)| **c != 1).collect();
            bail!("暂不支持一个代码仓库中出现同名 packages：{duplicates:?}");
        }
        Ok(Packages { map })
    }

    pub fn norun(&self, norun: &mut Norun) {
        for info in &self.packages_info {
            for target in info.targets.keys() {
                norun.update_target(target);
            }
        }
    }
}

#[derive(Debug)]
pub struct Packages {
    /// The order is by pkg_name and pkd_dir.
    map: IndexMap<XString, PackageInfoShared>,
}

impl Packages {
    pub fn package_set(&self) -> IndexSet<&str> {
        self.map.keys().map(|name| name.as_str()).collect()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn single_vec_of_pkg(&self, name: &str) -> Vec<Pkg> {
        let Some((_, k, v)) = self.map.get_full(name) else {
            return vec![];
        };
        v.targets
            .iter()
            .map(move |target| Pkg {
                name: k,
                dir: &v.pkg_dir,
                target,
            })
            .collect()
    }

    pub fn all_vec_of_pkg(&self) -> Vec<Pkg> {
        self.map
            .iter()
            .flat_map(|(name, info)| {
                info.targets.iter().map(move |target| Pkg {
                    name,
                    dir: &info.pkg_dir,
                    target,
                })
            })
            .collect()
    }

    #[cfg(test)]
    pub fn test_new(pkgs: &[&str]) -> Self {
        let host = crate::output::host_target_triple().to_owned();
        Packages {
            map: pkgs
                .iter()
                .map(|name| {
                    (
                        XString::from(*name),
                        PackageInfoShared {
                            pkg_dir: Utf8PathBuf::new(),
                            targets: vec![host.clone()],
                        },
                    )
                })
                .collect(),
        }
    }
}

#[derive(Debug)]
struct PackageInfoShared {
    /// manifest_dir, i.e. manifest_path without Cargo.toml
    pkg_dir: Utf8PathBuf,
    targets: Vec<String>,
}

pub struct Pkg<'a> {
    pub name: &'a str,
    pub dir: &'a Utf8Path,
    pub target: &'a str,
}
