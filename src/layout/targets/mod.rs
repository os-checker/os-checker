use super::detect_targets::PackageTargets;
use crate::{utils::PECULIAR_TARGETS, Result, XString};
use cargo_metadata::camino::{Utf8Path, Utf8PathBuf};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

mod type_conversion;

/// Refer to https://github.com/os-checker/os-checker/issues/26 for more info.
// FIXME: 把 tag 和 path 分开
// TODO: 在明确指定 targets 的情况下，还需要脚本指定的 targets 吗？(关于安装和 resolve)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetSource {
    RustToolchainToml(Utf8PathBuf),
    CargoConfigToml(Utf8PathBuf),
    CargoTomlDocsrsInPkgDefault(Utf8PathBuf),
    CargoTomlDocsrsInWorkspaceDefault(Utf8PathBuf),
    CargoTomlDocsrsInPkg(Utf8PathBuf),
    CargoTomlDocsrsInWorkspace(Utf8PathBuf),
    DetectedByPkgScripts(Utf8PathBuf),
    DetectedByRepoGithub(Utf8PathBuf),
    DetectedByRepoScripts(Utf8PathBuf),
    // 被配置文件指定
    SpecifiedInOsCheckerConfig,
    /// 非上面的方式指定，那么默认会增加一个 host target
    UnspecifiedDefaultToHostTarget,
}

/// A list of target triples obtained from multiple sources.
/// The orders in key and value demonstrates how they shape.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Targets {
    map: IndexMap<String, Vec<TargetSource>>,
}

impl std::ops::Deref for Targets {
    type Target = IndexMap<String, Vec<TargetSource>>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}
impl std::ops::DerefMut for Targets {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

impl Targets {
    pub fn new() -> Targets {
        Targets {
            map: IndexMap::with_capacity(4),
        }
    }

    pub fn merge(&mut self, other: &Self) {
        for (target, sources) in &other.map {
            if let Some(v) = self.get_mut(target) {
                v.extend(sources.iter().cloned());
            } else {
                self.insert(target.to_owned(), sources.clone());
            }
        }
    }

    fn unspecified_default(&mut self) {
        let target = crate::output::host_target_triple();
        if let Some(source) = self.get_mut(target) {
            source.push(TargetSource::UnspecifiedDefaultToHostTarget);
            warn!(
                ?source,
                "host target has already been added from a specific source."
            );
        } else {
            self.insert(
                target.to_owned(),
                vec![TargetSource::UnspecifiedDefaultToHostTarget],
            );
        }
    }

    /// This will only be called before pushing the first specified target.
    fn remove_unspecified_default(&mut self) {
        let target = crate::output::host_target_triple();
        self.shift_remove_entry(target);
    }

    fn specified(&mut self, target: &str) {
        if let Some(source) = self.get_mut(target) {
            source.push(TargetSource::SpecifiedInOsCheckerConfig);
        } else {
            self.insert(
                target.to_owned(),
                vec![TargetSource::SpecifiedInOsCheckerConfig],
            );
        }
    }

    pub fn push(
        &mut self,
        target: impl AsRef<str> + Into<String>,
        path: impl Into<Utf8PathBuf>,
        f: impl FnOnce(Utf8PathBuf) -> TargetSource,
    ) {
        let path = path.into();
        let target_ref = target.as_ref();
        match self.get_mut(target_ref) {
            Some(v) => {
                let src = f(path);
                if !v.contains(&src) {
                    // 如果一个文件内出现多次同样的 target，只需要记录一次
                    v.push(src);
                }
            }
            None => _ = self.insert(target.into(), vec![f(path)]),
        }
    }

    pub fn detected_by_repo_github(&mut self, target: &str, path: Utf8PathBuf) {
        self.push(target, path, TargetSource::DetectedByRepoGithub);
    }

    pub fn detected_by_repo_scripts(&mut self, target: &str, path: Utf8PathBuf) {
        self.push(target, path, TargetSource::DetectedByRepoScripts);
    }

    pub fn detected_by_pkg_scripts(&mut self, target: &str, path: Utf8PathBuf) {
        self.push(target, path, TargetSource::DetectedByPkgScripts);
    }

    pub fn cargo_config_toml(&mut self, target: String, path: Utf8PathBuf) {
        self.push(target, path, TargetSource::CargoConfigToml);
    }

    pub fn cargo_toml_docsrs_in_pkg_default(&mut self, target: &str, path: &Utf8Path) {
        self.push(target, path, TargetSource::CargoTomlDocsrsInPkgDefault);
    }

    pub fn cargo_toml_docsrs_in_pkg(&mut self, target: &str, path: &Utf8Path) {
        self.push(target, path, TargetSource::CargoTomlDocsrsInPkg);
    }

    pub fn cargo_toml_docsrs_in_workspace_default(&mut self, target: &str, path: &Utf8Path) {
        self.push(
            target,
            path,
            TargetSource::CargoTomlDocsrsInWorkspaceDefault,
        );
    }

    pub fn cargo_toml_docsrs_in_workspace(&mut self, target: &str, path: &Utf8Path) {
        self.push(target, path, TargetSource::CargoTomlDocsrsInWorkspace);
    }

    pub fn rust_toolchain_toml(&mut self, target: &str, path: &Utf8Path) {
        self.push(target, path, TargetSource::RustToolchainToml);
    }

    fn merge_more(&mut self, pkg_dir: &Utf8Path, repo: &Targets) -> Result<()> {
        super::detect_targets::scripts_in_pkg_dir(pkg_dir, self)?;
        self.remove_peculiar_targets(); // 搜索的 pkg_dir 也会引入 peculiar_targets
        self.merge(repo);
        if self.is_empty() {
            // 无指定的 targets
            self.unspecified_default();
        }
        Ok(())
    }

    pub fn remove_peculiar_targets(&mut self) {
        for &peculiar in PECULIAR_TARGETS {
            self.swap_remove(peculiar);
        }
    }
}

#[derive(Debug)]
pub struct PackageInfo {
    pub pkg_name: XString,
    /// i.e. manifest_dir
    pub pkg_dir: Utf8PathBuf,
    pub targets: Targets,
    pub toolchain: Option<usize>,
}

impl PackageInfo {
    pub fn new(pkg: PackageTargets, repo_targets: &Targets) -> Result<Self> {
        let PackageTargets {
            pkg_name,
            pkg_dir,
            mut targets,
            toolchain,
        } = pkg;
        targets.merge_more(&pkg_dir, repo_targets)?;

        debug!(?targets);
        Ok(PackageInfo {
            pkg_name,
            pkg_dir,
            targets,
            // 仓库指定的工具链
            toolchain: toolchain.map(|val| val.store()),
        })
    }
}
