use super::RepoConfig;
use indexmap::IndexMap;

/// Targets specified from JSON config.
pub struct TargetsSpecifed<'a> {
    pub repo: &'a [String],
    pub pkgs: IndexMap<&'a str, &'a [String]>,
}

impl RepoConfig {
    fn targets(&self) -> &[String] {
        self.targets.as_ref().map(|t| t.as_slice()).unwrap_or(&[])
    }

    pub fn targets_specified(&self) -> TargetsSpecifed {
        let repo = self.targets();
        let pkgs = self
            .packages
            .iter()
            .filter_map(|(name, config)| {
                let targets = config.targets();
                (!targets.is_empty()).then_some((name.as_str(), targets))
            })
            .collect();
        TargetsSpecifed { repo, pkgs }
    }
}
