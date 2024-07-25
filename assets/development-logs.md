# 第 1 周

[开发日志 1](https://docs.qq.com/slide/DTG5RWlpaU1JibmZk)

大纲：
* 计划/实施步骤
* 必要性
* 核心工具：os-checker CLI
* 障碍/挑战与解决方式
* 本周 coding 工作：设计和解析 yaml 配置文件

# 第 2 周

code diff: <https://github.com/os-checker/os-checker/compare/7bc4462..main>

```shell
$ git diff --stat 7bc4462 main
...
19 files changed, 3605 insertions(+), 197 deletions(-)

# 排除测试文件和其他一些文件
$ git diff --stat 7bc4462 main -- "src/*.rs" ":!*tests.rs"
 src/layout/mod.rs           |  13 +++---
 src/main.rs                 |  20 +++++++++
 src/repo/cmd.rs             |  31 ++++++++++----
 src/repo/mod.rs             | 214 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++--------------------------------
 src/run_checker/analysis.rs | 282 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 src/run_checker/mod.rs      | 308 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++----------
 6 files changed, 763 insertions(+), 105 deletions(-)
```

其他：
* 思考 [#11 使用 SARIF 格式来统一这些检查工具的输出？](https://github.com/os-checker/os-checker/discussions/11)：
  所有检查工具都有一些共性，比如问题的分类和发生的地点，而 SARIF 在这基础上深度衍生，作为一种交换格式规范，
  其目的是给自动化系统或工具使用。它不仅具有复杂性，还与 os-checker 核心功能（检查报告与统计）并无直接联系。
  最终，暂时不会把检查工具的输出统一成 SARIF 格式。
* [#13 `#[const_trait]` 在夜间版本造成 arceos 代码损坏](https://github.com/os-checker/os-checker/issues/13)：
  arceos 中长期未修复的代码损坏，并被 CI 的良性报告结果所掩埋。由于最近 arceos 正在把 crates 拆分到单独的仓库，
  如果在新仓库中依然存在该问题，那么我会去新仓库报告这个问题的解决方式。

