use super::{Config, Configs};
use crate::Result;
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
fn pkg_checker_action_only_fmt_clippy() -> Result<()> {
    let json = r#"
{
  "user/repo": {
    "all": true,
    "packages": {
      "crate1": {
        "fmt": false
      },
      "crate2": {
        "clippy": ["RUSTFLAGS=-cfg=abc cargo clippy"]
      },
      "crate3": {
        "all": false
      },
      "crate4": {
        "clippy": false
      }
    }
  }
}
"#;
    // let v = Config::from_json(json)?
    //     .config
    //     .pkg_checker_action(&Packages::test_new(&[
    //         "crate0", "crate1", "crate2", "crate3", "crate4",
    //     ]))?;
    // expect_file!["./snapshots/pkg_checker_action-fmt_clippy_only.txt"].assert_debug_eq(&v);

    Ok(())
}

#[test]
fn bad_check() {
    let bad1 = r#"
{
  "user/repo": {
    "clippy": ["cargo miri run"]
  }
}
"#;
    let err = format!("{}", Config::from_json(bad1).unwrap_err());
    expect!["命令 `cargo miri run` 与检查工具 `clippy` 不匹配"].assert_eq(&err);

    let bad2 = r#"
{
  "user/repo": {
    "packages": {
      "crate1": {
        "clippy": "cargo miri run"
      }
    }
  }
}
"#;
    let err = format!("{}", Config::from_json(bad2).unwrap_err());
    // FIXME: 或许可以更好的错误报告，比如在哪个仓库哪个库的命令上不匹配
    expect!["命令 `cargo miri run` 与检查工具 `clippy` 不匹配"].assert_eq(&err);
}

#[test]
fn uri() -> Result<()> {
    // 1. 本地路径以 file:// 开头，支持绝对路径和相对路径
    // 2. 任何 git repo url
    // 3. 对于 github git repo url，简化成 user/repo
    let yaml = r#"
{
  "file:///rust/my/os-checker/repos/os-checker-test-suite": {
    "all": true
  },
  "file://repos/arceos": {
    "all": true
  },
  "https://github.com/os-checker/os-checker-test-suite.git": {
    "all": true
  },
  "os-checker/os-checker": {
    "all": true
  }
}"#;
    let configs = serde_json::from_str::<Configs>(yaml)?;
    let join = configs.0.iter().map(|c| format!("{:?}", c.uri)).join("\n");
    let expected = expect![[r#"
        Uri { tag: Local("/rust/my/os-checker/repos/os-checker-test-suite"), user: "repos", repo: "os-checker-test-suite", _local_tmp_dir: None, key: "file:///rust/my/os-checker/repos/os-checker-test-suite" }
        Uri { tag: Local("repos/arceos"), user: "repos", repo: "arceos", _local_tmp_dir: None, key: "file://repos/arceos" }
        Uri { tag: Url("https://github.com/os-checker/os-checker-test-suite.git"), user: "os-checker", repo: "os-checker-test-suite", _local_tmp_dir: None, key: "https://github.com/os-checker/os-checker-test-suite.git" }
        Uri { tag: Github("os-checker/os-checker"), user: "os-checker", repo: "os-checker", _local_tmp_dir: None, key: "os-checker/os-checker" }"#]];
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
