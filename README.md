# os checker

[<img alt="github" src="https://img.shields.io/github/license/os-checker/os-checker" height="20">](https://github.com/os-checker/os-checker)
[<img alt="github" src="https://img.shields.io/crates/v/os-checker" height="20">](https://crates.io/crates/os-checker)

【WIP】针对 Rust crates 的代码检查工具合集和应用，用以督促和提高代码库的质量。

（虽然工具名称暗示与操作系统相关，但仅仅是以它为背景而起的名字。）

计划/实施步骤：

![](https://github.com/user-attachments/assets/b0a02af6-e602-4fc2-9cdf-37c7ec01c41b)

## 二进制工具：`os-checker`

`os-checker` 生成最常见的 Rust 代码检查工具的运行结果报告。

| 工具/命令                                    | 重要性 | 说明                                                                 | 跟踪  |
|----------------------------------------------|--------|----------------------------------------------------------------------|-------|
| [`cargo fmt`][fmt]                           | 最高   | 代码格式化工具；良好的格式既赏心悦目，又防止无聊的提交改动或合并冲突 | [#4]  |
| [`cargo clippy`][clippy]                     | 最高   | 捕获常见的编码错误，并使 Rust 代码更加地道                           |       |
| [`cargo miri`][miri]                         | 最高   | UB 动态检查工具；由 rustc 核心开发者维护，所以几乎不存在误报         | [#12] |
| [`cargo semver-checks`][cargo-semver-checks] | 高     | 检查是否违反版本语义；一个严肃的发版应该遵循语义化版本控制           | [#3]  |
| [`lockbud`][lockbud]                         | 中     | 用于常见内存和并发错误的静态检查工具；[见其论文][tse]                |       |

[#3]: https://github.com/os-checker/os-checker/issues/3
[#4]: https://github.com/os-checker/os-checker/issues/4
[#12]: https://github.com/os-checker/os-checker/issues/12

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

## 其他

* [开发日志](./assets/development-logs.md)
* 目前仅支持 fmt/clippy。其他功能正在开发中。

```shell
$ os-checker --help
Usage: os-checker [--config <config>]

Run a collection of checkers targeting Rust crates, and report bad checking results and statistics.

Options:
  --config          A yaml configuration file. Refer to
                    https://github.com/os-checker/os-checker/issues/5 for the
                    defined format.

$ os-checker --config assets/repos.yaml
The result of checking os-checker-test-suite | src: ./repos/os-checker-test-suite/
os-checker-test-suite counts on kind
╭───┬───────────────────┬───────╮
│   │ kind              │ count │
├───┼───────────────────┼───────┤
│ 1 │ Unformatted(File) │ 4     │
├───┼───────────────────┼───────┤
│ 2 │ Unformatted(Line) │ 6     │
├───┼───────────────────┼───────┤
│ 3 │ Clippy(Warn)      │ 1     │
├───┼───────────────────┼───────┤
│ 4 │ Clippy(Error)     │ 1     │
╰───┴───────────────────┴───────╯
os-checker-test-suite counts on file
╭───┬─────────────────────────────┬────────┬───────╮
│   │ file                        │ inside │ count │
├───┼─────────────────────────────┼────────┼───────┤
│ 1 │ examples/need-clippy-fix.rs │ true   │ 2     │
├───┼─────────────────────────────┼────────┼───────┤
│ 2 │ examples/need-fmt.rs        │ true   │ 2     │
├───┼─────────────────────────────┼────────┼───────┤
│ 3 │ src/main.rs                 │ true   │ 5     │
├───┼─────────────────────────────┼────────┼───────┤
│ 4 │ tests/need-fmt.rs           │ true   │ 3     │
╰───┴─────────────────────────────┴────────┴───────╯

$ os-checker --config assets/repos.yaml --emit path/to/a.json
[
  {
    "key": "0",
    "data": {
      "user": "repos",
      "repo": "os-checker-test-suite",
      "package": "",
      "total_count": 6,
      "Unformatted(File)": 4,
      "Unformatted(Line)": 6,
      "Clippy(Warn)": 1,
      "Clippy(Error)": 1
    },
    "children": [
      {
        "key": "1",
        "data": {
          "user": "repos",
          "repo": "os-checker-test-suite",
          "package": "os-checker-test-suite",
          "total_count": 6,
          "Unformatted(File)": 4,
          "Clippy(Warn)": 1,
          "Clippy(Error)": 1
        },
        "children": null
      }
    ]
  }
]
```
