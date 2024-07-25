use super::Config;
use crate::{layout::Package, Result};
use expect_test::expect;

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
fn parse() -> Result<()> {
    let parsed = Config::from_yaml(YAML)?;
    expect![[r#"
        [
            Config {
                repo: "os-checker/os-checker",
                config: RepoConfig {
                    fmt: Perform(
                        true,
                    ),
                    clippy: Lines(
                        [
                            "cargo clippy -F a,b,c",
                        ],
                    ),
                    miri: Lines(
                        [
                            "cargo miri run",
                            "cargo miri test -- a_test_fn",
                        ],
                    ),
                    semver-checks: Perform(
                        false,
                    ),
                },
            },
            Config {
                repo: "user/repo",
                config: RepoConfig {
                    all: Perform(
                        true,
                    ),
                    lockbud: Perform(
                        false,
                    ),
                    packages: {
                        "crate1": RepoConfig {
                            miri: Perform(
                                false,
                            ),
                        },
                        "crate2": RepoConfig {
                            semver-checks: Perform(
                                false,
                            ),
                        },
                    },
                },
            },
        ]
    "#]]
    .assert_debug_eq(&parsed);

    let v: Vec<_> = parsed
        .iter()
        .map(|c| (&c.repo, c.config.checker_action().unwrap()))
        .collect();
    expect![[r#"
        [
            (
                "os-checker/os-checker",
                [
                    (
                        Fmt,
                        Perform(
                            true,
                        ),
                    ),
                    (
                        Clippy,
                        Lines(
                            [
                                "cargo clippy -F a,b,c",
                            ],
                        ),
                    ),
                    (
                        Miri,
                        Lines(
                            [
                                "cargo miri run",
                                "cargo miri test -- a_test_fn",
                            ],
                        ),
                    ),
                    (
                        SemverChecks,
                        Perform(
                            false,
                        ),
                    ),
                ],
            ),
            (
                "user/repo",
                [
                    (
                        Lockbud,
                        Perform(
                            false,
                        ),
                    ),
                    (
                        Miri,
                        Perform(
                            false,
                        ),
                    ),
                    (
                        SemverChecks,
                        Perform(
                            false,
                        ),
                    ),
                ],
            ),
        ]
    "#]]
    .assert_debug_eq(&v);

    Ok(())
}

#[test]
fn pkg_checker_action() -> Result<()> {
    let parsed = Config::from_yaml(YAML)?;
    let v = parsed[0]
        .config
        .pkg_checker_action(&Package::test_new(["package1", "package2"]))?;
    expect![[r#"
        [
            Resolve {
                package: Package {
                    name: "package1",
                    cargo_toml: "./Cargo.toml",
                    workspace_root (file name): "unknown???",
                },
                checker: Fmt,
                expr: Cmd(
                    [
                        "cargo",
                        "fmt",
                        "--check",
                        "--manifest-path",
                        "./Cargo.toml",
                    ],
                ),
            },
            Resolve {
                package: Package {
                    name: "package1",
                    cargo_toml: "./Cargo.toml",
                    workspace_root (file name): "unknown???",
                },
                checker: Clippy,
                expr: Io(
                    Dir(
                        ".",
                    ),
                    Cmd(
                        [
                            "cargo",
                            "clippy",
                            "-F",
                            "a,b,c",
                        ],
                    ),
                ),
            },
            Resolve {
                package: Package {
                    name: "package2",
                    cargo_toml: "./Cargo.toml",
                    workspace_root (file name): "unknown???",
                },
                checker: Fmt,
                expr: Cmd(
                    [
                        "cargo",
                        "fmt",
                        "--check",
                        "--manifest-path",
                        "./Cargo.toml",
                    ],
                ),
            },
            Resolve {
                package: Package {
                    name: "package2",
                    cargo_toml: "./Cargo.toml",
                    workspace_root (file name): "unknown???",
                },
                checker: Clippy,
                expr: Io(
                    Dir(
                        ".",
                    ),
                    Cmd(
                        [
                            "cargo",
                            "clippy",
                            "-F",
                            "a,b,c",
                        ],
                    ),
                ),
            },
        ]
    "#]]
    .assert_debug_eq(&v);

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
        .pkg_checker_action(&Package::test_new([
            "crate0", "crate1", "crate2", "crate3", "crate4",
        ]))?;
    expect![[r#"
        [
            Resolve {
                package: Package {
                    name: "crate0",
                    cargo_toml: "./Cargo.toml",
                    workspace_root (file name): "unknown???",
                },
                checker: Fmt,
                expr: Cmd(
                    [
                        "cargo",
                        "fmt",
                        "--check",
                        "--manifest-path",
                        "./Cargo.toml",
                    ],
                ),
            },
            Resolve {
                package: Package {
                    name: "crate0",
                    cargo_toml: "./Cargo.toml",
                    workspace_root (file name): "unknown???",
                },
                checker: Clippy,
                expr: Cmd(
                    [
                        "cargo",
                        "clippy",
                        "--no-deps",
                        "--manifest-path",
                        "./Cargo.toml",
                    ],
                ),
            },
            Resolve {
                package: Package {
                    name: "crate1",
                    cargo_toml: "./Cargo.toml",
                    workspace_root (file name): "unknown???",
                },
                checker: Clippy,
                expr: Cmd(
                    [
                        "cargo",
                        "clippy",
                        "--no-deps",
                        "--manifest-path",
                        "./Cargo.toml",
                    ],
                ),
            },
            Resolve {
                package: Package {
                    name: "crate2",
                    cargo_toml: "./Cargo.toml",
                    workspace_root (file name): "unknown???",
                },
                checker: Fmt,
                expr: Cmd(
                    [
                        "cargo",
                        "fmt",
                        "--check",
                        "--manifest-path",
                        "./Cargo.toml",
                    ],
                ),
            },
            Resolve {
                package: Package {
                    name: "crate2",
                    cargo_toml: "./Cargo.toml",
                    workspace_root (file name): "unknown???",
                },
                checker: Clippy,
                expr: Io(
                    Dir(
                        ".",
                    ),
                    Io(
                        Env(
                            "RUSTFLAGS",
                            "-cfg abc",
                        ),
                        Cmd(
                            [
                                "cargo",
                                "clippy",
                            ],
                        ),
                    ),
                ),
            },
            Resolve {
                package: Package {
                    name: "crate4",
                    cargo_toml: "./Cargo.toml",
                    workspace_root (file name): "unknown???",
                },
                checker: Fmt,
                expr: Cmd(
                    [
                        "cargo",
                        "fmt",
                        "--check",
                        "--manifest-path",
                        "./Cargo.toml",
                    ],
                ),
            },
        ]
    "#]]
    .assert_debug_eq(&v);

    Ok(())
}

#[test]
fn bad_check() {
    let bad1 = "
user/repo: 
  clippy: cargo miri run
";
    let err = format!("{}", Config::from_yaml(bad1).unwrap_err());
    let expected = expect!["命令 `cargo miri run` 与检查工具 `clippy` 不匹配"];
    expected.assert_eq(&err);

    let bad2 = "
user/repo: 
  packages:
    crate1: 
      clippy: cargo miri run
";
    let err = format!("{}", Config::from_yaml(bad2).unwrap_err());
    // FIXME: 或许可以更好的错误报告，比如在哪个仓库哪个库的命令上不匹配
    let expected = expect!["命令 `cargo miri run` 与检查工具 `clippy` 不匹配"];
    expected.assert_eq(&err);
}

#[test]
fn parse_repos() -> Result<()> {
    let yaml = std::fs::read_to_string("assets/repos.yaml")?;
    let parsed = Config::from_yaml(&yaml)?;
    dbg!(&parsed);

    Ok(())
}
