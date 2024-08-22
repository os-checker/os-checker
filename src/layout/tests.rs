use super::Layout;
use crate::Result;
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
    crate::logger::init();
    let layout = Layout::parse("repos/e1000-driver", &[])?;
    expect_file!["./snapshots/e1000-driver.txt"].assert_debug_eq(&layout);
    Ok(())
}
