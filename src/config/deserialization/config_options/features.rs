use crate::Result;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

mod type_conversion;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Features {
    Complete(FeaturesCompleteState),
    Simple(FeaturesWithCommas),
}

impl Features {
    /// Validate feature names and targets.
    pub fn validate(&self, features: &[String], targets: &[String], pkg: &str) -> Result<()> {
        let exist = |features_comma: &FeaturesWithCommas| {
            for feature in &features_comma.features {
                ensure!(
                    features.iter().any(|f| f == feature),
                    "Feature `{feature}` doesn't exist for package `{pkg}`"
                );
            }
            Ok(())
        };

        match self {
            Features::Complete(complete) => {
                exist(&complete.f)?;
                for target in &complete.targets {
                    ensure!(
                        targets.iter().any(|t| t == target),
                        "Target `{target}` isn't found or specified for package `{pkg}"
                    )
                }
                ensure!(
                    [complete.no_default_features, complete.all_features] != [true; 2], 
                    "`no_default_features` and `all_features` can't be both true in package `{pkg}`"
                );
            }
            Features::Simple(simple) => exist(simple)?,
        }
        Ok(())
    }

    pub fn to_argument(&self, target: &str) -> Vec<String> {
        let mut args = Vec::new();
        match self {
            Features::Complete(c)
                if c.targets.is_empty() || c.targets.iter().any(|t| t == target) =>
            {
                if c.no_default_features {
                    args.push("--no-default-features".to_owned());
                }
                if c.all_features {
                    args.push("--all-features".to_owned());
                }
                if !c.f.is_empty() {
                    args.push("--features".to_owned());
                    args.push(c.f.features.join(","));
                }
            }
            Features::Simple(s) if !s.is_empty() => {
                args.push("--features".to_owned());
                args.push(s.features.join(","));
            }
            Features::Complete(_) | Features::Simple(_) => (),
        }
        args
    }
}

// {
//   "features": [
//     {"F": "feat1,feat2"},
//     {"F": "feat1,feat2", "no-default-features": true},
//     {"all-features": true},
//     {"F": "feat3", "targets": ["x86_64-unknown-linux-gnu", "riscv64gc-unknown-none-elf"]},
//   ]
// }
//
// with x86_64-unknown-linux-gnu、riscv64gc-unknown-none-elf、aarch64-unknown-none =>
//
// --target x86_64-unknown-linux-gnu -F feat1,feat2
// --target x86_64-unknown-linux-gnu -F feat1,feat2 --no-default-features
// --target x86_64-unknown-linux-gnu --all-features
// --target x86_64-unknown-linux-gnu -F feat3
//
// --target riscv64gc-unknown-none-elf -F feat1,feat2
// --target riscv64gc-unknown-none-elf -F feat1,feat2 --no-default-features
// --target riscv64gc-unknown-none-elf --all-features
// --target riscv64gc-unknown-none-elf -F feat3
//
// --target aarch64-unknown-none -F feat1,feat2
// --target aarch64-unknown-none -F feat1,feat2 --no-default-features
// --target aarch64-unknown-none --all-features
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeaturesCompleteState {
    // {"F": ""} is the same as {}
    #[serde(rename = "F", default)]
    #[serde(skip_serializing_if = "FeaturesWithCommas::is_empty")]
    f: FeaturesWithCommas,

    #[serde(rename = "no-default-features", default)]
    #[serde(skip_serializing_if = "skip_false")]
    no_default_features: bool,

    #[serde(rename = "all-features", default)]
    #[serde(skip_serializing_if = "skip_false")]
    all_features: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    targets: Vec<String>,
}

fn skip_false(b: &bool) -> bool {
    !*b
}

/// -F feat1,feat2,...
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(transparent)]
pub struct FeaturesWithCommas {
    /// vec!["feat1", "feat2", ...]
    #[serde(deserialize_with = "str_to_features")]
    #[serde(serialize_with = "features_string")]
    features: Vec<String>,
}

impl FeaturesWithCommas {
    fn is_empty(&self) -> bool {
        self.features.is_empty()
    }
}

/// Convert feat1,feat2 to ["feat1", "feat2"].
fn str_to_features<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let features = String::deserialize(deserializer)?;
    Ok(split_features(&features))
}

#[allow(clippy::ptr_arg)]
fn features_string<S>(val: &Vec<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    val.join(",").serialize(serializer)
}

fn split_features(features: &str) -> Vec<String> {
    if features.is_empty() {
        return Vec::new();
    }
    features.split(',').map(String::from).collect()
}
