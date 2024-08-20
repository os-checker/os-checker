use super::{
    cargo_check_verbose::{CargoCheckDiagnostics, DefaultTargetTriples},
    LayoutOwner,
};
use crate::Result;
use cargo_metadata::camino::Utf8Path;
use expect_test::{expect, expect_file};

#[test]
fn arceos_layout() {
    crate::logger::init();
    let excluded = &["tmp"];
    assert!(LayoutOwner::new("tmp", excluded).is_err());

    let arceos = LayoutOwner::new("./repos/arceos", excluded).unwrap();
    expect_file!["./snapshots/arceos.txt"].assert_debug_eq(&arceos);
    expect_file!["./snapshots/arceos-packages.txt"].assert_debug_eq(&arceos.packages());
}

#[test]
fn cargo_check_verbose() -> Result<()> {
    let current_dir = Utf8Path::new("repos/e1000-driver/examples").canonicalize_utf8()?;
    let pkg_dir = current_dir.as_str();
    let pkg_name = "e1000-driver-test";

    let DefaultTargetTriples { targets, .. } = DefaultTargetTriples::new(pkg_dir, pkg_name)?;
    expect![[r#"
        [
            "riscv64gc-unknown-none-elf",
            "x86_64-unknown-linux-gnu",
        ]
    "#]]
    .assert_debug_eq(&targets);

    let diagnostics: Vec<_> = targets
        .iter()
        .map(|target| CargoCheckDiagnostics::new(pkg_dir, pkg_name, target))
        .collect::<Result<_>>()?;
    expect_file!["./snapshots/check_diagnostics.txt"].assert_debug_eq(&diagnostics);

    Ok(())
}
