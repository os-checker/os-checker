//! 启发式了解项目的 Rust packages 组织结构。

use crate::{utils::walk_dir, Result};
use cargo_metadata::{
    camino::{Utf8Path, Utf8PathBuf},
    Metadata, MetadataCommand,
};
use std::{collections::BTreeMap, fmt};

#[cfg(test)]
mod tests;

/// Target triple list and cargo check diagnostics.
mod cargo_check_verbose;
use cargo_check_verbose::PackageInfo;

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

pub struct LayoutOwner {
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
    packages_info: Box<[PackageInfo]>,
}

impl fmt::Debug for LayoutOwner {
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

impl LayoutOwner {
    fn new(repo_root: &str, dirs_excluded: &[&str]) -> Result<LayoutOwner> {
        let root_path = Utf8PathBuf::from(repo_root);

        let cargo_tomls = find_all_cargo_toml_paths(repo_root, dirs_excluded);
        ensure!(
            !cargo_tomls.is_empty(),
            "repo_root `{repo_root}` (规范路径为 `{}`) 不是 Rust 项目，因为不包含任何 Cargo.toml",
            root_path.canonicalize_utf8()?
        );
        debug!(?cargo_tomls);

        let workspaces = parse(&cargo_tomls)?;

        let cargo_tomls_len = cargo_tomls.len();
        let mut pkg_info = Vec::with_capacity(cargo_tomls_len);
        for ws in workspaces.values() {
            for member in ws.workspace_packages() {
                let pkg_dir = member.manifest_path.parent().unwrap();
                pkg_info.push(PackageInfo::new(pkg_dir, &member.name)?);
            }
        }
        debug!(cargo_tomls_len, pkg_len = pkg_info.len());
        pkg_info.sort_unstable_by(|a, b| (&a.pkg_name, &a.pkg_dir).cmp(&(&b.pkg_name, &b.pkg_dir)));

        let layout = LayoutOwner {
            workspaces,
            cargo_tomls,
            root_path,
            packages_info: pkg_info.into_boxed_slice(),
        };
        debug!("layout={layout:#?}");
        Ok(layout)
    }

    // FIXME: remove Packages
    fn packages(&self) -> Packages {
        let cargo_tomls_len = self.cargo_tomls.len();
        let mut v = Vec::with_capacity(cargo_tomls_len);
        for (cargo_toml, ws) in &self.workspaces {
            for member in ws.workspace_packages() {
                v.push(Package {
                    name: &member.name,
                    cargo_toml: &member.manifest_path,
                    workspace_root: cargo_toml,
                });
            }
        }
        debug!(cargo_tomls_len, pkg_len = v.len());
        v.sort_unstable_by_key(|pkg| (pkg.name, pkg.cargo_toml));
        v.into_boxed_slice()
    }

    fn into_self_cell(self) -> Layout {
        Layout::new(self, Self::packages)
    }
}

/// package infomation
#[derive(Clone, Copy)]
pub struct Package<'a> {
    /// package name written in its Cargo.toml
    pub name: &'a str,
    /// i.e. manifest_path
    pub cargo_toml: &'a Utf8Path,
    /// workspace root path without manifest_path
    workspace_root: &'a Utf8Path,
}

impl fmt::Debug for Package<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let toml = self.cargo_toml;
        let root = self.workspace_root;
        let toml_stripped = strip_base_path(toml, root);
        f.debug_struct("Package")
            .field("name", &self.name)
            .field("cargo_toml", &toml_stripped.as_deref().unwrap_or(toml))
            .field(
                "workspace_root (file name)",
                &root.file_name().unwrap_or("unknown???"),
            )
            .finish()
    }
}

impl Package<'_> {
    #[cfg(test)]
    pub fn test_new<const N: usize>(names: [&'static str; N]) -> [Package<'static>; N] {
        use std::sync::LazyLock;
        static PATH: LazyLock<[Utf8PathBuf; 2]> =
            LazyLock::new(|| [Utf8PathBuf::from("./Cargo.toml"), Utf8PathBuf::from(".")]);

        let cargo_toml = &PATH[0];
        let workspace_root = &PATH[1];
        names.map(|name| Package {
            name,
            cargo_toml,
            workspace_root,
        })
    }
}

type Packages<'a> = Box<[Package<'a>]>;

self_cell::self_cell!(
    pub struct Layout {
        owner: LayoutOwner,
        #[covariant]
        dependent: Packages,
    }
    impl {Debug}
);

impl Layout {
    pub fn parse(repo_root: &str, dirs_excluded: &[&str]) -> Result<Layout> {
        LayoutOwner::new(repo_root, dirs_excluded).map(LayoutOwner::into_self_cell)
    }

    // pub fn layout(&self) -> &LayoutOwner {
    //     self.borrow_owner()
    // }

    pub fn packages(&self) -> &[Package] {
        self.borrow_dependent()
    }

    // pub fn root_path(&self) -> &Utf8Path {
    //     &self.layout().root_path
    // }
}
