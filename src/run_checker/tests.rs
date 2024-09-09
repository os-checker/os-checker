use super::*;
use expect_test::expect_file;
use serde_json::to_string_pretty;

fn config(json: &str) -> Config {
    Config::from_json(json).unwrap()
}

#[test]
#[instrument]
fn test_suite() -> Result<()> {
    let output: RepoOutput = config(
        r#"
{
  "file://repos/os-checker-test-suite": {
    "all": true
  }
}
"#,
    )
    .try_into()?;

    let json = JsonOutput::new(&[output]);
    let json_str = to_string_pretty(&json)?;
    expect_file!["./snapshots/json-test-suite.json"].assert_eq(&json_str);

    let json_bytes = json_str.as_bytes();
    let json_count = jq_count(json_bytes)?;
    expect_file!["./snapshots/json-test-suite_jq_count.json"].assert_eq(&json_count);

    Ok(())
}

#[test]
#[instrument]
fn arceos() -> Result<()> {
    let output: RepoOutput = config(
        r#"
{
  "file://repos/arceos": {
    "all": true
  }
}
"#,
    )
    .try_into()?;

    let json = JsonOutput::new(&[output]);
    expect_file!["./snapshots/json-arceos.json"].assert_eq(&to_string_pretty(&json)?);

    Ok(())
}

#[instrument]
fn jq_count(json_bytes: &[u8]) -> Result<String> {
    let query = "
. as $x | .data | group_by(.cmd_idx) | map({
  cmd_idx: .[0].cmd_idx,
  length: . | length,
} | . + { cmd: $x.cmd[.cmd_idx] | {package_idx, tool, count, duration_ms} } # 为了简洁，这里去掉一些字段
  | . + { package: $x.env.packages[.cmd.package_idx] }
  | . + { repo: $x.env.repos[.package.repo.repo_idx] }
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

#[test]
#[instrument]
fn lockbud_output() -> Result<()> {
    let s = super::lockbud::get_lockbud_result()?;
    println!("{s}");
    Ok(())
}
