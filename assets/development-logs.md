# 第 1 周

## PPT

[开发日志 1](https://docs.qq.com/slide/DTG5RWlpaU1JibmZk)

## 大纲

* 计划/实施步骤
* 必要性
* 核心工具：os-checker CLI
* 障碍/挑战与解决方式
* 本周 coding 工作：设计和解析 yaml 配置文件

# 第 2 周

## 主要工作

1. 基于上周解析的 yaml 配置文件和代码库组织结构，这周完成了对整个目标代码库运行 fmt 和 clippy 检查
2. 对检查结果进行初步的统计，目前统计了目标代码库中，
    * 每个 package 的 fmt/clippy 不良检查结果的分类数量（具体见表格和 [快照测试 - statistics-arceos][statistics-arceos]）
    * 每个 package 的每个不良文件中，检查结果的总数量

[statistics-arceos]: https://github.com/os-checker/os-checker/blob/ae2088eccaf33ff1ccaacfa1242c2cea35b86172/src/run_checker/snapshots/statistics-arceos.txt

| Kind                | 说明                    |
|---------------------|-------------------------|
| `Unformatted(File)` | 未格式化的文件          |
| `Unformatted(Line)` | 未格式化的行            |
| `Clippy(Warn)`      | rustc/clippy 发出的警告 |
| `Clippy(Error)`     | rustc/clippy 发出的错误 |

```sql
// 示例：arceos 的 deptool 统计结果
deptool counts on kind               deptool counts on file
╭───┬───────────────────┬───────╮    ╭───┬──────────────────────────┬────────┬───────╮
│   │ kind              │ count │    │   │ file                     │ inside │ count │
├───┼───────────────────┼───────┤    ├───┼──────────────────────────┼────────┼───────┤
│ 1 │ Unformatted(File) │ 35    │    │ 1 │ src/cmd_builder.rs       │ true   │ 4     │
├───┼───────────────────┼───────┤    ├───┼──────────────────────────┼────────┼───────┤
│ 2 │ Unformatted(Line) │ 45    │    │ 2 │ src/cmd_parser.rs        │ true   │ 35    │
├───┼───────────────────┼───────┤    ├───┼──────────────────────────┼────────┼───────┤
│ 3 │ Clippy(Warn)      │ 10    │    │ 3 │ src/d2_generator.rs      │ true   │ 5     │
╰───┴───────────────────┴───────╯    ├───┼──────────────────────────┼────────┼───────┤
                                     │ 4 │ src/lib.rs               │ true   │ 39    │
                                     ├───┼──────────────────────────┼────────┼───────┤
                                     │ 5 │ src/main.rs              │ true   │ 2     │
                                     ├───┼──────────────────────────┼────────┼───────┤
                                     │ 6 │ src/mermaid_generator.rs │ true   │ 5     │
                                     ╰───┴──────────────────────────┴────────┴───────╯

// 示例：arceos 的 axnet 统计结果
axnet counts on kind               axnet counts on file (1 outer file: 100%)
╭───┬───────────────┬───────╮      ╭───┬────────────────────────────────┬────────┬───────╮
│   │ kind          │ count │      │   │ file                           │ inside │ count │
├───┼───────────────┼───────┤      ├───┼────────────────────────────────┼────────┼───────┤
│ 1 │ Clippy(Error) │ 4     │      │ 1 │ OUTER/driver_common/src/lib.rs │ false  │ 4     │
╰───┴───────────────┴───────╯      ╰───┴────────────────────────────────┴────────┴───────╯
// 经我检查，是由于其依赖项 arceos 的 driver_common 库 `#[const_trait]` 代码损坏导致的错误
```

3. 代码库来源除了支持来自 github 的 `user/repo`，还支持本地路径 `file://...` 和任何 git repo url。示例：

```yaml
# 本地路径以 file:// 开头，支持绝对路径和相对路径
file:///path/to/os-checker-test-suite:
  all: true
file://repos/arceos:
  all: true

# 任何 git repo url
https://github.com/os-checker/os-checker.git:
  all: true

# 对于 github git repo url，简化成 user/repo
os-checker/os-checker:
  all: true
```

## code diff/stat

diff view: <https://github.com/os-checker/os-checker/compare/7bc4462..main>

```shell
$ git diff --stat 7bc4462 ae2088e
...
28 files changed, 4384 insertions(+), 407 deletions(-)

# 排除测试文件和其他一些文件
$ git diff --stat 7bc4462 ae2088e -- "src/*.rs" ":!*tests.rs"
 src/layout/mod.rs           |  13 +++---
 src/logger.rs               |  27 ++++++++++++
 src/main.rs                 |  14 ++----
 src/repo/cmd.rs             |  31 +++++++++++---
 src/repo/mod.rs             | 239 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++---------------------------------
 src/repo/uri.rs             |  98 ++++++++++++++++++++++++++++++++++++++++++
 src/run_checker/analysis.rs | 282 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/run_checker/mod.rs      | 317 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++----------
 8 files changed, 900 insertions(+), 121 deletions(-)
```

在本周 50 多次提交之后，核心功能代码增加 900 行，位于以上 8 处 rs 文件；其余行数增加主要来自快照测试 (+1870 行) 和日志测试 (+1140 行)。

## 其他

* 思考 [#11 使用 SARIF 格式来统一这些检查工具的输出？](https://github.com/os-checker/os-checker/discussions/11)：
  所有检查工具都有一些共性，比如问题的分类和发生的地点，而 SARIF 在这基础上深度衍生，作为一种交换格式规范，
  其目的是给自动化系统或工具使用。它不仅具有复杂性，还与 os-checker 核心功能（检查报告与统计）并无直接联系。
  最终，暂时不会把检查工具的输出统一成 SARIF 格式。
* [#13 `#[const_trait]` 在夜间版本造成 arceos 代码损坏](https://github.com/os-checker/os-checker/issues/13)：
  arceos 中长期未修复的代码损坏，并被 CI 的良性报告结果所掩埋。由于最近 arceos 正在把 crates 拆分到单独的仓库，
  如果在新仓库中依然存在该问题，那么我会去新仓库报告这个问题的解决方式。

# 第 3 周

## 主要工作

* 7 月 27 日：发布 os-checker v0.1.0 到 crates.io
* 7 月 27-28 日：创建 [os-checker/os-checker.github.io](https://github.com/os-checker/os-checker.github.io) 仓库
  * 编写 Dockerfile：设定目标仓库运行 os-checker 输出检查结果
  * 编写 Github Action CI 脚本：自动构建和推送 Docker 镜像到 [zjpzjp/os-checker]
* 7 月 29 日 - 8 月 2 日：学习 Nuxt 和 PrimeVue 前端框架，并构建简单的 WebUI，最终静态部署到 [os-checker.github.io] 网页

[zjpzjp/os-checker]: https://hub.docker.com/repository/docker/zjpzjp/os-checker
[os-checker.github.io]: https://os-checker.github.io/


## code diff/stat

os-checker CLI 仓库代码改动情况：

```text
$ git diff --stat 440a72b 393bd05
...
19 files changed, 1539 insertions(+), 76 deletions(-)

# 核心 Rust 代码改动（排除测试文件和其他一些文件）
$ git diff --stat 440a72b 393bd05 -- "src/*.rs" ":!*tests.rs"
 src/cli/mod.rs              |  87 +++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/layout/mod.rs           |   4 ++++
 src/main.rs                 |   3 ++-
 src/repo/cmd.rs             |  11 ++++++++---
 src/repo/mod.rs             |  18 +++++++++++++++++-
 src/repo/uri.rs             |  69 ++++++++++++++++++++++++++++++++++++++++++++-------------------------
 src/run_checker/analysis.rs | 103 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++---
 src/run_checker/mod.rs      | 103 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++-------
 8 files changed, 358 insertions(+), 40 deletions(-)
```

## issues 记录

含 WebUI 仓库

- [x] [feat: respect/respond to system light/dark mode change](https://github.com/os-checker/os-checker.github.io/issues/8)
- [x] [feat: toggle light / dark theme mode](https://github.com/os-checker/os-checker.github.io/issues/7)
- [x] [feat: 多列排序](https://github.com/os-checker/os-checker.github.io/issues/6)
- [x] [feat: 当总计为 0，不要显示 0](https://github.com/os-checker/os-checker.github.io/issues/5)
- [x] [feat: 展平单仓库单项目成一行数据](https://github.com/os-checker/os-checker.github.io/issues/4)
- [x] [feat: 只对非数值列筛选](https://github.com/os-checker/os-checker.github.io/issues/3)
- [ ] [改进 JSON 数据格式输出（或者改进 CLI 与 WebUI 交互）](https://github.com/os-checker/os-checker/issues/14)
- [x] [修复静态网页 baseURL](https://github.com/os-checker/os-checker.github.io/issues/2)
