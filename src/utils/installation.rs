use crate::Result;
use cargo_metadata::camino::Utf8Path;
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
    let expr = cmd("rustup", ["target", "add"].iter().chain(targets)).dir(dir);
    // info!(?expr, ?targets, %dir);
    run_cmd(expr, || {
        format!("在 {dir:?} 目录下安装如下 targets {targets:?} 失败")
    })
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
        let expr = cmd("rustup", &args);
        run_cmd(expr, || err(toolchain))
    };

    // FIXME: use Cow for non +nightly host toolchain?
    install_targets(super::PLUS_TOOLCHAIN_HOST)?;

    install_targets(super::PLUS_TOOLCHAIN_LOCKBUD)?;
    install_targets(super::PLUS_TOOLCHAIN_MIRAI)?;
    install_targets(super::PLUS_TOOLCHAIN_RAP)?;

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

fn detect_checker_if_exists(checker_bin: &str) -> Result<()> {
    match cmd!("which", checker_bin).read() {
        Ok(location) => {
            info!(checker_bin, location);
            Ok(())
        }
        Err(err) => {
            // error!(err = %err, "未找到 {checker_bin}");
            Err(err).context(format!("未找到 {checker_bin}"))
        }
    }
}

fn detect_checkers() -> Result<()> {
    detect_checker_if_exists("rap")?;
    detect_checker_if_exists("lockbud")?;
    detect_checker_if_exists("mirai")?;
    detect_checker_if_exists("rudra")?;
    Ok(())
}

/// This function can be called multiple times, but only perfrom
/// toolchains and checkers installation exactly only once.
pub fn init() {
    use std::sync::Once;
    static INIT_INSTALLATION: Once = Once::new();
    INIT_INSTALLATION.call_once(|| {
        crate::output::init_toolchain_info();
        detect_checkers().unwrap();
    });
}

#[test]
fn which_checker() {
    crate::logger::test_init(Some("debug"), "");
    detect_checker_if_exists("lockbud").unwrap();
    detect_checker_if_exists("mirai").unwrap();
    detect_checker_if_exists("mirai2").unwrap();
}
