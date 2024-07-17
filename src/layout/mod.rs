//! 启发式了解项目的 Rust packages 组织结构。

use crate::Result;
use cargo_metadata::{camino::Utf8PathBuf, Metadata, MetadataCommand};
use std::{collections::BTreeMap, fmt};

#[cfg(test)]
mod tests;

/// 寻找仓库内所有 Cargo.toml 所在的路径；假设每个 Cargo.toml 位于其 package 的根目录
fn find_all_cargo_toml_paths(repo_root: &str, dirs_excluded: &[&str]) -> Vec<Utf8PathBuf> {
    let mut cargo_tomls: Vec<Utf8PathBuf> = walkdir::WalkDir::new(repo_root)
        // .min_depth(1) // 别把 ./ 纳入进来
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
            // println!("{}", entry.path().display());
            if filename == "Cargo.toml" {
                entry.into_path().try_into().ok()
            } else {
                None
            }
        })
        .collect();

    cargo_tomls.sort();
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

pub struct Layout {
    /// 仓库根目录的完整路径，可用于去除 Metadata 中的路径前缀，让路径看起来更清爽
    root_path: Utf8PathBuf,
    /// 所有 Packages 的 Cargo.toml 路径
    pkgs: Vec<Utf8PathBuf>,
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
                    s.field(
                        &format!("[{idx}] root"),
                        &Utf8PathBuf::from(".").join(root.strip_prefix(self.1).unwrap_or(self.1)),
                    )
                    .field(
                        &format!("[{idx}] root.members"),
                        &meta
                            .workspace_packages()
                            .iter()
                            .map(|p| p.name.as_str())
                            .collect::<Vec<_>>(),
                    );
                }
                s.finish()
            }
        }

        f.debug_struct("Layout")
            .field("pkgs", &self.pkgs)
            .field(
                "workspaces",
                &WorkspacesDebug(&self.workspaces, &self.root_path),
            )
            .finish()
    }
}

impl Layout {
    pub fn new(repo_root: &str, dirs_excluded: &[&str]) -> Result<Layout> {
        let root_path = Utf8PathBuf::from(repo_root).canonicalize_utf8()?;

        let cargo_tomls = find_all_cargo_toml_paths(repo_root, dirs_excluded);
        ensure!(
            !cargo_tomls.is_empty(),
            "repo_root `{repo_root}` (路径 `{root_path}`) 不是 Rust 项目，因为不包含任何 Cargo.toml"
        );

        let layout = Layout {
            workspaces: parse(&cargo_tomls)?,
            pkgs: cargo_tomls,
            root_path,
        };
        debug!("layout={layout:#?}");
        Ok(layout)
    }
}

/// 由 Layout.workspaces 计算
struct Package {
    name: String,
    /// i.e. manifest_path
    cargo_toml: Utf8PathBuf,
}