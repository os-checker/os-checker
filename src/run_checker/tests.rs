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

    let json_str = to_string_pretty(&json)?;
    expect_file!["./snapshots/json-test-suite.txt"].assert_eq(&json_str);

    let json_bytes = json_str.as_bytes();
    let json_count = jq_count(json_bytes)?;
    expect_file!["./snapshots/json-test-suite_jq_count.txt"].assert_eq(&json_count);

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

fn jq_count(json_bytes: &[u8]) -> Result<String> {
    let query = "
. as $x | .data | group_by(.idx) | map({
  cmd_idx: .[0].idx,
  length: . | length,
} | . + { cmd: $x.idx[.cmd_idx] | {package, tool,count, duration_ms} } # 为了简洁，这里去掉一些字段
  | . + { package: $x.env.packages[.cmd.package] }
  | . + { repo: $x.env.repos[.package.repo.idx] }
)
";
    let out1 = duct::cmd!("jq", query)
        .stdin_bytes(json_bytes)
        .stdout_capture()
        .run()?;

    // 若结果为 []，则表示所有输出与 count 是一致的
    let out2 = duct::cmd!(
        "jq",
        format!("{query} | map(select(.length != .cmd.count))")
    )
    .stdin_bytes(json_bytes)
    .stdout_capture()
    .run()?;

    let diff: serde_json::Value = serde_json::from_slice(&out2.stdout)?;
    ensure!(
        diff.as_array().map(|arr| arr.is_empty()).unwrap_or(false),
        "输出统计与 count 数值不一致，差异为：\n{diff:?}"
    );

    Ok(String::from_utf8(out1.stdout)?)
}
