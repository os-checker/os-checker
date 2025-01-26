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

    fn is_empty(&self) -> bool {
        match self {
            Features::Complete(c) => c.f.features.is_empty(),
            Features::Simple(s) => s.features.is_empty(),
        }
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
    ensure!(set3.is_empty(), "features in {set3:?} is not empty");
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
                "no-default-features": true
              }
            ]"#]],
    )
}

// {"F": ""} is the same as {} and ""
#[test]
fn features_empty_string() -> Result<()> {
    let set1 = Features::new_simple("");
    ensure!(set1.is_empty(), "Features in {set1:?} is not empty");
    ser_de(
        &[set1],
        expect_test::expect![[r#"
        [
          ""
        ]"#]],
    )?;

    let set2 = Features::new_complete("", false, false, vec![]);
    ensure!(set2.is_empty(), "Features in {set2:?} is not empty");
    ser_de(
        &[set2.clone()],
        expect_test::expect![[r#"
        [
          {}
        ]"#]],
    )?;

    let empty1: Features = serde_json::from_str("{}")?;
    let empty2: Features = serde_json::from_str(r#"{"F": ""}"#)?;
    ensure!(empty1.is_empty(), "Features in {empty1:?} is not empty");
    ensure!(empty2.is_empty(), "Features in {empty2:?} is not empty");
    assert_eq!(format!("{empty1:?}"), format!("{empty2:?}"));
    assert_eq!(format!("{set2:?}"), format!("{empty2:?}"));

    Ok(())
}

#[test]
fn features_arguments_empty() -> Result<()> {
    ensure!(
        Features::new_simple("").to_argument("").is_empty(),
        "empty features should yields empty arguments"
    );
    ensure!(
        Features::new_complete("", false, false, vec![])
            .to_argument("")
            .is_empty(),
        "empty features should yields empty arguments"
    );

    ensure!(
        Features::new_complete(
            "",
            false,
            false,
            vec!["x86_64-unknown-linux-gnu".to_owned()]
        )
        .to_argument("x86_64-unknown-linux-gnu")
        .is_empty(),
        "empty features should yields empty arguments"
    );

    Ok(())
}

#[test]
fn features_arguments() -> Result<()> {
    assert_eq!(
        Features::new_simple("feat1").to_argument(""),
        ["--features", "feat1"]
    );
    assert_eq!(
        Features::new_simple("feat1,feat2").to_argument(""),
        ["--features", "feat1,feat2"]
    );

    assert_eq!(
        Features::new_complete("feat1,feat2", true, false, vec![]).to_argument(""),
        ["--no-default-features", "--features", "feat1,feat2"]
    );

    assert_eq!(
        Features::new_complete("", false, true, vec![]).to_argument(""),
        ["--all-features"]
    );

    Ok(())
}

#[test]
fn features_target() -> Result<()> {
    let targets = vec![
        "x86_64-unknown-linux-gnu".to_owned(),
        "riscv64gc-unknown-none-elf".to_owned(),
    ];

    assert_eq!(
        Features::new_complete("", false, true, targets.clone())
            .to_argument("x86_64-unknown-linux-gnu"),
        ["--all-features"]
    );

    assert!(Features::new_complete("", false, true, targets.clone())
        .to_argument("unsupported target")
        .is_empty());

    assert_eq!(
        Features::new_complete("feat1", true, false, targets.clone())
            .to_argument("riscv64gc-unknown-none-elf"),
        ["--no-default-features", "--features", "feat1"]
    );

    Ok(())
}
