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
fn statistics() -> Result<()> {
    let test_suite = Repo::new("repos/os-checker-test-suite", &[], config())?;

    let outputs = test_suite.run_check()?;
    let stats = Statistics::new(&outputs);
    let tables = stats
        .iter()
        .flat_map(|s| [s.table_of_count_of_kind(), s.table_of_count_of_file()])
        .join("\n\n");

    expect_file!["./snapshots/statistics.txt"].assert_eq(&tables);

    Ok(())
}

#[test]
fn repo() -> Result<()> {
    crate::test_logger_init("assets/run_checker.log");

    let test_suite = Repo::new("repos/os-checker-test-suite", &[], config())?;
    let arceos = Repo::new("repos/arceos", &[], config())?;
    let mut resolve = arceos.resolve()?;
    resolve.extend(test_suite.resolve()?);
    let outputs: Vec<_> = resolve.iter().map(run_check).try_collect()?;

    // 对不良统计结果进行快照（由于目前功能不太完善，先记录到日志文件）
    let stats = Statistics::new(&outputs);
    let bad = stats.iter().filter(|s| !s.check_fine()).collect_vec();
    info!("bad={bad:#?}");

    // 对所有库的检查输出进行快照（路径已去除无关前缀）
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
    }).join("\n──────────────────────────────────────────────────────────────────────────────────\n");

    let current_path = Utf8PathBuf::from(".").canonicalize_utf8()?;
    let stripped_path = snapshot.replace(current_path.as_str(), ".");
    expect_file!["./snapshots/outputs.txt"].assert_eq(&stripped_path);

    Ok(())
}
