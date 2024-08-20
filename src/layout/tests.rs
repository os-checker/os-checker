use super::LayoutOwner;
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
fn cargo_check_verbose() -> crate::Result<()> {
    use itertools::Itertools;

    let current_dir = "/rust/tmp/os-checker/e1000-driver/examples";
    _ = duct::cmd!("cargo", "clean").dir(current_dir).run()?;
    let output = duct::cmd!("cargo", "check", "-vv")
        .dir(current_dir)
        .stderr_capture()
        .unchecked()
        .run()?;

    let current_pkg_name = "e1000-driver-test";

    // 不建议使用 CRATE_NAME，它会把连字符转换成下划线
    let re_pkg_name = regex::Regex::new(r#"CARGO_PKG_NAME=(\S+)"#)?;
    let re_manifest_dir = regex::Regex::new(r#"CARGO_MANIFEST_DIR=(\S+)"#)?;
    let re_target_triple = regex::Regex::new(r#"--target\s+(\S+)"#)?;
    let re_running_cargo = regex::Regex::new(r#"^\s+Running `CARGO="#)?;

    let target_triples = cargo_metadata::Message::parse_stream(output.stderr.as_slice())
        .filter_map(|parsed| {
            if let cargo_metadata::Message::TextLine(mes) = &parsed.ok()? {
                if re_running_cargo.is_match(mes) {
                    let crate_name = re_pkg_name.captures(mes)?.get(1)?.as_str();
                    let manifest_dir = re_manifest_dir.captures(mes)?.get(1)?.as_str();
                    let target_triple = re_target_triple.captures(mes)?.get(1)?.as_str();
                    if crate_name == current_pkg_name && manifest_dir == current_dir {
                        return Some(target_triple.to_owned());
                    }
                }
            }
            None
        })
        .collect_vec();
    expect![[r#"
        [
            "riscv64gc-unknown-none-elf",
            "x86_64-unknown-linux-gnu",
        ]
    "#]]
    .assert_debug_eq(&target_triples);

    Ok(())
}
