use super::*;
use expect_test::expect_file;
use serde_json::to_string_pretty;

fn config(yaml: &str) -> Config {
    Config::from_yaml(yaml).unwrap().pop().unwrap()
}

#[test]
fn test_suite() -> Result<()> {
    let output: RepoOutput = config(
        "
file://repos/os-checker-test-suite:
  all: true
",
    )
    .try_into()?;

    let mut json = JsonOutput::new();
    output.with_json_output(&mut json);
    expect_file!["./snapshots/json-test-suite.txt"].assert_eq(&to_string_pretty(&json)?);

    Ok(())
}

#[test]
fn arceos() -> Result<()> {
    let output: RepoOutput = config(
        "
file://repos/arceos:
  all: true
",
    )
    .try_into()?;

    let mut json = JsonOutput::new();
    output.with_json_output(&mut json);
    expect_file!["./snapshots/json-arceos.txt"].assert_eq(&to_string_pretty(&json)?);

    Ok(())
}

// #[test]
// fn local_and_github() -> Result<()> {
//     use rayon::prelude::*;
//
//     // 该测试只写入日志文件
//     fn logging(configs: Vec<Config>) -> Result<()> {
//         debug!(?configs);
//         let repos: Vec<_> = configs
//             .into_par_iter()
//             .map(Repo::try_from)
//             .collect::<Result<_>>()?;
//         for repo in &repos {
//             trace!(?repo);
//             let stat = repo.outputs_and_statistics()?;
//             for s in stat.iter().filter(|s| !s.check_fine()) {
//                 let count_on_file = s.table_of_count_of_file();
//                 let count_on_kind = s.table_of_count_of_kind();
//                 info!("\n{count_on_file}\n{count_on_kind}");
//             }
//         }
//         Ok(())
//     }
//
//     crate::logger::test_init("assets/run_checker-github.log");
//
//     let yaml = std::fs::read_to_string("assets/repos.yaml")?;
//     logging(Config::from_yaml(&yaml)?)?;
//
//     let yaml = "
// os-checker/os-checker-test-suite:
//   all: true
// ";
//     logging(Config::from_yaml(yaml)?)?;
//
//     Ok(())
// }
