use super::*;

impl FeaturesWithCommas {
    fn new(features: &str) -> Self {
        Self {
            features: split_features(features),
        }
    }
}

impl Features {
    fn new_simple(features: &str) -> Self {
        Features::Simple(FeaturesWithCommas::new(features))
    }

    fn new_complete(
        features: &str,
        no_default_features: bool,
        all_features: bool,
        targets: Vec<String>,
    ) -> Self {
        Features::Complete(FeaturesCompleteState {
            f: FeaturesWithCommas::new(features),
            no_default_features,
            all_features,
            targets,
        })
    }
}

fn ser_de(features: &[Features], expect: expect_test::Expect) -> Result<()> {
    let json = serde_json::to_string_pretty(features)?;
    expect.assert_eq(&json);

    // config json should be constructed from merged config
    let set: Vec<Features> = serde_json::from_str(&json)?;
    // let set: Vec<Features> = serde_json::from_str(r#"["feat3"]"#)?;
    let de_json = serde_json::to_string_pretty(&set)?;
    ensure!(
        de_json == json,
        "features can't be decoded back:\n{}",
        prettydiff::diff_lines(&json, &de_json)
    );

    Ok(())
}

#[test]
fn features_simple() -> Result<()> {
    let set1 = Features::new_simple("feat1,feat2");
    let set2 = Features::new_simple("feat3");
    ser_de(
        &[set1, set2],
        expect_test::expect![[r#"
        [
          "feat1,feat2",
          "feat3"
        ]"#]],
    )
}

#[test]
fn features_complete() -> Result<()> {
    // -F feat1,feat2
    let set1 = Features::new_complete("feat1,feat2", false, false, vec![]);
    // -F feat3 --no-default-features
    let set2 = Features::new_complete("feat3", true, false, vec![]);
    // --all-features
    let set3 = Features::new_complete("", false, true, vec![]);
    match &set3 {
        Features::Complete(c) => {
            ensure!(c.f.features.is_empty(), "{:?} is not empty", c.f.features)
        }
        Features::Simple(_) => unreachable!(),
    }
    ser_de(
        &[set1, set2, set3],
        expect_test::expect![[r#"
            [
              {
                "F": "feat1,feat2"
              },
              {
                "F": "feat3",
                "no-default-features": true
              },
              {
                "F": "",
                "all-features": true
              }
            ]"#]],
    )
}

#[test]
fn features_hybrid() -> Result<()> {
    let set1 = Features::new_simple("feat1");
    let set2 = Features::new_complete("feat2", false, false, vec![]);
    let set3 = Features::new_complete("", true, false, vec![]);
    ser_de(
        &[set1, set2, set3],
        expect_test::expect![[r#"
            [
              "feat1",
              {
                "F": "feat2"
              },
              {
                "F": "",
                "no-default-features": true
              }
            ]"#]],
    )
}
