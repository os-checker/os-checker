use super::{TargetSource, Targets};
use os_checker_types::db as out;

// ********** CLI => os_checker_types **********

impl From<TargetSource> for out::TargetSource {
    fn from(value: TargetSource) -> Self {
        match value {
            TargetSource::RustToolchainToml(utf8_path_buf) => {
                Self::RustToolchainToml(utf8_path_buf)
            }
            TargetSource::CargoConfigToml(utf8_path_buf) => Self::CargoConfigToml(utf8_path_buf),
            TargetSource::CargoTomlDocsrsInPkgDefault(utf8_path_buf) => {
                Self::CargoTomlDocsrsInPkgDefault(utf8_path_buf)
            }
            TargetSource::CargoTomlDocsrsInWorkspaceDefault(utf8_path_buf) => {
                Self::CargoTomlDocsrsInWorkspaceDefault(utf8_path_buf)
            }
            TargetSource::CargoTomlDocsrsInPkg(utf8_path_buf) => {
                Self::CargoTomlDocsrsInPkg(utf8_path_buf)
            }
            TargetSource::CargoTomlDocsrsInWorkspace(utf8_path_buf) => {
                Self::CargoTomlDocsrsInWorkspace(utf8_path_buf)
            }
            TargetSource::UnspecifiedDefaultToHostTarget => Self::UnspecifiedDefaultToHostTarget,
            TargetSource::DetectedByPkgScripts(utf8_path_buf) => {
                Self::DetectedByPkgScripts(utf8_path_buf)
            }
            TargetSource::DetectedByRepoGithub(utf8_path_buf) => {
                Self::DetectedByRepoGithub(utf8_path_buf)
            }
            TargetSource::DetectedByRepoScripts(utf8_path_buf) => {
                Self::DetectedByRepoScripts(utf8_path_buf)
            }
        }
    }
}

impl From<Targets> for out::Targets {
    fn from(value: Targets) -> Self {
        let Targets { map } = value;
        Self {
            map: map
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().map(|t| t.into()).collect()))
                .collect(),
        }
    }
}

// ********** os_checker_types => CLI **********

impl From<out::TargetSource> for TargetSource {
    fn from(value: TargetSource) -> Self {
        match value {
            out::TargetSource::RustToolchainToml(utf8_path_buf) => {
                Self::RustToolchainToml(utf8_path_buf)
            }
            out::TargetSource::CargoConfigToml(utf8_path_buf) => {
                Self::CargoConfigToml(utf8_path_buf)
            }
            out::TargetSource::CargoTomlDocsrsInPkgDefault(utf8_path_buf) => {
                Self::CargoTomlDocsrsInPkgDefault(utf8_path_buf)
            }
            out::TargetSource::CargoTomlDocsrsInWorkspaceDefault(utf8_path_buf) => {
                Self::CargoTomlDocsrsInWorkspaceDefault(utf8_path_buf)
            }
            out::TargetSource::CargoTomlDocsrsInPkg(utf8_path_buf) => {
                Self::CargoTomlDocsrsInPkg(utf8_path_buf)
            }
            out::TargetSource::CargoTomlDocsrsInWorkspace(utf8_path_buf) => {
                Self::CargoTomlDocsrsInWorkspace(utf8_path_buf)
            }
            out::TargetSource::UnspecifiedDefaultToHostTarget => {
                Self::UnspecifiedDefaultToHostTarget
            }
            out::TargetSource::DetectedByPkgScripts(utf8_path_buf) => {
                Self::DetectedByPkgScripts(utf8_path_buf)
            }
            out::TargetSource::DetectedByRepoGithub(utf8_path_buf) => {
                Self::DetectedByRepoGithub(utf8_path_buf)
            }
            out::TargetSource::DetectedByRepoScripts(utf8_path_buf) => {
                Self::DetectedByRepoScripts(utf8_path_buf)
            }
        }
    }
}

impl From<out::Targets> for Targets {
    fn from(value: out::Targets) -> Self {
        let out::Targets { map } = value;
        Self {
            map: map
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().map(|t| t.into()).collect()))
                .collect(),
        }
    }
}
