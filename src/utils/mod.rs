use crate::Result;
use cargo_metadata::camino::{Utf8Path, Utf8PathBuf};
use duct::cmd;
use eyre::Context;
use std::{path::Path, time::Instant};

mod scan_for_targets;
pub use scan_for_targets::scan_scripts_for_target;

/// Dirs or files to be excluded.
mod exlucded;
pub use exlucded::{empty, Exclude};

mod installation;
pub use installation::{
    init as installation_init, install_toolchain, rustup_target_add, rustup_target_add_for_checkers,
};

/// 特殊的编译目标，os-checker 目前不支持在这上面运行检查。
pub const PECULIAR_TARGETS: &[&str] = &["x86_64-fuchsia", "avr-unknown-gnu-atmega328"];

/// 本机工具链（目前假设 CI 的运行平台）
pub const HOST_TARGET: &str = "x86_64-unknown-linux-gnu";

/// 检查工具固定的工具链
pub const PLUS_TOOLCHAIN_HOST: &str = "+nightly";

pub const PLUS_TOOLCHAIN_RUDRA: &str = "+nightly-2021-10-21";
pub const PLUS_TOOLCHAIN_MIRAI: &str = "+nightly-2025-01-10";
pub const PLUS_TOOLCHAIN_LOCKBUD: &str = "+nightly-2025-02-01";
pub const PLUS_TOOLCHAIN_ATOMVCHECKER: &str = "+nightly-2023-03-09";
pub const PLUS_TOOLCHAIN_RAP: &str = "+nightly-2024-10-12";

/// git clone 一个仓库到一个 dir。
/// 如果该仓库已存在，则 git pull 拉取最新的代码。
pub fn git_clone(dir: &Utf8Path, url: &str) -> Result<(std::process::Output, u64)> {
    let now = std::time::Instant::now();
    let output = if dir.exists() {
        cmd!("git", "pull", "--recurse-submodules").dir(dir).run()?
    } else {
        cmd!("git", "clone", "--recursive", url, dir).run()?
    };
    let millis = now.elapsed().as_millis() as u64;
    ensure!(
        output.status.success(),
        "git 获取 {url:?} 失败\nstderr={}\nstdout={}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );
    Ok((output, millis))
}

/// 遍历一个目录及其子目录的所有文件（但不进入 .git 和 target 目录）：
/// * 需要设置一个最大递归深度（虽然可以不设置这个条件，但大部分情况下，os-checker 不需要深度递归）
/// * op_on_file 为一个回调函数，其参数保证为一个文件路径，且返回值为 Some 时表示把它的值推到 Vec
///
/// NOTE: all the given paths and output paths are relative to repo root.
/// Be carefult when using it by comparing with absolute paths.
pub fn walk_dir<T, E: Exclude>(
    dir: impl AsRef<Path>,
    max_depth: usize,
    dirs_excluded: E,
    only_dirs: &[glob::Pattern],
    mut op_on_file: impl FnMut(Utf8PathBuf) -> Option<T>,
) -> Vec<T> {
    let dir = dir.as_ref();
    walkdir::WalkDir::new(dir)
        .max_depth(max_depth) // 目录递归上限
        .into_iter()
        .filter_entry(move |entry| {
            // 别进入这些文件夹（适用于子目录递归）
            const NO_JUMP_IN: &[&str] = &[".git", "target"];
            let filename = entry.file_name().to_str().unwrap();
            let exclude = NO_JUMP_IN.exclude(filename) || dirs_excluded.exclude(filename);
            !exclude
        })
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.file_type().is_file() {
                return None;
            }

            let path = Utf8PathBuf::try_from(entry.into_path()).ok()?;

            // Empty only dir means accepting all paths.
            // Since any returns false for empty iterator,
            // need to check emptiness skip.
            if only_dirs.is_empty() || {
                // only accept these matched path
                let path_str = path.strip_prefix(dir).unwrap_or(&*path).as_str();
                only_dirs.iter().any(|pat| pat.matches(path_str))
            } {
                op_on_file(path)
            } else {
                None
            }
        })
        .collect()
}

/// Perform an operation and get the execution time.
pub fn execution_time_ms<T>(op: impl FnOnce() -> T) -> (time::OffsetDateTime, u64, T) {
    let now_utc = time::OffsetDateTime::now_utc();
    let now = Instant::now();
    let value = op();
    let duration_ms = now.elapsed().as_millis() as u64;
    (now_utc, duration_ms, value)
}

// ignore_fail means when the cmd returns error, still reads stdout.
// It must be set to true when a checker use exit code to indicate bad diagnostics.
// pub fn cmd_run(bin: &str, args: &[&str], dir: &Utf8Path, ignore_fail: bool) -> Result<String> {
//     let _span = error_span!("cmd_run", bin, ?args).entered();
//
//     let output = cmd(bin, args)
//         .dir(dir)
//         .unchecked()
//         .stdout_capture()
//         .stderr_capture()
//         .run()?;
//
//     if !ignore_fail && !output.status.success() {
//         let raw_err = String::from_utf8_lossy(&output.stderr);
//         bail!("raw_err={raw_err}");
//     }
//
//     String::from_utf8(output.stdout).with_context(|| "stdout contains invalid UTF-8 chars")
// }

/// Same as `cmd_run`, but takes a callback to modify the cmd.
pub fn cmd_run_with(
    bin: &str,
    args: &[&str],
    dir: &Utf8Path,
    ignore_fail: bool,
    with: impl FnOnce(duct::Expression) -> duct::Expression,
) -> Result<String> {
    let _span = error_span!("cmd_run", bin, ?args).entered();

    let output = with(
        cmd(bin, args)
            .dir(dir)
            .unchecked()
            .stdout_capture()
            .stderr_capture(),
    )
    .run()?;

    if !ignore_fail && !output.status.success() {
        let raw_err = String::from_utf8_lossy(&output.stderr);
        bail!("raw_err={raw_err}");
    }

    String::from_utf8(output.stdout).with_context(|| "stdout contains invalid UTF-8 chars")
}

#[test]
fn test_walk_dir() {
    // NOTE: `**/os-checker-database/**` means all contents under os-checker-database dir,
    // but not including os-checker-database itself.
    // To exclude `os-checker-database` dir, specify `**/os-checker-database`.
    let dirs_excluded = [exlucded::pat("**/os-checker*")];
    let cargo_tomls = walk_dir(".", 3, dirs_excluded, &[], |file| {
        (file.file_name() == Some("Cargo.toml")).then_some(file)
    });
    dbg!(&cargo_tomls);
    assert!(!cargo_tomls
        .iter()
        .any(|p| p.as_str().contains("os-checker")));
}

#[test]
fn test_walk() {
    // file path depends on the given path on relative or absolute aspect
    let files = walk_dir(".", 2, empty(), &[], Some);
    dbg!(files);
}

#[test]
fn test_glob() {
    use exlucded::pat;

    let pat_a = pat("a*");
    assert!(pat_a.matches("a"));
    assert!(pat_a.matches("a/b"));
    let pat_a_rec = pat("a/**");
    assert!(pat_a_rec.matches("a/b"));
}

#[test]
fn test_pat() {
    use exlucded::pat;

    let pat1 = pat("crates/a/b*");
    assert!(!pat1.matches("crates/a"));
    assert!(pat1.matches("crates/a/b"));
    assert!(pat1.matches("crates/a/bc"));
    assert!(pat1.matches("crates/a/b/c"));

    let pat2 = pat("crates/a/*");
    assert!(!pat2.matches("crates/a"));
    assert!(pat2.matches("crates/a/"));
    assert!(pat2.matches("crates/a/b"));
    assert!(pat2.matches("crates/a/b/c"));

    let pat3 = pat("*test*");
    assert!(pat3.matches("test"));
    assert!(pat3.matches("tests"));
    assert!(pat3.matches("a-tests"));
    assert!(pat3.matches("a/tests"));
    assert!(pat3.matches("a/test/b"));
}
