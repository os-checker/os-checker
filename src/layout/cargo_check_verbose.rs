use crate::{Result, XString};
use cargo_metadata::{
    camino::{Utf8Path, Utf8PathBuf},
    CompilerMessage, Message,
};
use indexmap::IndexMap;

/// Refer to https://github.com/os-checker/os-checker/issues/26 for more info.
#[derive(Debug, Clone)]
pub enum TargetSource {
    SpecifiedDefault,
    UnspecifiedDefault,
    DetectedByPkgScripts(Utf8PathBuf),
    DetectedByRepoGithub(Utf8PathBuf),
    DetectedByRepoScripts(Utf8PathBuf),
    OverriddenInYaml,
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

    fn merge(&mut self, other: &Self) {
        for (target, sources) in &other.map {
            if let Some(v) = self.get_mut(target) {
                v.extend(sources.iter().cloned());
            } else {
                self.insert(target.to_owned(), sources.clone());
            }
        }
    }

    fn specified_default(&mut self, target: String) {
        if let Some(source) = self.get_mut(&target) {
            source.push(TargetSource::SpecifiedDefault);
        } else {
            self.insert(target, vec![TargetSource::SpecifiedDefault]);
        }
    }

    fn unspecified_default(&mut self) {
        let target = crate::utils::host_target_triple();
        if let Some(source) = self.get_mut(target) {
            source.push(TargetSource::UnspecifiedDefault);
        } else {
            self.insert(target.to_owned(), vec![TargetSource::UnspecifiedDefault]);
        }
    }

    pub fn detected_by_repo_github(&mut self, target: &str, path: Utf8PathBuf) {
        match self.get_mut(target) {
            Some(v) => v.push(TargetSource::DetectedByRepoGithub(path)),
            None => {
                _ = self.insert(
                    target.to_owned(),
                    vec![TargetSource::DetectedByRepoGithub(path)],
                )
            }
        }
    }

    pub fn detected_by_repo_scripts(&mut self, target: &str, path: Utf8PathBuf) {
        match self.get_mut(target) {
            Some(v) => v.push(TargetSource::DetectedByRepoScripts(path)),
            None => {
                _ = self.insert(
                    target.to_owned(),
                    vec![TargetSource::DetectedByRepoScripts(path)],
                )
            }
        }
    }

    pub fn detected_by_pkg_scripts(&mut self, target: &str, path: Utf8PathBuf) {
        match self.get_mut(target) {
            Some(v) => v.push(TargetSource::DetectedByPkgScripts(path)),
            None => {
                _ = self.insert(
                    target.to_owned(),
                    vec![TargetSource::DetectedByPkgScripts(path)],
                )
            }
        }
    }

    fn from_repo_and_workspace(
        ws: Vec<String>,
        pkg_dir: &Utf8Path,
        repo: &Targets,
    ) -> Result<Self> {
        let mut targets = Targets::new();
        if ws.is_empty() {
            // 无指定的 targets
            targets.unspecified_default();
        } else {
            for target in ws {
                targets.specified_default(target);
            }
        }
        super::detect_targets::in_pkg_dir(pkg_dir, &mut targets)?;
        targets.merge(repo);
        Ok(targets)
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
    pub cargo_check_diagnostics: Box<[CargoCheckDiagnostics]>,
}

impl PackageInfo {
    pub fn new(
        pkg_dir: Utf8PathBuf,
        pkg_name: XString,
        repo_targets: &Targets,
        ws_targets: Vec<String>,
    ) -> Result<Self> {
        let targets = Targets::from_repo_and_workspace(ws_targets, &pkg_dir, repo_targets)?;
        // super::detect_targets::in_pkg_dir(pkg_dir, &mut target_triples.targets)?;
        // target_triples.targets.merge(repo_targets);

        let cargo_check_diagnostics = targets
            .keys()
            .map(|target| CargoCheckDiagnostics::new(&pkg_dir, &pkg_name, target))
            .collect::<Result<_>>()?;
        Ok(PackageInfo {
            pkg_name,
            pkg_dir,
            targets,
            cargo_check_diagnostics,
        })
    }
}
