# os checker

【WIP】针对 Rust crates 的代码检查工具合集和应用，用以督促和提高代码库的质量。

（虽然工具名称暗示与操作系统相关，但仅仅是以它为背景而起的名字。）

计划/实施步骤：

![](https://github.com/user-attachments/assets/b0a02af6-e602-4fc2-9cdf-37c7ec01c41b)

## 二进制工具：`os-checker`

`os-checker` 生成最常见的 Rust 代码检查工具的运行结果报告。

| 工具/命令                                    | 重要性 | 说明                                                                 | 跟踪 |
|----------------------------------------------|--------|----------------------------------------------------------------------|------|
| [`cargo fmt`][fmt]                           | 最高   | 代码格式化工具；良好的格式既赏心悦目，又防止无聊的提交改动或合并冲突 | [#4] |
| [`cargo clippy`][clippy]                     | 最高   | 捕获常见的编码错误，并使 Rust 代码更加地道                           |      |
| [`cargo miri`][miri]                         | 最高   | UB 动态检查工具；由 rustc 核心开发者维护，所以几乎不存在误报         |      |
| [`cargo semver-checks`][cargo-semver-checks] | 高     | 检查是否违反版本语义；一个严肃的发版应该遵循语义化版本控制           | [#3] |
| [`lockbud`][lockbud]                         | 中     | 用于常见内存和并发错误的静态检查工具；[见其论文][tse]                |      |

[#3]: https://github.com/os-checker/os-checker/issues/3
[#4]: https://github.com/os-checker/os-checker/issues/4

注意：虽然 miri 具有最高质量的 UB 检查效果，但是并不适用于所有代码库。

据我了解，由于 miri 是动态检查的，这意味着检查的前提是需要实际的代码执行路径，也就是需要运行二进制程序，比如 
`cargo miri run` 或者 `cargo miri test`，这可能会排除某些操作系统的二进制文件，因为它们具有不寻常的、miri
不直接支持的目标平台（虽然 miri 被设计为跨平台，但可能需要一些工作才能适配成功）。

因此，我们还需要静态代码检查工具作为补充。这里暂时只选取 lockbud，它看上去更简单和实用。在安全检查工具方面，有一个完整的[清单][checker-list]，可供以后添加。

[fmt]: https://github.com/rust-lang/rustfmt
[clippy]: https://github.com/rust-lang/rust-clippy
[miri]: https://github.com/rust-lang/miri
[lockbud]: https://github.com/BurtonQin/lockbud
[tse]: https://burtonqin.github.io/publication/2020-03-11-rustdetector-tse-8
[cargo-semver-checks]: https://github.com/obi1kenobi/cargo-semver-checks
[checker-list]: https://burtonqin.github.io/posts/2024/07/rustcheckers/

## 代码贡献

关于测试的一些示例命令：

* `UPDATE_EXPECT=1 cargo t` 用于更新所有快照测试 （见 [expect-test]）
* `RUST_LOG=debug cargo t -- layout` 人工检查 layout 测试模块下的日志输出，支持更高级的环境变量条件筛选
  `RUST_LOG=target[span{field=value}]=level` （见 [tracing_subscriber::EnvFilter]）

[expect-test]: https://docs.rs/expect-test
[tracing_subscriber::EnvFilter]: https://docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/filter/struct.EnvFilter.html

## 其他

* [开发日志 1](https://docs.qq.com/slide/DTG5RWlpaU1JibmZk)
