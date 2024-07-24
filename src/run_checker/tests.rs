use super::*;
use expect_test::expect_file;

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
    let outputs: Vec<_> = resolve.iter().map(run_check).try_collect()?;

    expect_file!["./snapshots/analysis.txt"].assert_debug_eq(&analysis::Statistics::new(&outputs));

    let snapshot = outputs.iter().map(|output|  {
        let count = output.count;
        let checker = output.checker;
        let package_name = &output.package_name;
        let success = output.raw.status.success();
        let diagnostics = output.parsed.test_diagnostics();

        debug!(
            "[success={success} count={count}] {package_name} with {checker:?} checking in {}ms",
            output.duration_ms
        );

        format!(
            "[{package_name} with {checker:?} checking] success={success} count={count} diagnostics=\n{diagnostics}",
        )
    }).collect_vec();

    let current_path = Utf8PathBuf::from(".").canonicalize_utf8()?;
    let join = snapshot
        .join("\n──────────────────────────────────────────────────────────────────────────────────\n")
        .replace(current_path.as_str(), ".");
    expect_file!["./snapshots/outputs.txt"].assert_eq(&join);

    Ok(())
}
