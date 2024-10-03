use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct RepoConfig {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<Meta>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup: Option<Setup>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targets: Option<Targets>,
    /// 暂时只作用于 repo
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_install_targets: Option<Targets>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Cmds::is_empty")]
    pub cmds: Cmds,
    #[serde(default)]
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
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

impl Cmds {
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Meta {
    /// 当它为 false 时，对所有 pkgs 禁用检查。
    /// 该选项只适用于 repo；如果在 packages 内设置，则无效
    #[serde(default = "defalt_all_packages")]
    all_packages: bool,
}

fn defalt_all_packages() -> bool {
    true
}
