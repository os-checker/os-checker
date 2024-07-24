use super::*;

fn config() -> Config {
    let yaml = "
arceos:
  all: true
  miri: false
";
    Config::from_yaml(yaml).unwrap().pop().unwrap()
}

#[test]
fn repo() -> Result<()> {
    crate::test_logger_init("assets/run_checker.log");
    let test_suite = Repo::new("repos/os-checker-test-suite", &[], config())?;
    let arceos = Repo::new("repos/arceos", &[], config())?;
    let mut resolve = arceos.resolve()?;
    resolve.extend(test_suite.resolve()?);
    let mut snapshot = Vec::with_capacity(resolve.len());
    for res in resolve.iter() {
        let output = run_check(res)?;

        let success = output.raw.status.success();
        let count = output.count;
        let diagnostics = output.parsed.test_diagnostics();

        snapshot.push(format!(
            "[{} with {:?} checking] success={success} count={count} diagnostics=\n{diagnostics}",
            res.package.name, res.checker
        ));

        debug!(
            "[success={success} count={count}] {} with {:?} checking in {}ms",
            res.package.name, res.checker, output.duration_ms
        );
    }

    let current_path = Utf8PathBuf::from(".").canonicalize_utf8()?;
    let join = snapshot
        .join("\n──────────────────────────────────────────────────────────────────────────────────\n")
        .replace(current_path.as_str(), ".");
    expect_test::expect_file!["./snapshots/outputs.txt"].assert_eq(&join);

    Ok(())
}
