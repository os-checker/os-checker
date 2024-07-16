use super::Config;

const YAML: &str = "
os-checker/os-checker:
  fmt: true
  clippy: cargo clippy -F a,b,c
  miri: |
    # this is a comment line
    cargo miri run # a comment
    cargo miri test -- a_test_fn
  semver-checks: false

user/repo: 
  all: true
";

#[test]
fn parse() {
    let parsed = Config::from_yaml(YAML).unwrap();
    let expected = expect_test::expect![[r#"
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
    let expected = expect_test::expect![[r#"
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
