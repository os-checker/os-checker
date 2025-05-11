use super::*;
use eyre::Context;
use CheckerTool::*;

mod features;
pub use self::features::Features;

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

impl Setup {
    pub fn as_slice(&self) -> &[String] {
        self.0.as_slice()
    }

    /// Run a setup cmd through bash
    /// * -l logins to respect .bashrc, and -c takes a cmd string to execute
    /// * handle bash errors
    pub fn run(&self) {
        for cmd in self.as_slice() {
            match duct::cmd!("bash", "-lc", cmd).run() {
                Ok(output) => {
                    let stdout = &*String::from_utf8_lossy(&output.stdout);
                    let stderr = &*String::from_utf8_lossy(&output.stderr);
                    if !output.status.success() {
                        error!(?cmd, stdout, stderr, "setup didn't succeed");
                    } else {
                        debug!(stdout, stderr, "setup succeeded");
                    }
                }
                Err(err) => error!(?err, "failed to a setup cmd"),
            }
        }
    }
}

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
const DISABLE: EnableOrCustom = EnableOrCustom::Enable(false);

// The map must contain checker key to validate the JSON config.
fn default_checkers(run_all_checkers: bool) -> IndexMap<CheckerTool, EnableOrCustom> {
    let state = || if run_all_checkers { ENABLED } else { DISABLE };
    indexmap::indexmap! {
        Fmt => state(),
        Clippy => state(),
        SemverChecks => state(),
        Lockbud => state(),
        Mirai => state(),
        Audit => state(),
        Rapx => state(),
        Rudra => state(),
        Outdated => state(),
        Geiger => state(),
    }
}

impl Cmds {
    /// TODO: 其他工具待完成
    pub fn new_with_all_checkers_enabled(run_all_checkers: bool) -> Self {
        Self {
            map: default_checkers(run_all_checkers),
        }
    }

    /// TODO: 其他工具待完成
    pub fn enable_all_checkers(&mut self, run_all_checkers: bool) {
        self.map = default_checkers(run_all_checkers);
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
    #[serde(default = "empty_globs")]
    only_pkg_dir_globs: MaybeMulti,

    #[serde(default = "empty_globs")]
    skip_pkg_dir_globs: MaybeMulti,

    /// { "target1": { "ENV1": "val" } }
    #[serde(default)]
    pub target_env: TargetEnv,

    #[serde(default)]
    pub rerun: bool,

    #[serde(default)]
    pub use_last_cache: bool,

    #[serde(default = "run_all_checkers")]
    pub run_all_checkers: bool,
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
    pub fn only_pkg_dir_globs(&self) -> Box<[glob::Pattern]> {
        self.only_pkg_dir_globs
            .as_slice()
            .iter()
            .filter_map(|s| glob_pattern(s).ok())
            .collect()
    }

    pub fn check_only_pkg_dir_globs(&self) -> Result<()> {
        for s in self.only_pkg_dir_globs.as_slice() {
            glob_pattern(s)?;
        }
        Ok(())
    }

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

fn empty_globs() -> MaybeMulti {
    MaybeMulti::Multi(vec![])
}

pub fn run_all_checkers() -> bool {
    true
}

impl Default for Meta {
    fn default() -> Self {
        Self {
            only_pkg_dir_globs: empty_globs(),
            skip_pkg_dir_globs: empty_globs(),
            target_env: TargetEnv::default(),
            rerun: false,
            use_last_cache: false,
            run_all_checkers: run_all_checkers(),
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
