use super::{Cmds, Meta, Setup, Targets};
use os_checker_types::config as out;

impl From<Targets> for out::Targets {
    fn from(value: Targets) -> Self {
        out::Targets(value.0.into())
    }
}

impl From<Setup> for out::Setup {
    fn from(value: Setup) -> Self {
        out::Setup(value.0.into())
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
        let Meta { all_packages } = value;
        Self { all_packages }
    }
}
