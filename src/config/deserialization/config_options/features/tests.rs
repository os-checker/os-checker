use super::*;

#[cfg(test)]
impl FeaturesWithCommas {
    fn new(features: &str) -> Self {
        Self {
            features: split_features(features),
        }
    }
}

#[cfg(test)]
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
    let set1 = Features::Simple(FeaturesWithCommas::new("feat1,feat2"));
    let set2 = Features::Simple(FeaturesWithCommas::new("feat3"));
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
    let set1 = Features::Complete(FeaturesCompleteState {
        f: FeaturesWithCommas::new("feat1,feat2"),
        no_default_features: false,
        all_features: false,
        targets: vec![],
    });
    // -F feat3 --no-default-features
    let set2 = Features::Complete(FeaturesCompleteState {
        f: FeaturesWithCommas::new("feat3"),
        no_default_features: true,
        all_features: false,
        targets: vec![],
    });
    // --all-features
    let set3 = Features::Complete(FeaturesCompleteState {
        f: FeaturesWithCommas::new(""),
        no_default_features: false,
        all_features: true,
        targets: vec![],
    });
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
