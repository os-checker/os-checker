use super::{RepoConfig, Targets};
use indexmap::IndexMap;

/// Targets specified from JSON config.
pub struct TargetsSpecifed<'a> {
    pub repo: &'a [String],
    pub pkgs: IndexMap<&'a str, &'a [String]>,
    pub no_install: &'a [String],
}

impl RepoConfig {
    fn targets(&self) -> &[String] {
        targets(&self.targets)
    }

    pub fn targets_specified(&self) -> TargetsSpecifed<'_> {
        let repo = self.targets();
        let pkgs = self
            .packages
            .iter()
            .filter_map(|(name, config)| {
                let targets = config.targets();
                (!targets.is_empty()).then_some((name.as_str(), targets))
            })
            .collect();
        let no_install = targets(&self.no_install_targets);
        TargetsSpecifed {
            repo,
            pkgs,
            no_install,
        }
    }
}

fn targets(t: &Option<Targets>) -> &[String] {
    t.as_ref().map(|t| t.as_slice()).unwrap_or(&[])
}
