//! 启发式了解项目的 Rust packages 组织结构。

use crate::Result;
use cargo_metadata::{
    camino::{Utf8Path, Utf8PathBuf},
    Metadata, MetadataCommand,
};
use std::{collections::BTreeMap, fmt};

#[cfg(test)]
mod tests;

/// 寻找仓库内所有 Cargo.toml 所在的路径
fn find_all_cargo_toml_paths(repo_root: &str, dirs_excluded: &[&str]) -> Vec<Utf8PathBuf> {
    let mut cargo_tomls: Vec<Utf8PathBuf> = walkdir::WalkDir::new(repo_root)
        .max_depth(10) // 目录递归上限
        .into_iter()
        .filter_entry(|entry| {
            // 别进入这些文件夹（适用于子目录递归）
            const NO_JUMP_IN: &[&str] = &[".git", "target"];
            let filename = entry.file_name();
            let excluded = &mut NO_JUMP_IN.iter().chain(dirs_excluded);
            !excluded.any(|&dir| dir == filename)
        })
        .filter_map(|entry| {
            // 只搜索 Cargo.toml 文件
            let entry = entry.ok()?;
            if !entry.file_type().is_file() {
                return None;
            }
            let filename = entry.file_name().to_str()?;
            if filename == "Cargo.toml" {
                entry.into_path().try_into().ok()
            } else {
                None
            }
        })
        .collect();

    cargo_tomls.sort_unstable();
    cargo_tomls
}

type Workspaces = BTreeMap<Utf8PathBuf, Metadata>;

/// 解析所有 Cargo.toml 所在的 Package 的 metadata 来获取仓库所有的 Workspaces
fn parse(cargo_tomls: &[Utf8PathBuf]) -> Result<Workspaces> {
    let mut map = BTreeMap::new();
    for cargo_toml in cargo_tomls {
        // 暂时不解析依赖，原因：
        // * 不需要依赖信息
        // * 加快的解析速度
        // * 如何处理 features? features 会影响依赖吗？（待确认）
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
    workspaces: Workspaces,
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
            .finish()
    }
}

impl Layout {
    pub fn new(repo_root: &str, dirs_excluded: &[&str]) -> Result<Layout> {
        let root_path = Utf8PathBuf::from(repo_root);

        let cargo_tomls = find_all_cargo_toml_paths(repo_root, dirs_excluded);
        ensure!(
            !cargo_tomls.is_empty(),
            "repo_root `{repo_root}` (规范路径为 `{}`) 不是 Rust 项目，因为不包含任何 Cargo.toml",
            root_path.canonicalize_utf8()?
        );

        let layout = Layout {
            workspaces: parse(&cargo_tomls)?,
            cargo_tomls,
            root_path,
        };
        debug!("layout={layout:#?}");
        Ok(layout)
    }

    pub fn packages(&self) -> Vec<Package> {
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
        // debug!(cargo_tomls_len, pkg_len = v.len());
        v.sort_unstable_by_key(|pkg| (pkg.name, pkg.cargo_toml));
        v
    }
}

/// package infomation
pub struct Package<'a> {
    /// package name written in its Cargo.toml
    name: &'a str,
    /// i.e. manifest_path
    cargo_toml: &'a Utf8Path,
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
