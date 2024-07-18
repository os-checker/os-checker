use super::Config;
use expect_test::expect;

const YAML: &str = "
os-checker/os-checker:
  fmt: true
  clippy: cargo clippy -F a,b,c
  miri: |
    # this is a comment line
    cargo miri run # a comment
    cargo miri test -- a_test_fn
  semver-checks: false # TODO
  # non-existent key-value pair is ignored
  non-existent: pair

user/repo: 
  all: true # comment
";

#[test]
fn parse() {
    let parsed = Config::from_yaml(YAML).unwrap();
    let expected = expect![[r#"
        [
            Config {
                repo: "os-checker/os-checker",
                config: RepoConfig {
                    fmt: Perform(
                        true,
                    ),
                    clippy: Steps(
                        [
                            "cargo clippy -F a,b,c",
                        ],
                    ),
                    miri: Steps(
                        [
                            "cargo miri run",
                            "cargo miri test -- a_test_fn",
                        ],
                    ),
                },
            },
            Config {
                repo: "user/repo",
                config: RepoConfig {
                    all: Perform(
                        true,
                    ),
                },
            },
        ]
    "#]];
    expected.assert_debug_eq(&parsed);

    let v: Vec<_> = parsed
        .iter()
        .map(|c| (&c.repo, c.config.to_vec()))
        .collect();
    let expected = expect![[r#"
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
                        Steps(
                            [
                                "cargo clippy -F a,b,c",
                            ],
                        ),
                    ),
                    (
                        Miri,
                        Steps(
                            [
                                "cargo miri run",
                                "cargo miri test -- a_test_fn",
                            ],
                        ),
                    ),
                ],
            ),
            (
                "user/repo",
                [
                    (
                        All,
                        Perform(
                            true,
                        ),
                    ),
                ],
            ),
        ]
    "#]];
    expected.assert_debug_eq(&v);
}

const BAD: &str = "
user/repo: 
  clippy: cargo miri run
";

#[test]
fn check() {
    let err = format!("{}", Config::from_yaml(BAD).unwrap_err());
    let expected = expect!["命令 `cargo miri run` 与检查工具 `clippy` 不匹配"];
    expected.assert_eq(&err);
}
