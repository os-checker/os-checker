use super::{Config, Configs};
use crate::{layout::Packages, Result};
use expect_test::{expect, expect_file};
use itertools::Itertools;

const JSON_PATH: &str = "src/config/tests/a.json";
const LARGE_JSON_PATH: &str = "src/config/tests/large.json";

#[test]
fn parse_assets() -> Result<()> {
    Configs::from_json_path("assets/repos-ui.json".into())?;
    Configs::from_json_path("assets/repos-default.json".into())?;
    Ok(())
}

#[test]
fn parse_and_resolve() -> Result<()> {
    let configs = Configs::from_json_path(JSON_PATH.into())?;
    expect_file!["./snapshots/parse-a-json.txt"].assert_debug_eq(&configs.0);

    let v = configs.0[0].resolve(&Packages::test_new(&["package1", "package2"]))?;
    expect_file!["./snapshots/parse-a-json_resolve.txt"].assert_debug_eq(&v);

    expect_file!["./snapshots/batch1.txt"].assert_debug_eq(&configs.chunk(1));

    Ok(())
}

fn make_batch(f: impl FnOnce(usize) -> usize) -> Vec<Configs> {
    let configs = Configs::from_json_path(LARGE_JSON_PATH.into()).unwrap();
    let size = f(configs.0.len());
    configs.chunk(size)
}

#[test]
fn batch() -> Result<()> {
    let batches = make_batch(|len| len / 2);
    expect_file!["./snapshots/large-batch-split-by-half.txt"].assert_debug_eq(&batches);

    let batches = make_batch(|_| 3);
    expect_file!["./snapshots/large-batch-3.txt"].assert_debug_eq(&batches);

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

    let bad3 = r#"
{
  "user/repo": { "cmds": { "miri": false } }
}
"#;
    let err = format!(
        "{}",
        Config::from_json(bad3)
            .unwrap()
            .resolve(&Packages::test_new(&["a"]))
            .unwrap_err()
            .source()
            .unwrap()
    );
    expect!["Checker `miri` is not supported in cmds of repo `user/repo`"].assert_eq(&err);

    let bad4 = r#"
{
  "a/b": {
    "cmds": { "clippy:": "cargo clippy --no-deps" }
  }
}
"#;
    let err = format!("{}", Config::from_json(bad4).unwrap_err());
    // FIXME: 这里没有将错误指向 cmds 内
    expect![[r#"Should be an object like `{"user/repo": {...}}`"#]].assert_eq(&err);
}

#[test]
fn uri() -> Result<()> {
    // 1. 本地路径以 file:// 开头，支持绝对路径和相对路径
    // 2. 任何 git repo url
    // 3. 对于 github git repo url，简化成 user/repo
    let json = r#"
{
  "file:///rust/my/os-checker/repos/os-checker-test-suite": { },
  "file://repos/arceos": { },
  "https://github.com/os-checker/os-checker-test-suite.git": { },
  "os-checker/os-checker": { }
}"#;
    let configs = serde_json::from_str::<Configs>(json)?;
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
    let configs = Configs::merge(Configs::from_json(a)?, Configs::from_json(b)?);
    let configs_debug = format!("{configs:#?}");
    expect_file!["./snapshots/merge-configs.txt"].assert_eq(&configs_debug);

    let json = serde_json::to_string_pretty(&configs)?;
    expect_file!["./snapshots/merged-jsons.txt"].assert_eq(&json);
    let configs_debug2 = format!("{:#?}", Configs::from_json(&json)?);
    assert_eq!(configs_debug, configs_debug2);

    Ok(())
}

#[test]
fn parse_cmds() -> Result<()> {
    expect_file!["./snapshots/single-cmd.txt"].assert_debug_eq(&Config::from_json(
        r#"{"user/repo": {"cmds": {"clippy": "cargo clippy"}}}"#,
    )?);
    expect_file!["./snapshots/array-of-cmds.txt"].assert_debug_eq(&Config::from_json(
        r#"{"user/repo": {"cmds": {"clippy": ["cargo clippy"]}}}"#,
    )?);
    Ok(())
}
