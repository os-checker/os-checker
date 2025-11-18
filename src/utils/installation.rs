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

pub fn rustup_target_add_for_checkers(targets: &[&str]) -> Result<()> {
    let install_targets = |toolchain: &'static str, target: &str| {
        let expr = cmd("rustup", [toolchain, "target", "add", target]);
        run_cmd(expr, || {
            format!("在 {toolchain} 工具链上安装 target {target:?} 失败")
        })
    };

    let toolchains = [
        ("host", super::PLUS_TOOLCHAIN_HOST),
        ("Lockbud", super::PLUS_TOOLCHAIN_LOCKBUD),
        ("Atomvchecker", super::PLUS_TOOLCHAIN_ATOMVCHECKER),
        ("Mirai", super::PLUS_TOOLCHAIN_MIRAI),
        ("RAPx", super::PLUS_TOOLCHAIN_RAP),
    ];

    for (checker, toolchain) in toolchains {
        for target in targets {
            // 针对 checker 安装编译目标，可能工具链没有配备编译目标
            if let Err(err) = install_targets(toolchain, target) {
                error!(checker, ?err);
            }
        }
    }

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
    detect_checker_if_exists("rapx")?;
    detect_checker_if_exists("lockbud")?;
    detect_checker_if_exists("atomvchecker")?;
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
