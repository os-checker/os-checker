use super::{Config, Configs};
use crate::{layout::Packages, Result};
use expect_test::{expect, expect_file};
use itertools::Itertools;

#[test]
fn parse_a() -> Result<()> {
    let parsed = Configs::from_json_path("./tests/a.json".into())?.0;
    expect_file!["./snapshots/parse-a-json.txt"].assert_debug_eq(&parsed);

    //
    // let v = parsed[0]
    //     .config
    //     .pkg_checker_action(&Packages::test_new(&["package1", "package2"]))?;
    // expect_file!["./snapshots/parse-a-json_resolve.txt"].assert_debug_eq(&v);

    Ok(())
}

#[test]
fn resolve() -> Result<()> {
    let json = r#"
{
  "user/repo": {
    "packages": {
      "crate1": {
        "cmds": { "fmt": false }
      },
      "crate2": {
        "cmds": { "clippy": ["RUSTFLAGS=-cfg=abc cargo clippy --no-deps --message-format=json"] }
      },
      "crate3": { },
      "crate4": {
        "cmds": { "clippy": false }
      }
    }
  }
}
"#;
    let v = Config::from_json(json)?.resolve(&Packages::test_new(&[
        "crate0", "crate1", "crate2", "crate3", "crate4",
    ]))?;
    expect_file!["./snapshots/resolve.txt"].assert_debug_eq(&v);

    Ok(())
}

#[test]
fn bad_check() {
    let bad1 = r#"
{
  "user/repo": {
    "cmds": { "clippy": ["cargo miri run"] }
  }
}
"#;
    let err = format!("{}", Config::from_json(bad1).unwrap_err());
    expect!["For repo `user/repo`, `cargo miri run` doesn't contain the corresponding checker name `clippy`"].assert_eq(&err);

    let bad2 = r#"
{
  "user/repo": {
    "packages": {
      "crate1": {
        "cmds": { "clippy": "cargo miri run" }
      }
    }
  }
}
"#;
    let err = format!("{}", Config::from_json(bad2).unwrap_err());
    expect!["For pkg `crate1` in repo `user/repo`, `cargo miri run` doesn't contain the corresponding checker name `clippy`"].assert_eq(&err);
}

#[test]
fn uri() -> Result<()> {
    // 1. 本地路径以 file:// 开头，支持绝对路径和相对路径
    // 2. 任何 git repo url
    // 3. 对于 github git repo url，简化成 user/repo
    let yaml = r#"
{
  "file:///rust/my/os-checker/repos/os-checker-test-suite": { },
  "file://repos/arceos": { },
  "https://github.com/os-checker/os-checker-test-suite.git": { },
  "os-checker/os-checker": { }
}"#;
    let configs = serde_json::from_str::<Configs>(yaml)?;
    let join = configs.0.iter().map(|c| format!("{:?}", c.uri)).join("\n");
    let expected = expect![[r#"
        Local("/rust/my/os-checker/repos/os-checker-test-suite")
        Local("repos/arceos")
        Url("https://github.com/os-checker/os-checker-test-suite.git")
        Github("os-checker/os-checker")"#]];
    expected.assert_eq(&join);

    Ok(())
}

#[test]
fn merge_configs() -> Result<()> {
    let a = r#"{"user1/repo": {}, "user2/repo": { "setup": "make setup" }}"#;
    let b = r#"
{
  "user1/repo": {
    "cmds": { "fmt": false }
  },
  "user2/repo": {
    "cmds": { "clippy": "cargo clippy" }
  },
  "user3/repo": {
    "packages": {
      "a": { "targets": "x86_64-unknown-linux-gnu" }
    }
  }
}
"#;
    let configs = Configs::merge(Configs::from_json(a)?, Configs::from_json(b)?)?;
    let configs_debug = format!("{configs:#?}");
    // expect_file!["./snapshots/merge-two-jsons.txt"].assert_eq(&configs_debug);

    let json = serde_json::to_string_pretty(&configs)?;
    println!("{json}");
    let configs_debug2 = format!("{:#?}", Configs::from_json(&json)?);
    assert_eq!(configs_debug, configs_debug2);

    Ok(())
}

#[test]
fn parse_cmds() -> Result<()> {
    // single cmd
    dbg!(Config::from_json(
        r#"{"user/repo": {"cmds": {"clippy": "cargo clippy"}}}"#
    )?);
    // array of cmds
    dbg!(Config::from_json(
        r#"{"user/repo": {"cmds": {"clippy": ["cargo clippy"]}}}"#
    )?);
    Ok(())
}

#[test]
fn parse_configs() -> Result<()> {
    dbg!(Configs::from_json_path("src/config/tests/a.json".into())?);
    Ok(())
}
