use super::*;
use expect_test::expect;

#[test]
fn custom_target() -> Result<()> {
    let target = "x86_64-unknown-linux-gnu";
    // --target x86_64-unknown-linux-gnu 和 --target    x86_64-unknown-linux-gnu 产生一样的结果
    let line = "cargo clippy --target x86_64-unknown-linux-gnu";
    let (_, mut words) = parse_cmd(line)?;
    expect![[r#"
            [
                "cargo",
                "clippy",
                "--target",
                "x86_64-unknown-linux-gnu",
            ]
        "#]]
    .assert_debug_eq(&words);
    assert_eq!(target, extract_target(&words).unwrap_or("???"));
    assert_eq!(
        target,
        set_toolchain_and_target(&mut words, "???", None).unwrap_or_default()
    );

    // 但 --target=... 不一样
    let line = "cargo clippy --target=x86_64-unknown-linux-gnu";
    let (_, mut words) = parse_cmd(line)?;
    expect![[r#"
            [
                "cargo",
                "clippy",
                "--target=x86_64-unknown-linux-gnu",
            ]
        "#]]
    .assert_debug_eq(&words);
    assert_eq!(target, extract_target(&words).unwrap_or("???"));
    assert_eq!(
        target,
        set_toolchain_and_target(&mut words, "???", None).unwrap_or_default()
    );

    Ok(())
}

#[test]
fn custom_without_target() -> Result<()> {
    let line = "cargo clippy";
    let (_, mut words) = parse_cmd(line)?;

    let target = "riscv64gc-unknown-none-elf";
    assert_eq!("???", extract_target(&words).unwrap_or("???"));
    assert_eq!(
        "",
        set_toolchain_and_target(&mut words, target, None).unwrap_or_default()
    );
    assert_eq!(target, extract_target(&words).unwrap_or("???"));

    Ok(())
}

#[test]
fn custom_cmd() {
    let pkg = &Pkg {
        name: "nothing",
        dir: cargo_metadata::camino::Utf8Path::new("."),
        target: "x86_64-unknown-linux-gnu",
        toolchain: Some(0),
        env: Default::default(),
        audit: None,
        is_lib: true,
    };
    expect![[r#"
        Resolve {
            pkg_name: "nothing",
            pkg_dir: ".",
            target: "x86_64-unknown-linux-gnu",
            target_overriden: false,
            checker: Fmt,
            cmd: "cargo fmt --target=x86_64-unknown-linux-gnu --check",
            expr: Io(
                Dir(
                    ".",
                ),
                Cmd(
                    [
                        "cargo",
                        "fmt",
                        "--target=x86_64-unknown-linux-gnu",
                        "--check",
                    ],
                ),
            ),
        }
    "#]]
    .assert_debug_eq(&custom("cargo fmt --check", pkg, CheckerTool::Fmt).unwrap());

    expect![[r#"
        Resolve {
            pkg_name: "nothing",
            pkg_dir: ".",
            target: "x86_64-unknown-linux-gnu",
            target_overriden: false,
            checker: Clippy,
            cmd: "cargo clippy --target=x86_64-unknown-linux-gnu -F a,b,c -F e,f",
            expr: Io(
                Env(
                    "RUST_LOG",
                    "debug",
                ),
                Io(
                    Env(
                        "RUSTFLAGS",
                        "--cfg unstable",
                    ),
                    Io(
                        Dir(
                            ".",
                        ),
                        Cmd(
                            [
                                "cargo",
                                "clippy",
                                "--target=x86_64-unknown-linux-gnu",
                                "-F",
                                "a,b,c",
                                "-F",
                                "e,f",
                            ],
                        ),
                    ),
                ),
            ),
        }
    "#]]
    .assert_debug_eq(
        // 这里指定 -F 的方式可能是错误的，但目的是测试环境变量和引号处理
        &custom(
            r#"RUSTFLAGS="--cfg unstable" RUST_LOG=debug cargo clippy -F a,b,c -F "e,f" "#,
            pkg,
            CheckerTool::Clippy,
        )
        .unwrap(),
    );
}
