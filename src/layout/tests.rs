use super::LayoutOwner;
use cargo_metadata::{camino::Utf8Path, Message};
use expect_test::{expect, expect_file};
use itertools::Itertools;

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
    let current_dir = Utf8Path::new("repos/e1000-driver/examples").canonicalize_utf8()?;
    let pkg_dir = current_dir.as_str();
    let pkg_name = "e1000-driver-test";
    let target_triples = get_target_triples(pkg_dir, pkg_name)?;
    expect![[r#"
        [
            "riscv64gc-unknown-none-elf",
            "x86_64-unknown-linux-gnu",
        ]
    "#]]
    .assert_debug_eq(&target_triples);

    let mut diagnostics = Vec::with_capacity(target_triples.len());
    for target in &target_triples {
        let out = duct::cmd!(
            "cargo",
            "check",
            "--message-format=json",
            "--target",
            target
        )
        .dir(pkg_dir)
        .stdout_capture()
        .unchecked()
        .run()?;

        diagnostics.push(
            Message::parse_stream(out.stdout.as_slice())
                .filter_map(|mes| match mes.ok()? {
                    Message::CompilerMessage(mes) if mes.target.name == pkg_name => Some(mes),
                    _ => None,
                })
                .collect_vec(),
        );
    }
    expect_file!["./snapshots/check_diagnostics.txt"].assert_debug_eq(
        &diagnostics
            .iter()
            .zip(&target_triples)
            .map(|(v, t)| (t, v.iter().map(|d| d.message.to_string()).collect_vec()))
            .collect_vec(),
    );

    Ok(())
}

fn get_target_triples(pkg_dir: &str, pkg_name: &str) -> crate::Result<Vec<String>> {
    use regex::Regex;
    use std::sync::LazyLock;

    struct ExtractTriplePattern {
        pkg_name: Regex,
        manifest_dir: Regex,
        target_triple: Regex,
        running_cargo: Regex,
    }
    static RE: LazyLock<ExtractTriplePattern> = LazyLock::new(|| ExtractTriplePattern {
        pkg_name: regex::Regex::new(r#"CARGO_PKG_NAME=(\S+)"#).unwrap(),
        manifest_dir: regex::Regex::new(r#"CARGO_MANIFEST_DIR=(\S+)"#).unwrap(),
        target_triple: regex::Regex::new(r#"--target\s+(\S+)"#).unwrap(),
        running_cargo: regex::Regex::new(r#"^\s+Running `CARGO="#).unwrap(),
    });

    // NOTE: 似乎只有第一次运行 cargo check 才会强制编译所有 target triples，
    // 第二次开始运行 cargo check 之后，如果在某个 triple 上编译失败，不会编译其他 triple，
    // 这导致无法全部获取 triples 列表。因此为了避免缓存影响，清除 target dir。
    _ = duct::cmd!("cargo", "clean").dir(pkg_dir).run()?;
    let output = duct::cmd!("cargo", "check", "-vv")
        .dir(pkg_dir)
        .stderr_capture()
        .unchecked()
        .run()?;

    let target_triples = Message::parse_stream(output.stderr.as_slice())
        .filter_map(|parsed| {
            if let Message::TextLine(mes) = &parsed.ok()? {
                // 只需要当前 package 的 target triple：
                // * 需要 pkg_name 和 manifest_dir 是因为输出会产生依赖项的信息，仅有
                //   pkg_name 会造成可能的冲突（尤其 cargo check 最后才会编译当前 pkg）
                // * 实际的编译命令示例，见 https://github.com/os-checker/os-checker/commit/de95f5928a25f6b64bcf5f1964870351899f85c3
                if RE.running_cargo.is_match(mes) {
                    let crate_name = RE.pkg_name.captures(mes)?.get(1)?.as_str();
                    let manifest_dir = RE.manifest_dir.captures(mes)?.get(1)?.as_str();
                    let target_triple = RE.target_triple.captures(mes)?.get(1)?.as_str();
                    if crate_name == pkg_name && manifest_dir == pkg_dir {
                        return Some(target_triple.to_owned());
                    }
                }
            }
            None
        })
        .collect_vec();

    Ok(target_triples)
}
