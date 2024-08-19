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
    let re = regex::Regex::new(r#"^\s+Running `CARGO="#)?;
    let reader = duct::cmd!("cargo", "check", "-vv")
        // .dir("/rust/tmp/os-checker/e1000-driver/examples/")
        .stderr_to_stdout()
        .reader()?;
    // CARGO_CRATE_NAME=os_checker
    // CARGO_MANIFEST_DIR=/rust/my/os-checker
    let messages = cargo_metadata::Message::parse_stream(std::io::BufReader::new(reader))
        .filter_map(|parsed| {
            parsed.ok().and_then(|line| {
                if let cargo_metadata::Message::TextLine(line) = line {
                    if re.is_match(&line) {
                        return Some(line);
                    }
                }
                None
            })
        })
        .collect_vec();
    expect_file!["./snapshots/cargo-check-verbose.txt"].assert_debug_eq(&messages);

    let re_crate_name = regex::Regex::new(r#"CARGO_CRATE_NAME=(\S+)"#)?;
    let re_manifest_dir = regex::Regex::new(r#"CARGO_MANIFEST_DIR=(\S+)"#)?;

    let krate = messages
        .iter()
        .filter_map(|mes| {
            let crate_name = re_crate_name
                .captures(mes)
                .and_then(|cap| Some(cap.get(1)?.as_str()));
            let manifest_dir = re_manifest_dir
                .captures(mes)
                .and_then(|cap| Some(cap.get(1)?.as_str()));
            crate_name.zip(manifest_dir)
        })
        .collect_vec();
    expect![[r#"
        [
            (
                "os_checker",
                "/rust/my/os-checker",
            ),
        ]
    "#]]
    .assert_debug_eq(&krate);

    Ok(())
}
