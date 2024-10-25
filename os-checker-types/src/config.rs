use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Encode, Decode, Default, Clone)]
pub struct RepoConfig {
    pub meta: Option<Meta>,
    pub setup: Option<Setup>,
    pub targets: Option<Targets>,
    pub no_install_targets: Option<Targets>,
    #[musli(with = musli::serde)]
    pub cmds: Cmds,
    #[musli(with = musli::serde)]
    pub packages: IndexMap<String, RepoConfig>,
}

#[derive(Serialize, Deserialize, Encode, Decode, Clone)]
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

#[derive(Serialize, Deserialize, Encode, Decode, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Encode, Decode, Clone)]
pub struct Targets(pub MaybeMulti);

#[derive(Debug, Serialize, Deserialize, Encode, Decode, Clone)]
pub struct Setup(pub MaybeMulti);

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Cmds {
    pub map: IndexMap<crate::CheckerTool, EnableOrCustom>,
}

impl Cmds {
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize, Encode, Decode, Clone)]
pub struct Meta {
    pub skip_packages_globs: MaybeMulti,
}
