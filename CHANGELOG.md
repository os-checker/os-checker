# v0.3.0

os-checker CLI 支持数据库缓存每条检查命令的结果和仓库完整的检查结果。

在命令级别的检查缓存命中时，则不必执行该检查；

在仓库级别的检查缓存命中时，则不必下载仓库。

* [feat: 使用 redb 嵌入式数据库进行检查结果缓存](https://github.com/os-checker/os-checker/pull/99)
* [feat: 仓库缓存查询优化](https://github.com/os-checker/os-checker/pull/103)
