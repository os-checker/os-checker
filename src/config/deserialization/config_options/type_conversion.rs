use super::{Cmds, Env, Meta, Setup, TargetEnv, Targets};
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
            only_pkg_dir_globs,
            skip_pkg_dir_globs,
            target_env,
        } = value;
        Self {
            only_pkg_dir_globs: only_pkg_dir_globs.into(),
            skip_pkg_dir_globs: skip_pkg_dir_globs.into(),
            target_env: target_env.into(),
        }
    }
}

impl From<TargetEnv> for out::TargetEnv {
    fn from(value: TargetEnv) -> Self {
        out::TargetEnv {
            map: value.map.into_iter().map(|(k, v)| (k, v.into())).collect(),
        }
    }
}

impl From<Env> for out::Env {
    fn from(value: Env) -> Self {
        out::Env { map: value.map }
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
            only_pkg_dir_globs,
            skip_pkg_dir_globs,
            target_env,
        } = value;
        Self {
            only_pkg_dir_globs: only_pkg_dir_globs.into(),
            skip_pkg_dir_globs: skip_pkg_dir_globs.into(),
            target_env: target_env.into(),
        }
    }
}

impl From<out::TargetEnv> for TargetEnv {
    fn from(value: out::TargetEnv) -> Self {
        TargetEnv {
            map: value.map.into_iter().map(|(k, v)| (k, v.into())).collect(),
        }
    }
}

impl From<out::Env> for Env {
    fn from(value: out::Env) -> Self {
        Env { map: value.map }
    }
}
