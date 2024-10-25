use super::{Cmds, Meta, Setup, Targets};
use os_checker_types::config as out;

// ********** CLI => os_checker_types **********

impl From<Targets> for out::Targets {
    fn from(value: Targets) -> Self {
        Self(value.0.into())
    }
}

impl From<Setup> for out::Setup {
    fn from(value: Setup) -> Self {
        Self(value.0.into())
    }
}

impl From<Cmds> for out::Cmds {
    fn from(value: Cmds) -> Self {
        Self {
            map: value
                .map
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }
}

impl From<Meta> for out::Meta {
    fn from(value: Meta) -> Self {
        let Meta {
            skip_packages_globs,
        } = value;
        Self {
            skip_packages_globs: skip_packages_globs.into(),
        }
    }
}

// ********** os_checker_types => CLI **********

impl From<out::Targets> for Targets {
    fn from(value: out::Targets) -> Self {
        Self(value.0.into())
    }
}

impl From<out::Setup> for Setup {
    fn from(value: out::Setup) -> Self {
        Self(value.0.into())
    }
}

impl From<out::Cmds> for Cmds {
    fn from(value: out::Cmds) -> Self {
        Self {
            map: value
                .map
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }
}

impl From<out::Meta> for Meta {
    fn from(value: out::Meta) -> Self {
        let out::Meta {
            skip_packages_globs,
        } = value;
        Self {
            skip_packages_globs: skip_packages_globs.into(),
        }
    }
}
