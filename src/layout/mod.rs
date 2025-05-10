//! 启发式了解项目的 Rust packages 组织结构。

use crate::{
    cli::no_layout_error,
    config::{Features, Resolve, TargetEnv, TargetsSpecifed},
    db::out::{CacheLayout, CachePackageInfo, CacheResolve, CargoMetaData},
    output::{get_channel, install_toolchain_idx, remove_targets, uninstall_toolchains},
    run_checker::DbRepo,
    utils::{empty, walk_dir, Exclude},
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
fn find_all_cargo_toml_paths<E: Exclude>(
    repo_root: &str,
    dirs_excluded: E,
    only_dirs: &[glob::Pattern],
) -> Vec<Utf8PathBuf> {
    let mut cargo_tomls = walk_dir(repo_root, 10, dirs_excluded, only_dirs, |file_path| {
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
        let metadata = match MetadataCommand::new().manifest_path(cargo_toml).exec() {
            Ok(metadata) => metadata,
            Err(err) => {
                if no_layout_error() {
                    error!("无法从 {cargo_toml} 中读取 cargo metadata 的结果：\n{err}");
                    continue;
                } else {
                    bail!("无法从 {cargo_toml} 中读取 cargo metadata 的结果：\n{err}");
                }
            }
        };
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
    pub fn parse<E: Exclude>(
        repo_root: &str,
        dirs_excluded: E,
        only_dirs: &[glob::Pattern],
    ) -> Result<Layout> {
        let root_path = Utf8PathBuf::from(repo_root).canonicalize_utf8()?;

        let cargo_tomls = find_all_cargo_toml_paths(repo_root, dirs_excluded, only_dirs);
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
        let cargo_tomls = find_all_cargo_toml_paths(repo_root, empty(), &[]);
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
        //
        // NOTE: 这里对 package 进行了筛选，因为 cargo_tomls 结合了配置文件
        // 中的筛选方式，而不再指向全部 package 的 Cargo.toml.

        let libs = lib_pkgs(&self.workspaces, &self.cargo_tomls);
        let audit = {
            // query cargo audit with less pkgs
            let pkg_dirs: Vec<_> = if self.workspaces.len() > self.cargo_tomls.len() {
                self.cargo_tomls
                    .iter()
                    .map(|path| {
                        let mut path = path.to_owned();
                        // remove trailling Cargo.toml
                        assert_eq!(
                            path.file_name(),
                            Some("Cargo.toml"),
                            "{path} must be a Cargo.toml path"
                        );
                        path.pop();
                        path
                    })
                    .collect()
            } else {
                self.workspaces.keys().cloned().collect()
            };
            let res = CargoAudit::new_for_pkgs(pkg_dirs);
            match res {
                Ok(audit) => audit,
                Err(err) => {
                    if no_layout_error() {
                        error!(?err, "skip the error by setting empty audit result");
                        Default::default()
                    } else {
                        return Err(err);
                    }
                }
            }
        };

        let map: IndexMap<_, _> = self
            .packages_info
            .iter()
            .filter_map(|info| {
                let features_islib = libs.get(&info.pkg_name)?;
                Some((
                    info.pkg_name.clone(),
                    PackageInfoShared {
                        pkg_dir: info.pkg_dir.clone(),
                        targets: info.targets.keys().cloned().collect(),
                        features: features_islib.features.clone(),
                        toolchain: info.toolchain,
                        audit: audit.get(&info.pkg_name).cloned(),
                        is_lib: features_islib.is_lib,
                    },
                ))
            })
            .collect();

        let repo_root = self.repo_root().to_owned();
        Ok(Packages { repo_root, map })
    }

    pub fn set_installation_targets(&mut self, targets: TargetsSpecifed) {
        // 如果配置文件设置了 targets，则追加到安装列表
        for info in &mut self.packages_info {
            let old = self
                .installation
                .get_mut(&info.toolchain.unwrap_or(0))
                .unwrap();
            if let Some(pkg_targets) = targets.pkgs.get(&*info.pkg_name) {
                // append package targets
                old.extend_from_slice(pkg_targets);
                info.add_specified_targets(pkg_targets);
            }
            // append repo targets
            old.extend_from_slice(targets.repo);
            info.add_specified_targets(targets.repo);
            // remove repeated targets
            old.sort_unstable();
            old.dedup();
        }

        // remove no_install_targets from global toolchain
        for idx in self.installation.keys().copied() {
            remove_targets(idx, targets.no_install);
        }
        // remove no_install_targets from local repos
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
                    features_args: r.features_args.clone(),
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
                            features: vec![],
                            toolchain: Some(0),
                            audit: None,
                            is_lib: true,
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
    features: Vec<String>,
    toolchain: Option<usize>,
    audit: Audit,
    /// cargo-semver-checks only works for lib crate
    is_lib: bool,
}

impl PackageInfoShared {
    /// Generate a list of targets and features for the same package.
    pub fn pkgs<'a>(
        &'a self,
        name: &'a str,
        targets: Option<&'a [String]>,
        features: &[Features],
        env: Option<&'a IndexMap<String, String>>,
        target_env: Option<&TargetEnv>,
    ) -> Result<Vec<Pkg<'a>>> {
        let merged_targets = targets.unwrap_or(&self.targets);

        for feat in features {
            feat.validate(&self.features, merged_targets, name)?;
        }

        let feats_len = if features.is_empty() {
            1
        } else {
            features.len()
        };
        let cap = merged_targets.len() * feats_len;
        let mut pkgs = Vec::<Pkg>::with_capacity(cap);

        for target in merged_targets {
            let env = match (env, target_env) {
                (None, None) => IndexMap::default(),
                (None, Some(t)) => t.merge(target, &IndexMap::default()),
                (Some(g), None) => g.clone(),
                (Some(g), Some(t)) => t.merge(target, g),
            };

            let v_features_args = if features.is_empty() {
                vec![vec![]]
            } else {
                features
                    .iter()
                    .map(|feat| feat.to_argument(target))
                    .collect()
            };

            for features_args in v_features_args {
                pkgs.push(Pkg {
                    name,
                    dir: &self.pkg_dir,
                    target,
                    features_args,
                    toolchain: self.toolchain,
                    env: env.clone(),
                    audit: self.audit.as_ref(),
                    is_lib: self.is_lib,
                });
            }
        }

        Ok(pkgs)
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
    pub features_args: Vec<String>,
    pub toolchain: Option<usize>,
    pub env: IndexMap<String, String>,
    pub audit: Option<&'a Rc<CargoAudit>>,
    pub is_lib: bool,
}

#[derive(Debug)]
struct PkgFeaturesLib {
    features: Vec<String>,
    is_lib: bool,
}

/// Only extract pkgs from the given cargo_tomls.
/// By default, cargo_tomls points to the same pkg in Workspaces.
/// But if filtering, like only_pkg_dir_globs or skip_pkg_dir_globs, is set,
/// we only want the specifed ones.
fn lib_pkgs(
    workspaces: &Workspaces,
    cargo_tomls: &[Utf8PathBuf],
) -> IndexMap<XString, PkgFeaturesLib> {
    let mut map = IndexMap::new();
    for ws in workspaces.values() {
        'p: for p in ws.workspace_packages() {
            if !cargo_tomls.contains(&p.manifest_path) {
                // skip if the package Cargo.toml is not specifed
                continue;
            }
            let features: Vec<String> = p.features.keys().cloned().collect();
            let old = map.insert(
                XString::new(&*p.name),
                PkgFeaturesLib {
                    features,
                    is_lib: false,
                },
            );
            if no_layout_error() && old.is_some() {
                // solana-foundation/anchor: Package `crank` already exists.
                error!(
                    "Package `{}` already exists.\nOld={:?}",
                    p.name,
                    old.unwrap()
                );
            } else {
                assert!(
                    old.is_none(),
                    "Package `{}` already exists.\nOld={old:?}",
                    p.name
                );
            }
            for target in &p.targets {
                for kind in &target.kind {
                    if kind == "lib" {
                        // The package is inserted above just now.
                        map.get_mut(&*p.name).unwrap().is_lib = true;
                        continue 'p;
                    }
                }
            }
        }
    }
    map
}
