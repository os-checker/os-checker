use super::{cargo_check_verbose::PackageInfo, Layout};
use crate::Result;
use cargo_metadata::camino::Utf8Path;
use expect_test::expect_file;

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
    let pkg_dir = Utf8Path::new("repos/e1000-driver/examples").canonicalize_utf8()?;
    let pkg_name = "e1000-driver-test";

    let info = PackageInfo::new(&pkg_dir, pkg_name)?;
    expect_file!["./snapshots/e1000-driver-test_package_info.txt"].assert_debug_eq(&info);

    Ok(())
}
