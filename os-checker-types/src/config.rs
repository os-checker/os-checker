use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoConfig {
    meta: Option<Meta>,
    pub setup: Option<Setup>,
    pub targets: Option<Targets>,
    /// 暂时只作用于 repo
    pub no_install_targets: Option<Targets>,
    pub cmds: Cmds,
    pub packages: IndexMap<String, RepoConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum EnableOrCustom {
    Enable(bool),
    Single(String),
    Multi(Vec<String>),
}

impl fmt::Debug for EnableOrCustom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Enable(b) => b.fmt(f),
            Self::Single(s) => s.fmt(f),
            Self::Multi(v) => v.fmt(f),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum MaybeMulti {
    Single(String),
    Multi(Vec<String>),
}

impl fmt::Debug for MaybeMulti {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single(s) => s.fmt(f),
            Self::Multi(v) => v.fmt(f),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Targets(MaybeMulti);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Setup(MaybeMulti);

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(transparent)]
pub struct Cmds {
    map: IndexMap<crate::CheckerTool, EnableOrCustom>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Meta {
    /// 当它为 false 时，对所有 pkgs 禁用检查。
    /// 该选项只适用于 repo；如果在 packages 内设置，则无效
    all_packages: bool,
}
