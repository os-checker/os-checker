# v0.9.0

Fixes:
* continue when non-existing target is required for a checker (#402)

Chores:
* update checkers and default toolchain (#418)
* check Asterinas (#412)
* adjust configs for ArceOS (#406), axplat_crates (#410), and more (#380)
* update dependencies

# v0.8.0

Features:
* support `cargo-udeps` to detect unused dependencies (#372)
* integrate `AtomVChecker` to detect memory ordering misuse (#373)
* adjust FORCE_RUN_CHECK to accept a list of checkers (#376)

# v0.7.0

Features:

* add `--no-layout-error` layout subcommand option (#320)
* add run `--no-layout-error` and JSON input's `meta.only_pkg_dir_globs` (#328)
* support `meta.rerun` in config JSON (#330)
* run setup commands through bash before analyzing the repo (#334)
* add `convert_repo_json` CLI to extract files from lockbud output and rewrite ui/repos JSONs (#343)
* support local project checking and caching (#346)
* add `meta.use_last_cache` in JSON config and `--use-last-cache` in run subcommand (#351)
* support `meta.run_all_checkers` in JSON config (#356)

Fixes:
* `meta.only_pkg_dir_globs` filters in Cargo.toml paths; Layout::packages respects cargo_tomls (#354)

# v0.6.1

Change license from MIT to `GPL-3.0 OR MulanPubL`.

# v0.6.0

Features

* 更新工具和工具链；RAP 改名为 RAPx (#259)
* 配置文件支持指定 features (#269)

# v0.5.0

Features

* feat: cargo 诊断增加时间戳；rap 在检测到的 targets 上运行 ([#234](https://github.com/os-checker/os-checker/pull/234))
* feat: 增加配置文件指定的 target 源 ([#239](https://github.com/os-checker/os-checker/pull/239))
* feat: 支持对检查命令设置环境变量 ([#244](https://github.com/os-checker/os-checker/pull/244))

Fixes

* fix: ALL_TARGETS 通过的仓库计数 ([#236](https://github.com/os-checker/os-checker/pull/236))
* fix: layout 子命令应输出到文件而不是 stdout ([#247](https://github.com/os-checker/os-checker/pull/247))
* fix: 一旦启发式地找到确定的 target，不要默认添加 x86_64-unknown-linux-gnu ([#233](https://github.com/os-checker/os-checker/pull/233))
* fix: 在仓库和 package 中指定的 targets 应追加到安装列表，而不是覆盖安装列表 ([#238](https://github.com/os-checker/os-checker/pull/238))
* fix: 运行 rap 时指定 --target 参数 ([#231](https://github.com/os-checker/os-checker/pull/231))

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
