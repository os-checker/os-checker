use super::LayoutOwner;
use expect_test::expect_file;

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
        .stderr_to_stdout()
        .reader()?;
    let messages: Vec<_> = cargo_metadata::Message::parse_stream(std::io::BufReader::new(reader))
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
        .dedup()
        .collect();
    expect_file!["./snapshots/cargo-check-verbose.txt"].assert_debug_eq(&messages);
    Ok(())
}
