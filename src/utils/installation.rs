use super::{
    git_clone, BASE_DIR_CHECKERS, PLUS_TOOLCHAIN_HOST, PLUS_TOOLCHAIN_LOCKBUD,
    PLUS_TOOLCHAIN_MIRAI, TOOLCHAIN_LOCKBUD, TOOLCHAIN_RAP,
};
use crate::{utils::TOOLCHAIN_MIRAI, Result};
use cargo_metadata::camino::{Utf8Path, Utf8PathBuf};
use duct::{cmd, Expression};
use eyre::Context;

/// 安装工具链。dir 一般指向 rust-toolchain 所在的目录。
/// 安装成功时，返回 stdout 的字节（即 rustup show 的输出。
#[instrument(level = "info")]
pub fn install_toolchain(dir: &Utf8Path) -> Result<Vec<u8>> {
    let output = cmd!("rustup", "show")
        .dir(dir)
        .unchecked()
        .stdout_capture()
        .stderr_capture()
        .run()?;
    ensure!(
        output.status.success(),
        "安装工具链失败\nstderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}

pub fn rustup_target_add(targets: &[&str], dir: &Utf8Path) -> Result<()> {
    run_cmd(
        cmd("rustup", ["target", "add"].iter().chain(targets)).dir(dir),
        || format!("在 {dir:?} 目录下安装如下 targets {targets:?} 失败"),
    )
}

#[instrument(level = "info")]
pub fn rustup_target_add_for_checkers(targets: &[&str]) -> Result<()> {
    let err = |toolchain| format!("在 {toolchain} 工具链上安装如下 targets {targets:?} 失败");

    let mut args: Vec<_> = ["+", "target", "add"]
        .iter()
        .copied()
        .chain(targets.iter().copied())
        .collect();

    let mut install_targets = move |toolchain: &'static str| {
        args[0] = toolchain;
        run_cmd(cmd("rustup", &args), || err(toolchain))
    };

    // FIXME: use Cow for non +nightly host toolchain?
    install_targets(PLUS_TOOLCHAIN_HOST)?;

    install_targets(PLUS_TOOLCHAIN_LOCKBUD)?;
    install_targets(PLUS_TOOLCHAIN_MIRAI)?;

    Ok(())
}

/// 直接打印 stdout，但捕获 stderr。
fn run_cmd(expr: Expression, mut err: impl FnMut() -> String) -> Result<()> {
    let expr = expr.unchecked().stderr_capture();
    let output = expr.run().with_context(&mut err)?;
    ensure!(
        output.status.success(),
        "{}\nstderr={}",
        err(),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn setup_lockbud() -> Result<()> {
    let url = "https://github.com/BurtonQin/lockbud.git";
    let dir = &Utf8PathBuf::from_iter([BASE_DIR_CHECKERS, "lockbud"]);
    git_clone(dir, url)?;
    cmd!("rustup", "show").dir(dir).run()?;
    cmd!("cargo", "install", "--path", ".", "--force")
        .dir(dir)
        .run()?;
    Ok(())
}

fn setup_mirai() -> Result<()> {
    const URL: &str =
        "https://github.com/os-checker/MIRAI/releases/download/v1.1.9/mirai-installer.sh";
    cmd!("curl", "--proto", "=https", "--tlsv1.2", "-LsSf", URL)
        .pipe(cmd!("sh"))
        .run()
        .with_context(|| "安装 mirai 失败")?;
    install_checker_toolchain("mirai", TOOLCHAIN_MIRAI)?;
    Ok(())
}

fn detect_checker_if_exists(checker_bin: &str) -> bool {
    match cmd!("which", checker_bin).read() {
        Ok(location) => {
            info!(checker_bin, location);
            true
        }
        Err(err) => {
            error!(err = %err, "未找到 {checker_bin}");
            false
        }
    }
}

fn install_checker_toolchain(checker_bin: &str, toolchain: &str) -> Result<()> {
    cmd!("rustup", "toolchain", "install", toolchain).run()?;
    info!(
        checker_bin,
        "toolchain specified by the checker is installed."
    );
    Ok(())
}

/// 该函数检查是否存在 checker，如果不存在，则安装到本地。
/// 如果检查工具存在，确保安装该工具指定的工具链。
/// 该函数不安装 targets。
fn check_or_install_checkers() -> Result<()> {
    fn install(
        checker_bin: &str,
        toolchain: &str,
        setup: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        if !detect_checker_if_exists(checker_bin) {
            setup()?;
        } else {
            install_checker_toolchain(checker_bin, toolchain)?;
        }
        Ok(())
    }

    install("lockbud", TOOLCHAIN_LOCKBUD, setup_lockbud)?;
    install("mirai", TOOLCHAIN_MIRAI, setup_mirai)?;
    install_checker_toolchain("rap", TOOLCHAIN_RAP)?;

    Ok(())
}

/// This function can be called multiple times, but only perfrom
/// toolchains and checkers installation exactly only once.
pub fn init() {
    use std::sync::Once;
    static INIT_INSTALLATION: Once = Once::new();
    INIT_INSTALLATION.call_once(|| {
        crate::output::init_toolchain_info();
        check_or_install_checkers().unwrap();
    });
}

#[test]
fn which_checker() {
    crate::logger::test_init(Some("debug"), "");
    dbg!(
        detect_checker_if_exists("lockbud"),
        detect_checker_if_exists("mirai"),
        detect_checker_if_exists("mirai2"),
    );
}
