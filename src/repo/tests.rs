use super::Config;
use crate::{layout::Packages, Result};
use expect_test::{expect, expect_file};
use itertools::Itertools;

const YAML: &str = "
os-checker/os-checker:
  fmt: true
  clippy: cargo clippy -F a,b,c
  miri: |
    # this is a comment line
    cargo miri run # a comment
    cargo miri test -- a_test_fn
  semver-checks: false
  # non-existent key-value pair is ignored
  non-existent: pair

user/repo: 
  all: true # enable all tools for all packages, but ...
  lockbud: false # except lockbud for all packages
  packages: # packages are the union of all members across all workspaces
    crate1: 
      miri: false # except miri for crate1
    crate2:
      semver-checks: false # except semver-checks for crate2
";

#[test]
fn parse_basic() -> Result<()> {
    let parsed = Config::from_yaml(YAML)?;
    expect_file!["./snapshots/basic-config.txt"].assert_debug_eq(&parsed);

    let v: Vec<_> = parsed
        .iter()
        .map(|c| (&c.uri, c.config.checker_action().unwrap()))
        .collect();
    expect_file!["./snapshots/basic-config-checker_action.txt"].assert_debug_eq(&v);

    Ok(())
}

#[test]
fn pkg_checker_action() -> Result<()> {
    let parsed = Config::from_yaml(YAML)?;
    let v = parsed[0]
        .config
        .pkg_checker_action(&Packages::test_new(&["package1", "package2"]))?;
    expect_file!["./snapshots/pkg_checker_action-basic.txt"].assert_debug_eq(&v);

    Ok(())
}

#[test]
fn pkg_checker_action_only_fmt_clippy() -> Result<()> {
    let yaml = r#"
user/repo:
  all: true
  packages:
    crate1:
      fmt: false
    crate2:
      clippy: RUSTFLAGS="-cfg abc" cargo clippy
    crate3:
      all: false
    crate4:
      clippy: false
"#;
    let v = Config::from_yaml(yaml)?[0]
        .config
        .pkg_checker_action(&Packages::test_new(&[
            "crate0", "crate1", "crate2", "crate3", "crate4",
        ]))?;
    expect_file!["./snapshots/pkg_checker_action-fmt_clippy_only.txt"].assert_debug_eq(&v);

    Ok(())
}

#[test]
fn bad_check() {
    let bad1 = "
user/repo: 
  clippy: cargo miri run
";
    let err = format!("{}", Config::from_yaml(bad1).unwrap_err());
    expect!["命令 `cargo miri run` 与检查工具 `clippy` 不匹配"].assert_eq(&err);

    let bad2 = "
user/repo: 
  packages:
    crate1: 
      clippy: cargo miri run
";
    let err = format!("{}", Config::from_yaml(bad2).unwrap_err());
    // FIXME: 或许可以更好的错误报告，比如在哪个仓库哪个库的命令上不匹配
    expect!["命令 `cargo miri run` 与检查工具 `clippy` 不匹配"].assert_eq(&err);
}

#[test]
fn uri() -> Result<()> {
    let yaml = "
# 本地路径以 file:// 开头，支持绝对路径和相对路径
file:///path/to/os-checker-test-suite:
  all: true
file://repos/arceos:
  all: true

# 任何 git repo url
https://github.com/os-checker/os-checker.git:
  all: true

# 对于 github git repo url，简化成 user/repo
os-checker/os-checker:
  all: true
";
    let configs = Config::from_yaml(yaml)?;
    let join = configs.iter().map(|c| format!("{:?}", c.uri)).join("\n");
    let expected = expect![[r#"
        Uri { tag: Local("/path/to/os-checker-test-suite"), local: "os-checker-test-suite", _local_tmp_dir: None, key: "file:///path/to/os-checker-test-suite" }
        Uri { tag: Local("repos/arceos"), local: "arceos", _local_tmp_dir: None, key: "file://repos/arceos" }
        Uri { tag: Url("https://github.com/os-checker/os-checker.git"), local: "os-checker", _local_tmp_dir: None, key: "https://github.com/os-checker/os-checker.git" }
        Uri { tag: Github("os-checker/os-checker"), local: "os-checker", _local_tmp_dir: None, key: "os-checker/os-checker" }"#]];
    expected.assert_eq(&join);

    Ok(())
}
