use crate::{
    config::{CheckerTool, Resolve},
    layout::Pkg,
    output::host_toolchain,
    utils::{PLUS_TOOLCHAIN_LOCKBUD, PLUS_TOOLCHAIN_MIRAI, PLUS_TOOLCHAIN_RAP},
    Result,
};
use duct::cmd;
use yash_syntax::syntax::{SimpleCommand, Unquote, Value};

/// 默认运行 cargo fmt 的命令
// NOTE: cargo fmt 不支持 --target 参数，但依然会在不同的 target_triple 上运行，
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
    let name = pkg.name;
    let expr = cmd!("cargo", &toolchain, "fmt", "-p", name, "--", "--emit=json").dir(pkg.dir);
    debug!(?expr);
    let cmd = format!("cargo {toolchain} fmt -p {name} -- --emit=json");
    Resolve::new(pkg, CheckerTool::Fmt, cmd, expr)
}

/// 默认运行 cargo clippy 的命令
pub fn cargo_clippy(pkg: &Pkg) -> Resolve {
    let target = pkg.target;
    // 只分析传入 toml path 指向的 package，不分析其依赖
    let expr = cmd!(
        "cargo",
        "clippy",
        "--target",
        target,
        "--no-deps",
        "--message-format=json"
    )
    .dir(pkg.dir);
    debug!(?expr);
    let cmd = format!("cargo clippy --target {target} --no-deps --message-format=json");
    Resolve::new(pkg, CheckerTool::Clippy, cmd, expr)
}

/// 默认运行 cargo lockbud 的命令
pub fn cargo_lockbud(pkg: &Pkg) -> Resolve {
    let target = pkg.target;

    // // 由于 cargo build 进行增量编译时，不输出旧 MIR，
    // // lockbud 无法检查。因此要么不增量编译，要么 cargo clean，要么
    // // 单独放置编译目录放置来不影响别的检查的增量编译。
    // let mut lockbud_dir = pkg.dir.to_owned();
    // lockbud_dir.extend(["__lockbud__", pkg.target]);

    let expr = cmd!(
        "cargo",
        PLUS_TOOLCHAIN_LOCKBUD,
        "lockbud",
        "-k",
        "all",
        "--",
        "--target",
        target,
        // "--target-dir",
        // &lockbud_dir
    )
    .dir(pkg.dir);
    debug!(?expr);
    let cmd = format!("cargo {PLUS_TOOLCHAIN_LOCKBUD} lockbud -k all -- --target {target}");
    Resolve::new(pkg, CheckerTool::Lockbud, cmd, expr)
}

/// 默认运行 cargo mirai 的命令
pub fn cargo_mirai(pkg: &Pkg) -> Resolve {
    let target = pkg.target;

    let expr = cmd!(
        "cargo",
        PLUS_TOOLCHAIN_MIRAI,
        "mirai",
        "--target",
        target,
        "--message-format=json"
    )
    .dir(pkg.dir);
    debug!(?expr);
    let cmd = format!("cargo {PLUS_TOOLCHAIN_MIRAI} mirai --target {target} --message-format=json");
    Resolve::new(pkg, CheckerTool::Mirai, cmd, expr)
}

/// 运行 cargo rap 检查 use after free 的命令
pub fn cargo_rap_uaf(pkg: &Pkg) -> Resolve {
    // let target = pkg.target;

    let expr = cmd!(
        "cargo",
        PLUS_TOOLCHAIN_RAP,
        "rap",
        "-F" // -F -M 尚不同时支持；也不支持指定 --target
             // "--target",
             // target,
    )
    .env("RAP_LOG", "WARN")
    .dir(pkg.dir);
    debug!(?expr);
    let cmd = format!("cargo {PLUS_TOOLCHAIN_RAP} rap -F");
    Resolve::new(pkg, CheckerTool::Rap, cmd, expr)
}

/// 运行 cargo rap 检查 memory leak 的命令
pub fn cargo_rap_memoryleak(pkg: &Pkg) -> Resolve {
    // let target = pkg.target;

    let expr = cmd!(
        "cargo",
        PLUS_TOOLCHAIN_RAP,
        "rap",
        "-M" // -F -M 尚不同时支持；也不支持指定 --target
             // "--target",
             // target,
    )
    .env("RAP_LOG", "WARN")
    .dir(pkg.dir);
    debug!(?expr);
    let cmd = format!("cargo {PLUS_TOOLCHAIN_RAP} rap -M");
    Resolve::new(pkg, CheckerTool::Rap, cmd, expr)
}

/// 自定义检查命令。
#[instrument(level = "trace")]
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
#[instrument(level = "trace")]
fn parse_cmd(line: &str) -> Result<(SimpleCommand, Vec<String>)> {
    let input: SimpleCommand = line.parse().map_err(|err| match err {
        Some(err) => {
            eyre!("解析 `{line}` 失败：\n{err}\n请输入正确的 shell 命令（暂不支持复杂的命令）")
        }
        None => eyre!("解析 `{line}` 失败，请输入正确的 shell 命令（暂不支持复杂的命令）"),
    })?;
    let words: Vec<_> = input.words.iter().map(|word| word.unquote().0).collect();
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
