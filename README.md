# os checker

[<img alt="github" src="https://img.shields.io/github/license/os-checker/os-checker" height="20">](https://github.com/os-checker/os-checker)
[<img alt="github" src="https://img.shields.io/crates/v/os-checker" height="20">](https://crates.io/crates/os-checker)

对 Rust 编写的代码运行一系列检查工具，并对结果进行报告和统计，用以督促和提高代码库的质量。

虽然工具名称暗示与操作系统相关，但仅仅是以它为背景而起的名字。也就是说， os-checker 适用于任何 Rust 代码库。

详细文档见： [os-checker book](https://os-checker.github.io/book/checkers.html) | [PPT](https://docs.qq.com/slide/DTEdZdFhMSFR5QVBZ)。

os-checker 由以下部分组成：

| 工具                    | 仓库                          |                                                 version                                                | 功能                                              |
|-------------------------|-------------------------------|:------------------------------------------------------------------------------------------------------:|---------------------------------------------------|
| os-checker CLI          | [os-checker]                  |          [<img alt="github" src="https://img.shields.io/crates/v/os-checker" height="20">][1]          | 对目标仓库运行一系列检查工具，最终输出检查结果    |
| os-checker-types Lib    | [os-checker]                  |      [<img alt="github" src="https://img.shields.io/crates/v/os-checker-types" height="20">][1.1]      | os-checker 公开可序列化和反序列的类型库           |
| os-checker-database CLI | [os-checker]                  |     [<img alt="github" src="https://img.shields.io/crates/v/os-checker-database" height="20">][1.2]    | 操作数据库文件并生成 WebUI 所需的基于目录的 JSONs |
| plugin Lib              | [plugin]                      |       [<img alt="github" src="https://img.shields.io/crates/v/os-checker-plugin" height="20">][0]      | 作为其他 plugin CLIs 的功能共享库                 |
| plugin-docs CLI         | [plugin-docs]                 |    [<img alt="github" src="https://img.shields.io/crates/v/os-checker-plugin-docs" height="20">][2]    | 构建基于仓库最新提交的所有库的 rustdoc 文档       |
| plugin-cargo CLI        | [plugin-cargo]                |    [<img alt="github" src="https://img.shields.io/crates/v/os-checker-plugin-cargo" height="20">][3]   | 解析仓库的 cargo 和 git 信息，比如每个包的信息    |
| plugin-github-api CLI   | [plugin-github-api]           | [<img alt="github" src="https://img.shields.io/crates/v/os-checker-plugin-github-api" height="20">][4] | 通过 API 获取仓库在 Github 上的基本信息           |
| WebUI                   | [os-checker.github.io][WebUI] |                                                                                                        | 通过网页应用呈现检查结果，并部署到 Github Pages   |
| database                | [database]                    |                                                                                                        | 存储检查结果和基础信息数据                        |
| Github Action Workflow  | [os-checker-action]           |                                                                                                        | 在 Github 仓库 CI 中自动化部署上述工具            |
| Docker 容器             | [zjpzjp/os-checker]           |                                                                                                        | 基于 Docker 容器自动化部署上述工具                |
| 文档                    | [book]                        |                                                                                                        | 介绍 os-checker                                   |



[os-checker]: https://github.com/os-checker/os-checker
[1]: https://crates.io/crates/os-checker
[1.1]: https://crates.io/crates/os-checker-types
[1.2]: https://crates.io/crates/os-checker-database
[plugin]: https://github.com/os-checker/plugin
[0]: https://crates.io/crates/os-checker-plugin
[plugin-docs]: https://github.com/os-checker/docs
[2]: https://crates.io/crates/os-checker-plugin-docs
[plugin-cargo]: https://github.com/os-checker/plugin-cargo
[3]: https://crates.io/crates/os-checker-plugin-cargo
[plugin-github-api]: https://github.com/os-checker/plugin-github-api
[4]: https://crates.io/crates/os-checker-plugin-github-api

[os-checker-action]: https://github.com/os-checker/os-checker-action
[zjpzjp/os-checker]: https://hub.docker.com/repository/docker/zjpzjp/os-checker

[WebUI]: https://github.com/os-checker/os-checker.github.io
[os-checker.github.io]: https://os-checker.github.io
[database]: https://github.com/os-checker/database
[book]: https://github.com/os-checker/book

os-checker 目前设计为检查 Github 上的仓库代码，并且采用 Github Action 进行自动化检查。

已推出自己的 Github Action Workflow 和 Docker 镜像，来对上述工具进行自动化部署。

## 二进制工具：`os-checker`

`os-checker` 生成最常见的 Rust 代码检查工具的运行结果报告。

| checker 类别 |    子类别    | 工具       | 重要程度 | 亮点/论文             | issue  | 说明                                    |
|:------------:|:------------:|------------|----------|-----------------------|--------|-----------------------------------------|
| 程序分析工具 |              |            |          |                       |        |                                         |
|      👉      | 静态检查工具 |            |          |                       |        |                                         |
|              |              | [clippy]   | ⭐⭐⭐   | 社区实践标准          |        | 捕获常见的编码错误，并使代码更加地道    |
|              |              | [mirai]    | ⭐⭐     | [论文][mirai-paper]   | [#36]  | 具有非标注和标注两种检查方式            |
|              |              | [lockbud]  | ⭐⭐     | [论文][lockbud-paper] | [#34]  | 检查常见内存和并发错误                  |
|              |              | [rap]      | ⭐⭐     | [RAP book][rap-book]  | [#138] | 检查 UAF 和内存泄露                     |
|              |              | [rudra]    | ⭐⭐     | [论文][rudra-paper]   | [#161] | 检查 panic safety 和 Send/Sync Variance |
|      👉      | 动态检查工具 |            |          |                       |        |                                         |
|              |              | 测试       | ⭐⭐⭐   | 工程实践标准          |        | `cargo test` 或者自定义测试?            |
|              |              | [miri]     | ⭐⭐⭐   | 社区实践标准          | [#12]  | 最高质量的 UB 检查结果                  |
| 辅助检查工具 |              |            |          |                       |        |                                         |
|      👉      |  格式化检查  | [fmt]      | ⭐⭐⭐   | 社区实践标准          | [#4]   | 检查未格式化的代码                      |
|      👉      |  供应链审查  |            |          |                       |        |                                         |
|              |              | [audit]    | ⭐⭐⭐   | 社区实践标准          | [#42]  | 检查是否存在已报告安全漏洞的依赖版本    |
|              |              | [outdated] | ⭐       |                       | [#131] | 尽可能使用最新的依赖                    |
|      👉      |   代码统计   | [geiger]   | ⭐       |                       | [#154] | 尽可能警惕不安全代码                    |
|      👉      | 版本语义检查 | [semver]   | ⭐⭐     | 社区实践标准          |        | 一个严肃的发版应该遵循语义化版本控制    |

[fmt]: https://github.com/rust-lang/rustfmt
[#4]: https://github.com/os-checker/os-checker/issues/4

[audit]: https://github.com/RustSec/rustsec/tree/main/cargo-audit
[#42]: https://github.com/os-checker/os-checker/issues/42

[outdated]: https://github.com/kbknapp/cargo-outdated
[#131]: https://github.com/os-checker/os-checker/issues/131

[geiger]: https://github.com/geiger-rs/cargo-geiger
[#154]: https://github.com/os-checker/os-checker/issues/154

[clippy]: https://github.com/rust-lang/rust-clippy

[mirai]: https://github.com/endorlabs/MIRAI
[mirai-paper]: https://alastairreid.github.io/papers/hatra2020.pdf
[#36]: https://github.com/os-checker/os-checker/issues/36

[lockbud]: https://github.com/BurtonQin/lockbud
[lockbud-paper]: https://burtonqin.github.io/publication/2020-03-11-rustdetector-tse-8
[#34]: https://github.com/os-checker/os-checker/issues/34

[rap]: https://github.com/Artisan-Lab/RAP
[rap-book]: https://artisan-lab.github.io/RAP-Book
[#138]: https://github.com/os-checker/os-checker/issues/138

[rudra]: https://github.com/sslab-gatech/Rudra
[rudra-paper]: https://github.com/sslab-gatech/Rudra/blob/master/rudra-sosp21.pdf
[#161]: https://github.com/os-checker/os-checker/issues/161

[miri]: https://github.com/rust-lang/miri
[#12]: https://github.com/os-checker/os-checker/issues/12

[semver]: https://github.com/obi1kenobi/cargo-semver-checks
[checker-list]: https://burtonqin.github.io/posts/2024/07/rustcheckers/

此外，os-checker 还应包括基础信息：
* Cargo.toml：Package 维度；由许多工具读取和使用，应该正确维护
* Github API：仓库维度



