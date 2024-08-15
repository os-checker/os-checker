# os-checker JSON 输出格式（设计稿）

为了把信息统一组织起来，也为了方便处理输出的数据，需要明确 JSON 数据应该包含的内容，最后附上最简示例。

从逻辑上，应该把完整的 os-checker 检查划分为几个维度：

* 检查环境：运行一次 os-checker CLI 的环境，包括
    * 各种工具信息：Rust 工具链信息（版本号等）、所应用检查工具信息（名称、版本号等)、os-checker 信息（运行和结束的时间）
    * 宿主机器信息：架构、操作系统
    * 检查对象信息：
        * 仓库信息：
            * user 
            * repo 
            * Rust 项目结构信息：由 [cargo metadata] 提供，重要的有 workspace 布局（即 cargoLayout，包含各个 package 的路径）
            * 其他基础信息：分支名、最后一次的提交 sha、时间等，可通过 Github API 获得，见末尾的 Misc
        * package 信息：每个检查工具作用的直接对象
            * 仓库信息索引
            * user：虽然通过仓库信息索引可以知道，但这个数据很常用，复制过来以减轻数据分析的复杂度
            * repo：理由同 user
            * branch：理由同 user；注意，同一个仓库下，不同 branch 的 package 应该视为不同的 packge（目前只考虑默认分支，因此一般情况下 package 就是指 `user repo package`）
            * pkg：即 Cargo.toml 中的 `package.name`
            * dir：即 cargo_toml_path 去除 Cargo.toml 的父目录；这可能也很常用，从 cargoLayout 复制过来
            * 定义的 cargo targets[^1]：需要确认，如果同时存在 main.rs 和 lib.rs，检查哪个或者
            * 定义的 features：不仅是基础信息，还用于校验检查命令
* 检查过程：每条检查结果必须通过数字索引对应一条检查过程；将过程和结果分开放置，是出于数据压缩考虑，把它们放到一起会很冗余
    * package 索引：指向检查对象
    * 检查工具名
    * 检查命令：来自 os-checker 默认提供或者 os-checker 利用某种方式分析生成或者使用者指定
    * 编译条件：
        * 架构名和架构目标三元组：即 `<arch>` 和 `<arch><sub>-<vendor>-<sys>-<abi>`
        * 指定的 features：即 `--features ...`
        * 其他 rustc 编译选项：即 `RUSTFLAGS='...'`，上面的编译条件都可以视为 rustc 编译选项，它们直接控制编译哪些源代码（也就是直接影响检查哪些代码），但这里放置除上面之外的选项，比如用于条件编译的 `--cfg` 或者众多不稳定的 `-Z` 编译选项
* 检查结果：问题文件信息；实际上，问题发生的地点应该由 `(文件名, 行, 列)` 描述，但出于简化，只到文件级别
    * idx 索引：指向一个检查过程，检查过程会包含检查对象（user/repo#package）
    * 问题文件路径：需统一处理所有工具报告的文件路径；有些工具报告绝对路径，有些报告相对路径，os-checker 尽量统一为相对路径；注意，如果问题来自该 package 之外，那么此时文件指向依赖项的绝对路径
    * 原始检查输出
    * 诊断类别：比如 clippy 这个工具可以发出 `Clippy(Warn | Error)` 两个类别、lockbud 可以发出围绕 deadlock/memory/panic 的一些详细检查类别；我认为分类展示检查结果，非常有必要

[cargo metadata]: https://doc.rust-lang.org/cargo/commands/cargo-metadata.html#json-format

[^1]: 注意：os-checker 只对 lib 和 bin 进行分析，虽然可以支持对 tests 之类的 targets 分析，但当前我不建议做那么全。

```json
{
  "env": {
    "tools": [
      {"rust": {"version": "1.82.0-nightly (91376f416 2024-08-12)"}},
      {"clippy": {"version": "clippy 0.1.82 (91376f4 2024-08-12)"}},
      {"lockbud": {"version": "sha...", "date": "...", "rustToolchain": "..."}}, // lockbud 需要固定工具链
      {"os-checker": {"start": "...", "finish": "..."}}
    ],
    "host": { "arch": "x86_64", "kernel": "..." }, // arch 命令和 cat /proc/version
    "repos": [
      {"user": "arceos-org", "repo": "arceos", "cargoLayout": [...], "info": {...}}
    ],
    "packages": [ // repo 指向 repos 数组中的一项
      {"repo": 0, "user": "arceos-org", "repo": "arceos", "branch": "main", "pkg": "axstd", "dir": "/absolute/path/to/package", "cargo_targets": [...], "features": [...]}
    ]
  },
  "idx": [ // package 指向 packages 数组中的一项
    {
      "package": 0, "tool": "clippy", "cmd": "cargo clippy --no-deps --message-format=json",
      "arch": "x86_64", "targetTriple": "x86_64-unknown-linux-gnu",
      "features": ["a", "b"],
      "flags": ["--cfg=...", "-Z...", "-C..."]
    },
    {
      "package": 0, "tool": "clippy", "cmd": "cargo clippy --target riscv64gc-unknown-none-elf --no-deps --message-format=json",
      "arch": "riscv64", "targetTriple": "riscv64gc-unknown-none-elf",
      "features": [], "flags": []
    },
    {
      "package": 0, "tool": "lockbud", "cmd": "cargo lockbud",
      "arch": "x86_64", "targetTriple": "x86_64-unknown-linux-gnu",
      "features": [], "flags": []
    }
  ],
  "data": [ // 这里的 idx 指向 idx 数组中的一项检查过程
    {"idx": 0, "file": "path/to/file.rs", "kind": "Clippy(Error)", "raw": "raw report ..."},
    {"idx": 1, "file": "path/to/file.rs", "kind": "Clippy(Warn)", "raw": "raw report ..."},
    {"idx": 2, "file": "path/to/file.rs", "kind": "Lockbud(DoubleLock)", "raw": "raw report ..."}
  ]
}
```

# Misc：使用 Github API 获取仓库基础信息

作为每个仓库基本信息，在 `.repos[].info` 字段中。

```console
# 最后一次提交的时间和 sha
$ export onwer=os-checker repo=os-checker
$ gh api "repos/$onwer/$repo/commits" --jq '.[0] | {sha, date: .commit.committer.date}'
{"date":"2024-08-12T23:24:38Z","sha":"e18daf15d5f850ab8c9f917e71c9e1838e5a32aa"}

# arceos 仓库的主分支名和最后一次推送到主分支的时间、许可协议和一些统计数据
$ export onwer=arceos-org repo=arceos
$ gh api "repos/$onwer/$repo" -q '{branch:{default:.default_branch,pushed_at},license:{key:.license.key,name:.license.name},
  stats:{size,stargazers_count,watchers_count,subscribers_count,forks_count,open_issues_count}}'
{
  "branch": {
    "default": "main",
    "pushed_at": "2024-08-13T10:22:52Z"
  },
  "license": {
    "key": "apache-2.0",
    "name": "Apache License 2.0"
  },
  "stats": {
    "forks_count": 248,
    "open_issues_count": 14,
    "size": 15471,
    "stargazers_count": 468,
    "subscribers_count": 16,
    "watchers_count": 468
  }
}
```
