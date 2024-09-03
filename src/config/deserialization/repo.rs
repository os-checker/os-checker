use super::*;

#[derive(Serialize, Deserialize)]
#[serde(try_from = "serde_json::Value")]
pub enum Repo {
    Plain(String),
    Config { name: String, config: RepoConfig },
}

impl Repo {
    fn validate(&self) -> Result<()> {
        match self {
            Repo::Plain(_) => Ok(()),
            Repo::Config { name, config } => config.validate_checker_name(name),
        }
    }

    pub fn into_name_and_config(self) -> (String, RepoConfig) {
        match self {
            Repo::Plain(name) => (name, RepoConfig::default()),
            Repo::Config { name, config } => (name, config),
        }
    }

    pub fn from_json(json: &str) -> Result<(String, RepoConfig)> {
        let repo: Repo = serde_json::from_str(json)?;
        Ok(repo.into_name_and_config())
    }
}

impl TryFrom<Value> for Repo {
    type Error = eyre::Error;

    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        if let Value::Object(obj) = value {
            // assert_eq!(config.len(), 1);
            if let Some((name, deserializer)) = obj.into_iter().next() {
                if let Ok(config) = RepoConfig::deserialize(deserializer) {
                    let repo = Repo::Config { name, config };
                    repo.validate()?;
                    return Ok(repo);
                }
            }
        }
        let s = r#"Should be an object like `{"user/repo": {...}}`"#;
        bail!("{s}")
    }
}

impl Debug for Repo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plain(s) => s.fmt(f),
            Self::Config { name, config } => f.debug_map().entry(name, config).finish(),
        }
    }
}
