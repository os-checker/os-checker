//! 配置文件的合并转换成 JSON 的操作应在 os-checker CLI 中执行，而不应该使用这里的类型。
//! 因为 os-checker 中的类型具有额外的 serde 属性，但这里的类型不使用 serde 属性。

use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Encode, Decode, Default, Clone)]
pub struct RepoConfig {
    pub meta: Option<Meta>,
    pub setup: Option<Setup>,
    pub targets: Option<Targets>,
    pub no_install_targets: Option<Targets>,
    #[musli(with = musli::serde)]
    pub features: Option<Vec<Features>>,
    #[musli(with = musli::serde)]
    pub env: Option<IndexMap<String, String>>,
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

impl Default for MaybeMulti {
    fn default() -> Self {
        MaybeMulti::Multi(Vec::new())
    }
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Features {
    Complete(FeaturesCompleteState),
    Simple(FeaturesWithCommas),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeaturesWithCommas {
    pub features: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeaturesCompleteState {
    pub f: FeaturesWithCommas,
    pub no_default_features: bool,
    pub all_features: bool,
    pub targets: Vec<String>,
}

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
    #[serde(default = "empty_globs")]
    pub only_pkg_dir_globs: MaybeMulti,
    #[serde(default = "empty_globs")]
    pub skip_pkg_dir_globs: MaybeMulti,
    /// { "target1": { "ENV1": "val" } }
    #[serde(default)]
    #[musli(with = musli::serde)]
    pub target_env: TargetEnv,
    #[serde(default)]
    pub rerun: bool,
    #[serde(default)]
    pub use_last_cache: bool,
}

fn empty_globs() -> MaybeMulti {
    MaybeMulti::Multi(vec![])
}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(transparent)]
pub struct TargetEnv {
    pub map: IndexMap<String, Env>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(transparent)]
pub struct Env {
    pub map: IndexMap<String, String>,
}
