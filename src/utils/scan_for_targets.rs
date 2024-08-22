use super::{cmd, Itertools, LazyLock, Result, Utf8PathBuf};
use regex::bytes::Regex;

pub fn scan_scripts_for_target(
    files: &[Utf8PathBuf],
    mut f: impl FnMut(&str, Utf8PathBuf),
) -> Result<()> {
    let mut buffer = Vec::with_capacity(1024);
    for file in files {
        std::io::copy(&mut std::fs::File::open(file)?, &mut buffer)?;
        for target in extract(&buffer) {
            f(target, file.to_owned());
        }
        buffer.clear();
    }
    Ok(())
}

fn pattern_target_list() -> String {
    let target_list = cmd!("rustc", "--print=target-list").read().unwrap();
    let target_formatter = target_list
        .lines()
        .format_with("|", |target, f| f(&format_args!("({target})")));
    format!(r"(?-u:\b){target_formatter}(?-u:\b)")
}

// e.g. static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?-u:\b)(x86_64-unknown-linux-gnu)(?-u:\b)"));
static RE: LazyLock<Regex> = LazyLock::new(|| {
    let pattern = pattern_target_list();
    Regex::new(&pattern)
        .inspect(|r| {
            debug!(static_captures_len = r.static_captures_len(), %pattern);
        })
        .unwrap()
});

fn extract(src: &[u8]) -> impl Iterator<Item = &'_ str> {
    RE.captures_iter(src)
        .filter_map(|c| std::str::from_utf8(c.extract::<1>().1.first()?).ok())
}

#[test]
fn targets() {
    let s = r#""x86_64-unknown-linux-gnu" aarch64-apple-darwin
aarch64-unknown-linux-gnu， riscv64gc-unknown-none-elf|x86_64-win7-windows-msvc|
"#;
    let found = extract(s.as_bytes()).collect_vec();
    expect_test::expect![[r#"
        [
            "x86_64-unknown-linux-gnu",
            "aarch64-apple-darwin",
            "aarch64-unknown-linux-gnu",
            "riscv64gc-unknown-none-elf",
            "x86_64-win7-windows-msvc",
        ]
    "#]]
    .assert_debug_eq(&found);
}
