# os-checker 工具集

[<img alt="github" src="https://img.shields.io/github/license/os-checker/os-checker" height="20">](https://github.com/os-checker/os-checker)
[<img alt="github" src="https://img.shields.io/crates/v/os-checker" height="20">](https://crates.io/crates/os-checker)

对 Rust 编写的代码运行一系列检查工具，并对结果进行报告和统计，用以督促和提高代码库的质量。

虽然工具名称暗示与操作系统相关，但仅仅是以它为背景而起的名字。也就是说， os-checker 适用于任何 Rust 代码库。

详细文档见： [os-checker book](https://os-checker.github.io/book/checkers.html) | [工作原理](https://os-checker.github.io/book/under-the-hood.html)。

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

上述仓库统一作为子模块放到 [os-checker-repertoire] 仓库。

[os-checker-repertoire]: https://github.com/os-checker/os-checker-repertoire

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

已支持 [Github Action Workflow][os-checker-action] 和 [Docker 镜像][zjpzjp/os-checker]，来对上述工具进行自动化部署。

已集成 [以下检查工具](https://os-checker.github.io/book/checkers.html)：

[![checkers](https://github.com/user-attachments/assets/2c488c58-ff69-42e5-aa20-0b8e174f416f)](https://os-checker.github.io/book/checkers.html)

此外，os-checker 生成包括基础信息：
* Cargo.toml：Package 维度；由许多工具读取和使用，应该正确维护
* Github API：仓库维度

# Licenses

This project is under dual licensing `GPL-3.0 OR MulanPubL`.

