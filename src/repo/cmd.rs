use crate::Result;
use cargo_metadata::camino::Utf8Path;
use duct::{cmd, Expression};
use eyre::ContextCompat;
use yash_syntax::syntax::{SimpleCommand, Unquote, Value};

pub fn cargo_fmt(toml: &Utf8Path) -> Expression {
    // e.g. cargo fmt --check --manifest-path tmp/test-fmt/Cargo.toml
    cmd!("cargo", "fmt", "--check", "--manifest-path", toml)
}

pub fn cargo_clippy(toml: &Utf8Path) -> Expression {
    cmd!("cargo", "clippy", "--no-deps", "--manifest-path", toml)
}

pub fn custom(raw: &str, toml: &Utf8Path) -> Result<Expression> {
    let input: SimpleCommand = raw.parse().map_err(|err| match err {
        Some(err) => {
            eyre!("解析 `{raw}` 失败：\n{err}\n请输入正确的 shell 命令（暂不支持复杂的命令）")
        }
        None => eyre!("解析 `{raw}` 失败，请输入正确的 shell 命令（暂不支持复杂的命令）"),
    })?;

    let mut words: Vec<_> = input.words.iter().map(|word| word.unquote().0).collect();
    ensure!(!words.is_empty(), "请输入检查工具的执行文件名称或路径");

    // 构造命令
    let exe = words.remove(0);
    let mut expr = cmd(exe, words);

    // 设置环境变量
    println!("assigns.len={}", input.assigns.len());
    for assgin in &input.assigns {
        let name = &*assgin.name;
        let val = match &assgin.value {
            Value::Scalar(word) => word.unquote().0,
            Value::Array(_) => bail!("对于 `{raw}`，不支持设置 Array assgin 环境变量"),
        };
        println!("[env] {name}={val}");
        expr = expr.env(name, val);
    }

    // 设置工作目录
    let working_dir = toml
        .parent()
        .with_context(|| format!("无法获取 Cargo.toml 路径 `{toml}` 的父目录"))?;
    expr = expr.dir(working_dir);

    // 暂不处理重定向

    Ok(expr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn custom_cmd() {
        let toml: &Utf8Path = "./Cargo.toml".into();
        expect![[r#"
            Io(
                Dir(
                    ".",
                ),
                Cmd(
                    [
                        "cargo",
                        "fmt",
                        "--check",
                    ],
                ),
            )
        "#]]
        .assert_debug_eq(&custom("cargo fmt --check", toml).unwrap());

        expect![[r#"
            Io(
                Dir(
                    ".",
                ),
                Io(
                    Env(
                        "RUST_LOG",
                        "debug",
                    ),
                    Io(
                        Env(
                            "RUSTFLAGS",
                            "--cfg unstable",
                        ),
                        Cmd(
                            [
                                "cargo",
                                "clippy",
                                "-F",
                                "a,b,c",
                                "-F",
                                "e,f",
                            ],
                        ),
                    ),
                ),
            )
        "#]]
        .assert_debug_eq(
            // 这里指定 -F 的方式可能是错误的，但目的是测试环境变量和引号处理
            &custom(
                r#"RUSTFLAGS="--cfg unstable" RUST_LOG=debug cargo clippy -F a,b,c -F "e,f" "#,
                toml,
            )
            .unwrap(),
        );
    }
}
