//! 启发式了解项目的 Rust packages 组织结构。

use crate::{
    config::{Resolve, TargetsSpecifed},
    db::out::{CacheLayout, CachePackageInfo, CacheResolve, CargoMetaData},
    output::{get_channel, install_toolchain_idx, uninstall_toolchains},
    run_checker::DbRepo,
    utils::walk_dir,
    Result, XString,
};
use audit::CargoAudit;
use cargo_metadata::{
    camino::{Utf8Path, Utf8PathBuf},
    Metadata, MetadataCommand,
};
use indexmap::IndexMap;
use std::{fmt, rc::Rc};

#[cfg(test)]
mod tests;

/// Target triple list and cargo check diagnostics.
mod targets;
use targets::PackageInfo;

mod detect_targets;
pub use detect_targets::RustToolchain;

/// run cargo audit but share the result with related pkgs
mod audit;

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

pub type Workspaces = IndexMap<Utf8PathBuf, Metadata>;

/// 解析所有 Cargo.toml 所在的 Package 的 metadata 来获取仓库所有的 Workspaces
#[instrument(level = "trace")]
fn parse(cargo_tomls: &[Utf8PathBuf]) -> Result<Workspaces> {
    let mut map = IndexMap::new();
    for cargo_toml in cargo_tomls {
        // NOTE: 一旦支持 features，这里可能需要传递它们
        let metadata = MetadataCommand::new()
            .manifest_path(cargo_toml)
            .exec()
            .map_err(|err| eyre!("无法读取 cargo metadata 的结果：{err}"))?;
        let root = &metadata.workspace_root;
        // 每个 member package 解析的 workspace_root 和 members 是一样的
        if !map.contains_key(root) {
            map.insert(root.clone(), metadata);
        }
    }
    map.sort_unstable_keys();
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
    /// 当 parse 出现问题时的错误信息
    parse_error: Option<Box<str>>,
    /// toolchains and targets required
    installation: IndexMap<usize, Vec<String>>,
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
    #[instrument(level = "trace")]
    pub fn parse(repo_root: &str, dirs_excluded: &[&str]) -> Result<Layout> {
        let root_path = Utf8PathBuf::from(repo_root).canonicalize_utf8()?;

        let cargo_tomls = find_all_cargo_toml_paths(repo_root, dirs_excluded);
        ensure!(
            !cargo_tomls.is_empty(),
            "repo_root `{repo_root}` (规范路径为 `{root_path}`) 不是 Rust \
             项目，因为不包含任何 Cargo.toml",
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

        let installation = installation(&pkg_info);

        let layout = Layout {
            workspaces,
            cargo_tomls,
            root_path,
            packages_info: pkg_info.into_boxed_slice(),
            parse_error: None,
            installation,
        };
        debug!("layout={layout:#?}");
        Ok(layout)
    }

    /// NOTE: 此函数在 parse 失败时调用
    pub fn empty(repo_root: &str, err: eyre::Error) -> Self {
        // 回溯错误应使用 `{:?}`，并携带了 ansi 转义字符
        let err = format!("{err:?}");
        info!("{repo_root} 仓库在解析项目布局时遇到解析错误：\n{err}");
        let parse_error = strip_ansi_escapes::strip_str(err).into_boxed_str();

        let root_path = Utf8PathBuf::from(repo_root);
        let cargo_tomls = find_all_cargo_toml_paths(repo_root, &[]);
        let (workspaces, packages_info, installation) = Default::default();
        Layout {
            root_path,
            cargo_tomls,
            workspaces,
            packages_info,
            parse_error: Some(parse_error),
            installation,
        }
    }

    pub fn get_parse_error(&self) -> Option<&str> {
        self.parse_error.as_deref()
    }

    pub fn repo_root(&self) -> &Utf8Path {
        &self.root_path
    }

    pub fn packages(&self) -> Result<Packages> {
        // FIXME: 这里开始假设一个仓库不存在同名 package；这其实不正确：
        // 如果具有多个 workspaces，那么可能存在同名 package。
        // 但如果要支持同名 package，还需要修改 RepoConfig。
        // 目前没有计划支持这么做，因为出现同名 package 的情况并不常见。
        // 从根本上解决这个问题，必须不允许同名 package，比如统一成
        // 路径，或者对同名 package 进行检查，必须包含额外的路径。
        // 无论如何，这都带来复杂性，目前来看并不值得。

        let audit = CargoAudit::new_for_pkgs(self.workspaces.keys())?;

        let map: IndexMap<_, _> = self
            .packages_info
            .iter()
            .map(|info| {
                (
                    info.pkg_name.clone(),
                    PackageInfoShared {
                        pkg_dir: info.pkg_dir.clone(),
                        targets: info.targets.keys().cloned().collect(),
                        toolchain: info.toolchain,
                        audit: audit.get(&info.pkg_name).cloned(),
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
        let repo_root = self.repo_root().to_owned();
        Ok(Packages { repo_root, map })
    }

    // pub fn rust_toolchain_idxs(&self) -> Vec<usize> {
    //     let toolchains = self.packages_info.iter().filter_map(|p| p.toolchain);
    //     toolchains.sorted().dedup().collect()
    // }

    pub fn set_installation_targets(&mut self, targets: TargetsSpecifed) {
        // 如果配置文件设置了 targets，则直接覆盖
        let repo_overridden = !targets.repo.is_empty();
        for info in &self.packages_info {
            let old = self
                .installation
                .get_mut(&info.toolchain.unwrap_or(0))
                .unwrap();
            if let Some(pkg_targets) = targets.pkgs.get(&*info.pkg_name) {
                *old = pkg_targets.to_vec();
            } else if repo_overridden {
                *old = targets.repo.to_vec();
            }
        }
        dbg!(&targets.no_install, &self.installation);
        for no_install in targets.no_install {
            for v in self.installation.values_mut() {
                if let Some(pos) = v.iter().position(|t| t == no_install) {
                    v.remove(pos);
                }
            }
        }
    }

    /// 安装仓库工具链，并在主机和检查工具所在的工具链上安装 targets。
    pub fn install_toolchains(&self) -> Result<()> {
        for (&idx, targets) in &self.installation {
            install_toolchain_idx(idx, targets)?;
        }

        // 如何处理 targets？需要考虑配置文件所指定的 targets 吗？
        Ok(())
    }

    /// 删除仓库工具链，但不删除主机和检查工具所在的工具链上安装的 targets。
    pub fn uninstall_toolchains(&self) -> Result<()> {
        for &idx in self.installation.keys() {
            if idx != 0 {
                uninstall_toolchains(idx)?;
            }
        }

        Ok(())
    }

    /// Clone the data as a `CacheLayout`.
    pub fn set_layout_cache(&self, resolves: &[Resolve], db_repo: Option<DbRepo>) {
        let Some(db_repo) = db_repo else { return };

        let packages_info = self
            .packages_info
            .iter()
            .map(|info| CachePackageInfo {
                pkg_name: info.pkg_name.clone(),
                pkg_dir: info.pkg_dir.clone(),
                targets: info.targets.clone().into(),
                channel: get_channel(info.toolchain.unwrap_or(0)),
            })
            .collect();

        let layout = CacheLayout {
            root_path: self.root_path.clone(),
            cargo_tomls: self.cargo_tomls.clone().into_boxed_slice(),
            workspaces: self
                .workspaces
                .iter()
                .map(|(k, v)| (k.clone(), CargoMetaData::from_meta_data(v).unwrap()))
                .collect(),
            packages_info,
            resolves: resolves
                .iter()
                .map(|r| CacheResolve {
                    pkg_name: r.pkg_name.clone(),
                    target: r.target.clone(),
                    target_overridden: r.target_overridden,
                    channel: get_channel(r.toolchain.unwrap_or(0)),
                    checker: r.checker.into(),
                    cmd: r.cmd.clone(),
                })
                .collect(),
        };

        db_repo.set_layout_cache(layout);
    }

    /// All dir paths of workspace in the repo.
    pub fn workspace_dirs(&self) -> Vec<&Utf8Path> {
        self.workspaces.keys().map(|p| p.as_path()).collect()
    }
}

fn installation(info: &[PackageInfo]) -> IndexMap<usize, Vec<String>> {
    let mut map = IndexMap::<usize, Vec<String>>::with_capacity(info.len());

    // 对所有 pkgs 的工具链去重安装和检查工具
    for (toolchain, targets) in info.iter().map(|info| {
        (
            info.toolchain.unwrap_or(0),
            info.targets.keys().map(|s| s.to_owned()),
        )
    }) {
        match map.get_mut(&toolchain) {
            Some(v) => v.extend(targets),
            None => _ = map.insert(toolchain, targets.collect()),
        }
    }
    for v in map.values_mut() {
        v.sort_unstable();
        v.dedup();
    }
    map
}

#[derive(Debug)]
pub struct Packages {
    repo_root: Utf8PathBuf,
    /// The order is by pkg_name and pkd_dir.
    map: IndexMap<XString, PackageInfoShared>,
}

impl Packages {
    #[cfg(test)]
    pub fn test_new(pkgs: &[&str]) -> Self {
        let host = crate::output::host_target_triple().to_owned();
        Packages {
            repo_root: Utf8PathBuf::new(),
            map: pkgs
                .iter()
                .map(|name| {
                    (
                        XString::from(*name),
                        PackageInfoShared {
                            pkg_dir: Utf8PathBuf::new(),
                            targets: vec![host.clone()],
                            toolchain: Some(0),
                            audit: None,
                        },
                    )
                })
                .collect(),
        }
    }

    pub fn select<'a, I>(&self, globs: &[glob::Pattern], pkgs: I) -> Vec<(&str, &PackageInfoShared)>
    where
        I: Iterator<Item = &'a str>,
    {
        // default to all searched pkgs
        let mut map: IndexMap<&str, &PackageInfoShared> = self
            .iter()
            .map(|(name, info)| (name.as_str(), info))
            .collect();

        for (name, info) in &self.map {
            // once glob is matched, skip the pkg
            let pkg_dir = info.pkg_dir.strip_prefix(&self.repo_root).unwrap();
            for pat in globs {
                let matches = pat.matches(pkg_dir.as_str());
                if matches {
                    map.swap_remove(name.as_str());
                }
            }
        }

        // 已经校验过 pkg name 了；pkgs 来自 packages 字段，一定检查它们
        // 在已经 skip 过的 pkgs 上，可由 packages 指定回来
        map.extend(pkgs.map(|pkg| {
            let (_, name, info) = self.get_full(pkg).unwrap();
            (name.as_str(), info)
        }));

        map.sort_unstable_keys();
        map.into_iter().collect()
    }
}

impl std::ops::Deref for Packages {
    type Target = IndexMap<XString, PackageInfoShared>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

pub type Audit = Option<Rc<CargoAudit>>;

#[derive(Debug)]
pub struct PackageInfoShared {
    /// manifest_dir, i.e. manifest_path without Cargo.toml
    pkg_dir: Utf8PathBuf,
    targets: Vec<String>,
    toolchain: Option<usize>,
    audit: Audit,
}

impl PackageInfoShared {
    pub fn pkgs<'a>(&'a self, name: &'a str, targets: Option<&'a [String]>) -> Vec<Pkg<'a>> {
        targets
            .unwrap_or(&self.targets)
            .iter()
            .map(|target| Pkg {
                name,
                dir: &self.pkg_dir,
                target,
                toolchain: self.toolchain,
                audit: self.audit.as_ref(),
            })
            .collect()
    }

    pub fn targets(&self) -> Vec<String> {
        self.targets.clone()
    }
}

#[derive(Debug)]
pub struct Pkg<'a> {
    pub name: &'a str,
    pub dir: &'a Utf8Path,
    pub target: &'a str,
    pub toolchain: Option<usize>,
    pub audit: Option<&'a Rc<CargoAudit>>,
}
