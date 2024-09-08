use super::{
    detect_targets::{CargoConfigToml, CargoConfigTomlTarget, RustToolchain, RustToolchainToml},
    Layout,
};
use crate::Result;
use expect_test::{expect, expect_file};

#[test]
fn arceos_layout() {
    crate::logger::init();
    let excluded = &["tmp"];
    assert!(Layout::parse("tmp", excluded).is_err());

    let arceos = Layout::parse("./repos/arceos", excluded).unwrap();
    expect_file!["./snapshots/arceos.txt"].assert_debug_eq(&arceos);
    expect_file!["./snapshots/arceos-packages.txt"].assert_debug_eq(&arceos.packages());
}

#[test]
fn cargo_check_verbose() -> Result<()> {
    crate::logger::init();
    let layout = Layout::parse("repos/e1000-driver", &[])?;
    expect_file!["./snapshots/e1000-driver.txt"].assert_debug_eq(&layout);
    Ok(())
}

#[test]
fn cargo_config_toml_deserialize() -> Result<()> {
    expect![[r#"
        [build]
        target = ["a"]
    "#]]
    .assert_eq(&basic_toml::to_string(&CargoConfigToml::test())?);

    let s = std::fs::read("repos/e1000-driver/examples/.cargo/config.toml")?;
    let config: CargoConfigToml = basic_toml::from_slice(&s)?;
    expect![[r#"
        CargoConfigToml {
            build: BuildTarget {
                target: Multiple(
                    [
                        "x86_64-unknown-linux-gnu",
                        "riscv64gc-unknown-none-elf",
                    ],
                ),
            },
        }
    "#]]
    .assert_debug_eq(&config);
    Ok(())
}

#[test]
fn cargo_config_toml_from_child_to_root() -> Result<()> {
    let child = "repos/e1000-driver/examples/src".into();
    let root = ".".into();
    let target = CargoConfigTomlTarget::search(child, root)?;
    expect![[r#"
        Some(
            (
                Multiple(
                    [
                        "x86_64-unknown-linux-gnu",
                        "riscv64gc-unknown-none-elf",
                    ],
                ),
                "/rust/my/os-checker/repos/e1000-driver/examples/.cargo/config.toml",
            ),
        )
    "#]]
    .assert_debug_eq(&target);
    Ok(())
}

#[test]
fn rust_toolchain() -> Result<()> {
    let pkg_dir = "/rust/tmp/os-checker/kern-crates/repos/shilei-massclouds/axdtb/rt_axdtb";
    let root_dir = "/rust/tmp/os-checker/kern-crates/repos/shilei-massclouds";
    let toolchain = RustToolchain::search(pkg_dir.into(), root_dir.into())?;
    dbg!(&toolchain);

    let toolchain: RustToolchainToml = basic_toml::from_str(
        r#"
[toolchain]
channel = "nightly-2024-05-02"
"#,
    )?;
    dbg!(&toolchain);
    Ok(())
}
