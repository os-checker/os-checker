use super::*;
use analysis::Statistics;
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
fn test_suite() -> Result<()> {
    let test_suite = Repo::new("repos/os-checker-test-suite", &[], config())?;
    let outputs = test_suite.run_check()?;

    let tables = stat_tables(&outputs);
    expect_file!["./snapshots/statistics-test-suite.txt"].assert_eq(&tables);

    let snapshot = snapshot_outputs(&outputs)?;
    expect_file!["./snapshots/outputs-test-suite.txt"].assert_eq(&snapshot);

    Ok(())
}

#[test]
fn arceos() -> Result<()> {
    crate::test_logger_init("assets/run_checker.log");

    let arceos = Repo::new("repos/arceos", &[], config())?;
    let resolve = arceos.resolve()?;
    let outputs: Vec<_> = resolve.iter().map(run_check).try_collect()?;

    let tables = stat_tables(&outputs);
    expect_file!["./snapshots/statistics-arceos.txt"].assert_eq(&tables);

    let snapshot = snapshot_outputs(&outputs)?;
    expect_file!["./snapshots/outputs-arceos.txt"].assert_eq(&snapshot);

    Ok(())
}

/// 对不良统计结果进行快照
fn stat_tables(outputs: &[Output]) -> String {
    Statistics::new(outputs)
        .iter()
        .filter(|s| !s.check_fine())
        .flat_map(|s| [s.table_of_count_of_kind(), s.table_of_count_of_file()])
        .join("\n\n")
}

/// 对所有库的检查输出进行快照（路径已去除无关前缀）
fn snapshot_outputs(outputs: &[Output]) -> Result<String> {
    let sep =
        "\n──────────────────────────────────────────────────────────────────────────────────\n";
    let snapshot = outputs.iter()
        .map(|output| {
            let count = output.count;
            let checker = output.checker;
            let package_name = &output.package_name;
            let success = output.raw.status.success();
            let diagnostics = output.parsed.test_diagnostics();

            debug!(
                "[success={success} count={count}] {package_name} with {checker:?} checking in {}ms",
                output.duration_ms
            );

            format!("[{package_name} with {checker:?} checking] success={success} count={count} diagnostics=\n{diagnostics}")
        })
        .join(sep);
    let current_path = Utf8PathBuf::from(".").canonicalize_utf8()?;
    Ok(snapshot.replace(current_path.as_str(), "."))
}
