# v0.4.2

添加 `config` 子命令和 `--merged`、`--list-repos` 参数。

# v0.4.1

添加 `layout --list-targets` 参数。

# v0.4.0

集成更多检查工具： rap、mirai、rudra、audit、outdated、geiger。

# v0.3.0

os-checker CLI 支持数据库缓存每条检查命令的结果和仓库完整的检查结果。

在命令级别的检查缓存命中时，则不必执行该检查；

在仓库级别的检查缓存命中时，则不必下载仓库。

* [feat: 使用 redb 嵌入式数据库进行检查结果缓存](https://github.com/os-checker/os-checker/pull/99)
* [feat: 仓库缓存查询优化](https://github.com/os-checker/os-checker/pull/103)
