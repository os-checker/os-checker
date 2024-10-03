use super::config_options::{EnableOrCustom, MaybeMulti};
use os_checker_types::config as out;

impl From<super::RepoConfig> for out::RepoConfig {
    fn from(value: super::RepoConfig) -> Self {
        let super::RepoConfig {
            meta,
            setup,
            targets,
            no_install_targets,
            cmds,
            packages,
        } = value;
        out::RepoConfig {
            meta: meta.map(|m| m.into()),
            setup: setup.map(|s| s.into()),
            targets: targets.map(|t| t.into()),
            no_install_targets: no_install_targets.map(|t| t.into()),
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
