#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
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
            }
            Features::Simple(simple) => exist(&simple.features)?,
        }
        Ok(())
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
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct FeaturesCompleteState {
    #[serde(rename = "F")]
    f: FeaturesWithCommas,
    #[serde(rename = "no-default-features", default)]
    no_default_features: bool,
    #[serde(rename = "all-features", default)]
    all_features: bool,
    #[serde(default)]
    targets: Vec<String>,
}

/// -F feat1,feat2,...
#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(transparent)]
struct FeaturesWithCommas {
    /// vec!["feat1", "feat2", ...]
    #[serde(deserialize_with = "str_to_features")]
    features: Vec<String>,
}

/// Convert feat1,feat2 to ["feat1", "feat2"].
fn str_to_features<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let features = <&str>::deserialize(deserializer)?;
    Ok(features.split(',').map(String::from).collect())
}
