use super::{
    config_options::{EnableOrCustom, MaybeMulti},
    RepoConfig,
};
use os_checker_types::config as out;

// ********** CLI => os_checker_types **********

impl From<RepoConfig> for out::RepoConfig {
    fn from(value: RepoConfig) -> Self {
        let RepoConfig {
            meta,
            setup,
            targets,
            no_install_targets,
            env,
            cmds,
            packages,
        } = value;
        Self {
            meta: meta.map(|m| m.into()),
            setup: setup.map(|s| s.into()),
            targets: targets.map(|t| t.into()),
            no_install_targets: no_install_targets.map(|t| t.into()),
            env,
            cmds: cmds.into(),
            packages: packages.into_iter().map(|(k, v)| (k, v.into())).collect(),
        }
    }
}

impl From<EnableOrCustom> for out::EnableOrCustom {
    fn from(value: EnableOrCustom) -> Self {
        match value {
            EnableOrCustom::Enable(b) => Self::Enable(b),
            EnableOrCustom::Single(s) => Self::Single(s),
            EnableOrCustom::Multi(v) => Self::Multi(v),
        }
    }
}

impl From<MaybeMulti> for out::MaybeMulti {
    fn from(value: MaybeMulti) -> Self {
        match value {
            MaybeMulti::Single(s) => Self::Single(s),
            MaybeMulti::Multi(v) => Self::Multi(v),
        }
    }
}

// ********** os_checker_types => CLI **********

impl From<out::RepoConfig> for RepoConfig {
    fn from(value: out::RepoConfig) -> Self {
        let out::RepoConfig {
            meta,
            setup,
            targets,
            no_install_targets,
            env,
            cmds,
            packages,
        } = value;
        Self {
            meta: meta.map(|m| m.into()),
            setup: setup.map(|s| s.into()),
            targets: targets.map(|t| t.into()),
            no_install_targets: no_install_targets.map(|t| t.into()),
            env,
            cmds: cmds.into(),
            packages: packages.into_iter().map(|(k, v)| (k, v.into())).collect(),
        }
    }
}

impl From<out::EnableOrCustom> for EnableOrCustom {
    fn from(value: out::EnableOrCustom) -> Self {
        match value {
            out::EnableOrCustom::Enable(b) => Self::Enable(b),
            out::EnableOrCustom::Single(s) => Self::Single(s),
            out::EnableOrCustom::Multi(v) => Self::Multi(v),
        }
    }
}

impl From<out::MaybeMulti> for MaybeMulti {
    fn from(value: out::MaybeMulti) -> Self {
        match value {
            out::MaybeMulti::Single(s) => Self::Single(s),
            out::MaybeMulti::Multi(v) => Self::Multi(v),
        }
    }
}
