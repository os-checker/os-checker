use crate::{Result, XString};
use cargo_metadata::{
    camino::{Utf8Path, Utf8PathBuf},
    CompilerMessage, Message,
};
use indexmap::IndexMap;

use super::detect_targets::PackageTargets;

/// Refer to https://github.com/os-checker/os-checker/issues/26 for more info.
// FIXME: 把 tag 和 path 分开
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetSource {
    RustToolchainToml(Utf8PathBuf),
    CargoConfigToml(Utf8PathBuf),
    CargoTomlDocsrsInPkgDefault(Utf8PathBuf),
    CargoTomlDocsrsInWorkspaceDefault(Utf8PathBuf),
    CargoTomlDocsrsInPkg(Utf8PathBuf),
    CargoTomlDocsrsInWorkspace(Utf8PathBuf),
    /// 非上面的方式指定，那么默认会增加一个 host target
    UnspecifiedDefaultToHostTarget,
    DetectedByPkgScripts(Utf8PathBuf),
    DetectedByRepoGithub(Utf8PathBuf),
    DetectedByRepoScripts(Utf8PathBuf),
    OverriddenInOsCheckerYaml,
}

/// A list of target triples obtained from multiple sources.
/// The orders in key and value demonstrates how they shape.
#[derive(Debug, Default)]
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
        } else {
            self.insert(
                target.to_owned(),
                vec![TargetSource::UnspecifiedDefaultToHostTarget],
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
        if self.is_empty() {
            // 无指定的 targets
            self.unspecified_default();
        }
        super::detect_targets::scripts_in_pkg_dir(pkg_dir, self)?;
        self.merge(repo);
        Ok(())
    }
}

pub struct CargoCheckDiagnostics {
    pub target_triple: String,
    pub compiler_messages: Box<[CompilerMessage]>,
    pub duration_ms: u64,
}

impl CargoCheckDiagnostics {
    pub fn new(pkg_dir: &Utf8Path, pkg_name: &str, target_triple: &str) -> Result<Self> {
        let (duration_ms, out) = crate::utils::execution_time_ms(|| {
            duct::cmd!(
                "cargo",
                "check",
                "--message-format=json",
                "--target",
                target_triple
            )
            .dir(pkg_dir)
            .stdout_capture()
            .unchecked()
            .run()
        });

        Ok(CargoCheckDiagnostics {
            target_triple: target_triple.to_owned(),
            compiler_messages: Message::parse_stream(out?.stdout.as_slice())
                .filter_map(|mes| match mes.ok()? {
                    Message::CompilerMessage(mes) if mes.target.name == pkg_name => Some(mes),
                    _ => None,
                })
                .collect(),
            duration_ms,
        })
    }
}

impl std::fmt::Debug for CargoCheckDiagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(test)]
        {
            f.debug_struct("CargoCheckDiagnostics")
                .field("target_triple", &self.target_triple)
                .field(
                    "compiler_messages",
                    &self
                        .compiler_messages
                        .iter()
                        .map(|d| d.message.to_string())
                        .collect::<Vec<_>>(),
                )
                .finish()
        }
        #[cfg(not(test))]
        f.debug_struct("CargoCheckDiagnostics")
            .field("target_triple", &self.target_triple)
            .field("duration_ms", &self.duration_ms)
            .field("compiler_messages.len", &self.compiler_messages.len())
            .finish()
    }
}

#[derive(Debug)]
pub struct PackageInfo {
    pub pkg_name: XString,
    /// i.e. manifest_dir
    pub pkg_dir: Utf8PathBuf,
    pub targets: Targets,
    pub toolchain: Option<usize>,
    pub cargo_check_diagnostics: Box<[CargoCheckDiagnostics]>,
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
        let cargo_check_diagnostics = targets
            .keys()
            .map(|target| CargoCheckDiagnostics::new(&pkg_dir, &pkg_name, target))
            .collect::<Result<_>>()?;
        Ok(PackageInfo {
            pkg_name,
            pkg_dir,
            targets,
            toolchain: toolchain.map(|val| val.store()),
            cargo_check_diagnostics,
        })
    }
}
