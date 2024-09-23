use crate::Result;
use cargo_metadata::camino::{Utf8Path, Utf8PathBuf};
use duct::cmd;
use eyre::Context;
use std::{path::Path, time::Instant};

mod scan_for_targets;
pub use scan_for_targets::scan_scripts_for_target;

/// Temp dir for os-checker, used for installing checkers.
pub const BASE_DIR_CHECKERS: &str = "/tmp/os-checker/checkers";

/// 特殊的编译目标，os-checker 目前不支持在这上面运行检查。
pub const PECULIAR_TARGETS: &[&str] = &["x86_64-fuchsia", "avr-unknown-gnu-atmega328"];

/// 检查工具固定的工具链
pub const PLUS_TOOLCHAIN_LOCKBUD: &str = "+nightly-2024-05-21";
pub const TOOLCHAIN_MIRAI: &str = "nightly-2023-12-30";
pub const PLUS_TOOLCHAIN_MIRAI: &str = "+nightly-2023-12-30";

/// git clone 一个仓库到一个 dir。
/// 如果该仓库已存在，则 git pull 拉取最新的代码。
#[instrument(level = "trace")]
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

/// 安装工具链。dir 一般指向 rust-toolchain 所在的目录。
/// 安装成功时，返回 stdout 的字节（即 rustup show 的输出。
#[instrument(level = "trace")]
pub fn install_toolchain(dir: &Utf8Path) -> Result<Vec<u8>> {
    let output = cmd!("rustup", "show").dir(dir).run()?;
    ensure!(
        output.status.success(),
        "安装工具链失败\nstderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}

pub fn rustup_target_add(targets: &[&str], dir: Option<&Utf8Path>) -> Result<()> {
    let mut command = cmd("rustup", ["target", "add"].iter().chain(targets));
    if let Some(dir) = dir {
        command = command.dir(dir);
    }
    let _ = command
        .run()
        .with_context(|| format!("在 {dir:?} 目录下安装如下 targets {targets:?} 失败"))?;
    Ok(())
}

/// 遍历一个目录及其子目录的所有文件（但不进入 .git 和 target 目录）：
/// * 需要设置一个最大递归深度（虽然可以不设置这个条件，但大部分情况下，os-checker 不需要深度递归）
/// * op_on_file 为一个回调函数，其参数保证为一个文件路径，且返回值为 Some 时表示把它的值推到 Vec
pub fn walk_dir<T>(
    dir: impl AsRef<Path>,
    max_depth: usize,
    dirs_excluded: &[&str],
    mut op_on_file: impl FnMut(Utf8PathBuf) -> Option<T>,
) -> Vec<T> {
    walkdir::WalkDir::new(dir.as_ref())
        .max_depth(max_depth) // 目录递归上限
        .into_iter()
        .filter_entry(move |entry| {
            // 别进入这些文件夹（适用于子目录递归）
            const NO_JUMP_IN: &[&str] = &[".git", "target"];
            let filename = entry.file_name();
            let excluded = &mut NO_JUMP_IN.iter().chain(dirs_excluded);
            !excluded.any(|&dir| dir == filename)
        })
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.file_type().is_file() {
                return None;
            }
            let path = Utf8PathBuf::try_from(entry.into_path()).ok()?;
            op_on_file(path)
        })
        .collect()
}

/// Perform an operation and get the execution time.
pub fn execution_time_ms<T>(op: impl FnOnce() -> T) -> (u64, T) {
    let now = Instant::now();
    let value = op();
    let duration_ms = now.elapsed().as_millis() as u64;
    (duration_ms, value)
}
