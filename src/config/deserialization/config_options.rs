use super::*;
use eyre::Context;
use CheckerTool::*;

mod type_conversion;

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(untagged)]
pub enum EnableOrCustom {
    Enable(bool),
    Single(String),
    Multi(Vec<String>),
}

impl Debug for EnableOrCustom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Enable(b) => b.fmt(f),
            Self::Single(s) => s.fmt(f),
            Self::Multi(v) => v.fmt(f),
        }
    }
}

impl EnableOrCustom {
    /// 检查自定义命令是否包含 checker name：
    /// 当返回值为 Some 时，表示不包含，并返回这个 checker name；
    /// 当返回值为 None 时，表示检查通过，该命令包含 checker name。
    #[instrument(level = "trace")]
    pub fn validate_checker_name(&self, checker: &str) -> Result<(), &str> {
        match self {
            EnableOrCustom::Single(s) if !s.contains(checker) => Err(s),
            EnableOrCustom::Multi(v) => {
                for s in v {
                    if !s.contains(checker) {
                        return Err(s);
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn cmd(&self) -> either::Either<bool, &[String]> {
        match self {
            EnableOrCustom::Enable(b) => either::Left(*b),
            EnableOrCustom::Single(s) => either::Right(std::slice::from_ref(s)),
            EnableOrCustom::Multi(v) => either::Right(v),
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(untagged)]
pub enum MaybeMulti {
    Single(String),
    Multi(Vec<String>),
}

impl MaybeMulti {
    pub fn as_slice(&self) -> &[String] {
        match self {
            MaybeMulti::Single(s) => std::slice::from_ref(s),
            MaybeMulti::Multi(v) => v,
        }
    }
}

impl Debug for MaybeMulti {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single(s) => s.fmt(f),
            Self::Multi(v) => v.fmt(f),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Targets(MaybeMulti);

impl Targets {
    pub fn as_slice(&self) -> &[String] {
        self.0.as_slice()
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Setup(MaybeMulti);

// impl Setup {
//     pub fn as_slice(&self) -> &[String] {
//         self.0.as_slice()
//     }
// }

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
#[serde(transparent)]
pub struct Cmds {
    map: IndexMap<CheckerTool, EnableOrCustom>,
}

// TODO: remove me
impl JsonSchema for Cmds {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Cmds")
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        serde_json::Map::<String, serde_json::Value>::json_schema(generator)
    }
}

const ENABLED: EnableOrCustom = EnableOrCustom::Enable(true);

impl Cmds {
    /// TODO: 其他工具待完成
    pub fn new_with_all_checkers_enabled() -> Self {
        Self {
            map: indexmap::indexmap! {
                Fmt => ENABLED,
                Clippy => ENABLED,
                SemverChecks => ENABLED,
                Lockbud => ENABLED,
                Mirai => ENABLED,
                Audit => ENABLED,
                Rapx => ENABLED,
                Rudra => ENABLED,
                Outdated => ENABLED,
                Geiger => ENABLED,
            },
        }
    }

    /// TODO: 其他工具待完成
    pub fn enable_all_checkers(&mut self) {
        for checker in [
            Fmt,
            Clippy,
            SemverChecks,
            Lockbud,
            Mirai,
            Audit,
            Rapx,
            Rudra,
            Outdated,
            Geiger,
        ] {
            self.entry(checker)
                .and_modify(|cmd| *cmd = ENABLED)
                .or_insert(ENABLED);
        }
    }

    /// Override self by setting values from the other,
    /// or keep the original value if the other doesn't set it.
    pub fn merge(&mut self, other: &Self) {
        self.extend(other.iter().map(|(&key, val)| (key, val.clone())));
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl std::ops::Deref for Cmds {
    type Target = IndexMap<CheckerTool, EnableOrCustom>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl std::ops::DerefMut for Cmds {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Meta {
    #[serde(default = "defalt_skip_pkg_dir_globs")]
    skip_pkg_dir_globs: MaybeMulti,
    /// { "target1": { "ENV1": "val" } }
    #[serde(default)]
    pub target_env: TargetEnv,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone, Default)]
#[serde(transparent)]
pub struct TargetEnv {
    map: IndexMap<String, Env>,
}

impl TargetEnv {
    pub fn merge(
        &self,
        target: &str,
        global: &IndexMap<String, String>,
    ) -> IndexMap<String, String> {
        let mut map = global.clone();
        if let Some(env) = self.map.get(target) {
            // override env if exists
            map.extend(env.map.clone());
        }
        map
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(transparent)]
struct Env {
    map: IndexMap<String, String>,
}

impl Meta {
    pub fn skip_pkg_dir_globs(&self) -> Box<[glob::Pattern]> {
        self.skip_pkg_dir_globs
            .as_slice()
            .iter()
            .filter_map(|s| glob_pattern(s).ok())
            .collect()
    }

    pub fn check_skip_pkg_dir_globs(&self) -> Result<()> {
        for s in self.skip_pkg_dir_globs.as_slice() {
            glob_pattern(s)?;
        }
        Ok(())
    }
}

fn glob_pattern(s: &str) -> Result<glob::Pattern> {
    glob::Pattern::new(s).with_context(|| format!("{s} is not a valid glob pattern."))
}

fn defalt_skip_pkg_dir_globs() -> MaybeMulti {
    MaybeMulti::Multi(vec![])
}

impl Default for Meta {
    fn default() -> Self {
        Self {
            skip_pkg_dir_globs: defalt_skip_pkg_dir_globs(),
            target_env: TargetEnv::default(),
        }
    }
}

#[test]
fn target_env() {
    let s = r#"
{
  "target_env": {
    "target1": { "ENV1": "val" },
    "target2": { "ENV2": "val", "ENV3": "val" }
  }
}"#;

    let meta: Meta = serde_json::from_str(s).unwrap();
    dbg!(&meta);
}
