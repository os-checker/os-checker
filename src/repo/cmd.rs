use crate::{
    layout::Pkg,
    repo::{CheckerTool, Resolve},
    Result,
};
use duct::cmd;
use yash_syntax::syntax::{SimpleCommand, Unquote, Value};

/// 默认运行 cargo fmt 的命令
pub fn cargo_fmt(pkg: &Pkg) -> Resolve {
    let target = pkg.target;
    let expr = cmd!("cargo", "fmt", "--target", target, "--", "--emit=json").dir(pkg.dir);
    debug!(?expr);
    let cmd = format!("cargo fmt --target {target} -- --emit=json");
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

/// 自定义检查命令。
pub fn custom(line: &str, pkg: &Pkg, checker: CheckerTool) -> Result<Resolve> {
    let (input, mut words) = parse_cmd(line)?;
    ensure!(
        words.len() > 2,
        "请输入检查工具的执行文件名称或路径：命令切分的字长必须大于 2"
    );

    let overriden = append_target(&mut words, pkg.target);
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

fn append_target(words: &mut Vec<String>, candidate_target: &str) -> Option<String> {
    let overriden = extract_target(words).map(String::from);
    if overriden.is_none() {
        words.insert(2, format!("--target={candidate_target}"));
    }
    overriden
}

#[cfg(test)]
mod tests;
