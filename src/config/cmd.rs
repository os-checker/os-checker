use crate::{
    config::{CheckerTool, Resolve},
    layout::Pkg,
    output::host_toolchain,
    utils::{
        PLUS_TOOLCHAIN_ATOMVCHECKER, PLUS_TOOLCHAIN_LOCKBUD, PLUS_TOOLCHAIN_MIRAI,
        PLUS_TOOLCHAIN_RAP, PLUS_TOOLCHAIN_RUDRA,
    },
    Result,
};
use duct::{cmd, Expression};
use indexmap::IndexMap;
use yash_syntax::syntax::{SimpleCommand, Unquote, Value};

fn add_env(mut expr: Expression, env: &IndexMap<String, String>) -> (Expression, String) {
    use std::fmt::Write;
    let mut env_str = String::new();
    for (name, val) in env {
        expr = expr.env(name, val);
        _ = write!(env_str, "{name}={val:?} ");
    }
    (expr, env_str)
}

/// 默认运行 cargo fmt 的命令
// NOTE: cargo fmt 不支持 --target 和 -F 参数，但依然会在不同的 target_triple 上运行，
// 尽管这会造成报告重复。
//
// $ cargo fmt --target x86_64-unknown-linux-gnu
// error: unexpected argument '--target' found
//
//   tip: to pass '--target' as a value, use '-- --target'
//
// Usage: cargo fmt [OPTIONS] [-- <rustfmt_options>...]
// For more information, try '--help'.
pub fn cargo_fmt(pkg: &Pkg) -> Resolve {
    let toolchain = host_toolchain();
    let expr = cmd!("cargo", &toolchain, "fmt", "--", "--emit=json").dir(pkg.dir);
    let (expr, env_str) = add_env(expr, &pkg.env);
    debug!(?expr);
    let cmd = format!("{env_str}cargo {toolchain} fmt");
    Resolve::new(pkg, CheckerTool::Fmt, cmd, expr)
}

/// 默认运行 cargo clippy 的命令
pub fn cargo_clippy(pkg: &Pkg) -> Resolve {
    // 只分析传入 toml path 指向的 package，不分析其依赖
    let mut args = vec![
        "clippy",
        "--target",
        pkg.target,
        "--no-deps",
        "--message-format=json",
    ];
    args.extend(pkg.features_args.iter().map(|s| &**s));
    let expr = cmd("cargo", args).dir(pkg.dir);
    let (expr, env_str) = add_env(expr, &pkg.env);
    debug!(?expr);
    let cmd = format!(
        "{env_str}cargo clippy --target {} {} --no-deps",
        pkg.target,
        pkg.features_args.join(" ")
    );
    Resolve::new(pkg, CheckerTool::Clippy, cmd, expr)
}

/// 默认运行 cargo lockbud 的命令
pub fn cargo_lockbud(pkg: &Pkg) -> Resolve {
    // // 由于 cargo build 进行增量编译时，不输出旧 MIR，
    // // lockbud 无法检查。因此要么不增量编译，要么 cargo clean，要么
    // // 单独放置编译目录放置来不影响别的检查的增量编译。
    // let mut lockbud_dir = pkg.dir.to_owned();
    // lockbud_dir.extend(["__lockbud__", pkg.target]);

    let mut args = vec![
        PLUS_TOOLCHAIN_LOCKBUD,
        "lockbud",
        "-k",
        "all",
        "--",
        "--target",
        pkg.target,
    ];
    args.extend(pkg.features_args.iter().map(|s| &**s));
    let expr = cmd("cargo", args).dir(pkg.dir);
    let (expr, env_str) = add_env(expr, &pkg.env);
    debug!(?expr);
    // NOTE: add -b -l to the cmd string?; but -b -l seems not working as expected:
    // these crates are still reported with the option.
    let cmd = format!(
        "{env_str}cargo {PLUS_TOOLCHAIN_LOCKBUD} lockbud -k all -- --target {} {}",
        pkg.target,
        pkg.features_args.join(" ")
    );
    Resolve::new(pkg, CheckerTool::Lockbud, cmd, expr)
}

pub fn cargo_atomvchecker(pkg: &Pkg) -> Resolve {
    let mut args = vec![
        PLUS_TOOLCHAIN_ATOMVCHECKER,
        "atomvchecker",
        "-k",
        "atomicity_violation",
        "--",
        "--target",
        pkg.target,
    ];
    args.extend(pkg.features_args.iter().map(|s| &**s));
    let expr = cmd("cargo", args).dir(pkg.dir);
    let (expr, env_str) = add_env(expr, &pkg.env);
    debug!(?expr);
    let cmd = format!(
        "{env_str}cargo {PLUS_TOOLCHAIN_ATOMVCHECKER} atomvchecker -k atomicity_violation -- --target {} {}",
        pkg.target,
        pkg.features_args.join(" ")
    );
    Resolve::new(pkg, CheckerTool::Atomvchecker, cmd, expr)
}

/// 默认运行 cargo mirai 的命令
pub fn cargo_mirai(pkg: &Pkg) -> Resolve {
    let mut args = vec![
        PLUS_TOOLCHAIN_MIRAI,
        "mirai",
        "--target",
        pkg.target,
        "--message-format=json",
    ];
    args.extend(pkg.features_args.iter().map(|s| &**s));
    let expr = cmd("cargo", args).dir(pkg.dir);
    let (expr, env_str) = add_env(expr, &pkg.env);
    debug!(?expr);
    let cmd = format!(
        "{env_str}cargo {PLUS_TOOLCHAIN_MIRAI} mirai --target {} {}",
        pkg.target,
        pkg.features_args.join(" ")
    );
    Resolve::new(pkg, CheckerTool::Mirai, cmd, expr)
}

pub fn cargo_rap(pkg: &Pkg) -> Resolve {
    let mut args = vec![
        PLUS_TOOLCHAIN_RAP,
        "rapx",
        "-F",
        "-M",
        "-timeout=300",
        "--",
        "--target",
        pkg.target,
        "--color=never",
    ];
    args.extend(pkg.features_args.iter().map(|s| &**s));
    let expr = cmd("cargo", args).env("RAP_LOG", "WARN").dir(pkg.dir);
    let (expr, env_str) = add_env(expr, &pkg.env);
    debug!(?expr);
    let cmd = format!(
        "{env_str}cargo {PLUS_TOOLCHAIN_RAP} rapx -F -M -- --target {} {}",
        pkg.target,
        pkg.features_args.join(" ")
    );
    Resolve::new(pkg, CheckerTool::Rapx, cmd, expr)
}

// FIXME: check how cargo check arguments are supported by rudra
pub fn cargo_rudra(pkg: &Pkg) -> Resolve {
    let mut args = vec![PLUS_TOOLCHAIN_RUDRA, "rudra", "--target", pkg.target];
    args.extend(pkg.features_args.iter().map(|s| &**s));
    let expr = cmd("cargo", args).dir(pkg.dir);
    let (expr, env_str) = add_env(expr, &pkg.env);
    debug!(?expr);
    let cmd = format!(
        "{env_str}cargo {PLUS_TOOLCHAIN_RUDRA} rudra --target {} {}",
        pkg.target,
        pkg.features_args.join(" ")
    );
    Resolve::new(pkg, CheckerTool::Rudra, cmd, expr)
}

pub fn cargo_geiger(pkg: &Pkg) -> Resolve {
    let toolchain = host_toolchain();
    let expr = cmd!(
        "cargo",
        &toolchain,
        "geiger",
        "--output-format",
        "Ascii",
        "--color",
        "never",
    )
    .dir(pkg.dir);
    let (expr, env_str) = add_env(expr, &pkg.env);
    debug!(?expr);
    let cmd = format!("{env_str}cargo {toolchain} geiger --output-format Ascii");
    Resolve::new(pkg, CheckerTool::Geiger, cmd, expr)
}

pub fn cargo_outdated(pkg: &Pkg) -> Resolve {
    let toolchain = host_toolchain();
    let expr = cmd!(
        "cargo",
        &toolchain,
        "outdated",
        "-R",
        "--exit-code=2",
        "--color=never"
    )
    .dir(pkg.dir);
    let (expr, env_str) = add_env(expr, &pkg.env);
    debug!(?expr);
    let cmd = format!("{env_str}cargo {toolchain} outdated -R --exit-code=2");
    Resolve::new(pkg, CheckerTool::Outdated, cmd, expr)
}

pub fn cargo_semver_checks(pkg: &Pkg) -> Resolve {
    let toolchain = host_toolchain();
    let mut args = vec![
        &toolchain,
        "semver-checks",
        "--target",
        pkg.target,
        "--color=never",
    ];
    args.extend(pkg.features_args.iter().map(|s| &**s));
    let expr = cmd("cargo", args).dir(pkg.dir);
    let (expr, env_str) = add_env(expr, &pkg.env);
    debug!(?expr);
    let cmd = format!(
        "{env_str}cargo {toolchain} semver-checks --target {} {}",
        pkg.target,
        pkg.features_args.join(" ")
    );
    Resolve::new(pkg, CheckerTool::SemverChecks, cmd, expr)
}

pub fn cargo_udeps(pkg: &Pkg) -> Resolve {
    let toolchain = host_toolchain();
    let mut args = vec![&toolchain, "udeps", "--color=never", "--target", pkg.target];
    args.extend(pkg.features_args.iter().map(|s| &**s));

    let expr = cmd("cargo", args).dir(pkg.dir);

    let (expr, env_str) = add_env(expr, &pkg.env);
    debug!(?expr);
    let cmd = format!(
        "{env_str}cargo {toolchain} udeps --target {} {}",
        pkg.target,
        pkg.features_args.join(" ")
    );
    Resolve::new(pkg, CheckerTool::Udeps, cmd, expr)
}
/// 自定义检查命令。
pub fn custom(line: &str, pkg: &Pkg, checker: CheckerTool) -> Result<Resolve> {
    let (input, mut words) = parse_cmd(line)?;
    ensure!(
        words.len() > 2,
        "请输入检查工具的执行文件名称或路径：命令切分的字长必须大于 2"
    );

    let overriden = set_toolchain_and_target(&mut words, pkg.target, None);
    let cmd_str = words.join(" ");

    // 构造命令、设置工作目录
    let exe = words.remove(0);
    let mut expr = cmd(exe, words).dir(pkg.dir);

    // 设置环境变量
    debug!(assigns.len = input.assigns.len());
    for assgin in &input.assigns {
        let name = &*assgin.name;
        let val = match &assgin.value {
            Value::Scalar(word) => word.unquote().0,
            Value::Array(_) => bail!("对于 `{line}`，不支持设置 Array assgin 环境变量"),
        };
        debug!("[env] {name}={val}");
        expr = expr.env(name, val);
    }

    // 暂不处理重定向

    debug!(?expr);

    let resolve = if let Some(target) = overriden {
        Resolve::new_overrriden(pkg, target, checker, cmd_str, expr)
    } else {
        Resolve::new(pkg, checker, cmd_str, expr)
    };
    Ok(resolve)
}

/// TODO: os-checker 已经检查每行检查命令必须包含对应的工具名，但这并不意味着
/// 每行检查命令只有一个 shell command。我们可以支持 `{ prerequisite1; prerequisite2; ...; tool cmd; }`
/// 其中 prerequisite 不包含 tool name。暂时尚未编写一行检查命令中支持多条语句的代码，如需支持，则把
/// SimpleCommand 换成 Command。
fn parse_cmd(line: &str) -> Result<(SimpleCommand, Vec<String>)> {
    let input: SimpleCommand = line.parse().map_err(|err| match err {
        Some(err) => {
            eyre!("解析 `{line}` 失败：\n{err}\n请输入正确的 shell 命令（暂不支持复杂的命令）")
        }
        None => eyre!("解析 `{line}` 失败，请输入正确的 shell 命令（暂不支持复杂的命令）"),
    })?;
    let words: Vec<_> = input.words.iter().map(|word| word.0.unquote().0).collect();
    Ok((input, words))
}

/// 从自定义命令中提取 --target
fn extract_target(words: &[String]) -> Option<&str> {
    words.iter().enumerate().find_map(|(idx, word)| {
        if word == "--target" {
            words.get(idx + 1).map(|w| &**w)
        } else {
            word.strip_prefix("--target=")
        }
    })
}

/// 添加可能的工具链或者 target。
/// 如果已经命令已经包含 target，则不设置；否则设置 target。
/// 如果 set_toolchain 为 Some，则设置，否则不设置工具链。
fn set_toolchain_and_target(
    words: &mut Vec<String>,
    candidate_target: &str,
    set_toolchain: Option<&str>, // FIXME: 考虑 lockbud 设置的工具链
) -> Option<String> {
    let overriden = extract_target(words).map(String::from);
    // `cargo +toolchain xxx (--target=...) rest...`
    // or `cargo xxx (--target=...) rest...`
    match words.as_mut_slice() {
        [cargo, toolchain, _, ..] if cargo == "cargo" && toolchain.starts_with("+") => {
            if let Some(set) = set_toolchain {
                toolchain.clear();
                toolchain.push('+');
                toolchain.push_str(set);
            }
            if overriden.is_none() {
                words.insert(3, format!("--target={candidate_target}"));
            }
        }
        [cargo, _, ..] if cargo == "cargo" => {
            let pos = if let Some(set) = set_toolchain {
                words.insert(1, format!("+{set}"));
                3
            } else {
                2
            };
            if overriden.is_none() {
                words.insert(pos, format!("--target={candidate_target}"));
            }
        }
        _ => panic!("Only `cargo +toolchain subcmd` or `cargo subcmd` is supported for now"),
    }
    overriden
}

#[cfg(test)]
mod tests;
