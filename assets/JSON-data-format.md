# os-checker JSON 输出格式（设计稿）

为了把信息统一组织起来，也为了方便处理输出的数据，需要明确 JSON 数据应该包含的内容，最后附上最简示例。

从逻辑上，应该把完整的 os-checker 检查划分为几个维度：

* 执行检查的环境：运行一次 os-checker CLI 的环境，包括
    * 各种工具信息：Rust 工具链信息（版本号等）、所应用检查工具信息（名称、版本号等)、os-checker 信息
    * 诊断分类信息：一个检查工具可能发出不同类别，比如 clippy 可以发出 Warn/Error 两个类别、lockbud 发出更详细安全检查类别
    * 宿主机器信息：架构、操作系统等信息
    * 检查对象信息：
        * 仓库信息：
            * user 
            * repo 
            * Rust 项目结构信息：由 [cargo metadata] 提供，重要的有 workspace 布局（即 cargoLayout，包含各个 package 的路径）
            * 其他基础信息：分支名、最后一次的提交 sha、时间等，可通过 Github API 获得，见末尾的 Misc
        * package 信息：每个检查工具作用的直接对象
            * name：即 Cargo.toml 中的 `package.name`
            * repo_idx：指向检查对象的某个仓库
            * user：虽然通过仓库信息索引可以知道，但这个数据很常用，复制过来以减轻数据分析的复杂度
            * repo：理由同 user
            * branch：理由同 user；注意，同一个仓库下，不同 branch 的 package 应该视为不同的 packge（目前只考虑默认分支，因此一般情况下 package 就是指 `user repo package`）
            * 定义的 cargo targets[^1]：需要确认，如果同时存在 main.rs 和 lib.rs，检查哪个或者都检查
            * 定义的 features：不仅是基础信息，还用于校验检查命令
* 执行检查的命令：每条检查结果必须通过数字索引对应一条检查命令；将命令和结果分开，是出于数据压缩考虑
    * package_idx 索引：指向检查对象中的某项 package；从某种角度说，完整的检查命令由工作目录（该信息就在检查对象信息内）和 shell 命令两部分组成
    * tool：检查工具名
    * count：该检查命令产生的结果数量
    * duration_ms：执行检查命令花费的毫秒时间；注意，每个检查命令以并行的方式执行
    * 检查命令：来自 os-checker 默认提供或者 os-checker 利用某种方式分析生成或者使用者指定的 shell 命令
    * 编译条件：
        * 架构名和架构目标三元组：即 `<arch>` 和 `<arch><sub>-<vendor>-<sys>-<abi>`
        * 指定的 features：即 `--features ...`
        * 其他 rustc 编译选项：即 `RUSTFLAGS='...'`，上面的编译条件都可以视为 rustc 编译选项，它们直接控制编译哪些源代码（也就是直接影响检查哪些代码），但这里放置除上面之外的选项，比如用于条件编译的 `--cfg` 或者众多不稳定的 `-Z` 编译选项
* 执行检查的结果：问题文件信息；实际上，问题发生的地点应该由 `(文件名, 行, 列)` 描述，但出于简化，只到文件级别
    * cmd_idx 索引：指向一个检查命令
    * 问题文件路径：需统一处理所有工具报告的文件路径；有些工具报告绝对路径，有些报告相对路径，os-checker 尽量统一为相对路径；注意，如果问题来自该 package 之外，那么此时文件指向依赖项的绝对路径
    * 原始检查输出：严格来说，整个 JSON 都可以视为 os-checker 提供的检查结果，但在这个上下文，检查结果与原始检查输出同义
    * 诊断类别

[cargo metadata]: https://doc.rust-lang.org/cargo/commands/cargo-metadata.html#json-format

[^1]: 注意：os-checker 只对 lib 和 bin 进行分析，虽然可以支持对 tests 之类的 targets 分析，但当前我不建议做那么全。

```json
{
  "env": {
    "tools": {
      "rust_toolchain": {
        "host": {...}, // 总是默认最新的 nightly Rust
        "installed": [...] // host 以及所有 repos、packages 和检查工具指定的 rust-toolchain 数组，repo/package/cmd 通过索引指向这
      }, 
      "clippy": {"version": "clippy 0.1.82 (91376f4 2024-08-12)"},
      "lockbud": {"version": "sha...", "date": "...", "rust_toolchain_idx": 1}, // lockbud 需要固定工具链
      "os_checker": {"start": "...", "finish": "...", "duration_ms": 3, "git_time": "...", "git_sha": "..."}
    },
    "kinds": {
      "order": ["Clippy(Error)", "Clippy(Warn)", "Unformatted"], // 类别的优先程度（我认为的）
      "mapping": {
        "clippy": ["Clippy(Error)", "Clippy(Warn)"],
        "fmt": ["Unformatted"]
      }
    },
    "host": {"arch": "x86_64", "kernel": "..."}, // arch 命令和 cat /proc/version
    "targets": [ // target_idx 指向这
      {"triple": "x86_64-unknown-linux-gnu", "arch": "x86_64"},
      {"triple": "riscv64gc-unknown-none-elf", "arch": "riscv64gc"}
    ],
    "target_spec": [ // 尚未确定；完整示例见 https://github.com/os-checker/os-checker/issues/25
      {"arch": "x86_64", "cpu": "x86-64", ...},
      {"arch": "riscv64", "cpu": "generic-rv64", ...},
    ],
    "repos": [
      {"user": "arceos-org", "repo": "arceos", "cargo_layout": [...], "info": {...}, "rust_toolchain_idx": 2}
    ],
    "packages": [ // repo_idx 指向 .env.repos 数组中的一项
      {
        "name": "axstd", "rust_toolchain_idx": "...", // 注意：package 有可能设置和 repo 不一样的 rustc 版本
        "repo": {"repo_idx": 0, "user": "arceos-org", "repo": "arceos", "branch": "main"},
        "cargo": {"targets": [...], "features": [...]}
      }
    ]
  },
  "cmd": [ // package_idx 指向 .env.packages 数组中的一项；spec_idx 指向 .env.target_spec 数组中的一项
    {
      "package_idx": 0, "tool": "clippy", "count": 1, "duration_ms": 1,
      "cmd": "cargo clippy --no-deps --message-format=json",
      "target_idx": 0, "spec_idx": 0, "rust_toolchain_idx": 2,
      "features": ["a", "b"],
      "flags": ["--cfg=...", "-Z...", "-C..."]
    },
    {
      "package_idx": 0, "tool": "clippy", "count": 1, "duration_ms": 1,
      "cmd": "cargo clippy --target riscv64gc-unknown-none-elf --no-deps --message-format=json",
      "target_idx": 1, "spec_idx": 1, "rust_toolchain_idx": 2,
      "features": [], "flags": []
    },
    {
      "package_idx": 0, "tool": "lockbud", "count": 1, "duration_ms": 1,
      "cmd": "cargo lockbud",
      "target_idx": 0, "spec_idx": 0, "rust_toolchain_idx": 2,
      "features": [], "flags": []
    }
  ],
  "data": [ // 这里的 cmd_idx 指向 .cmd 数组中的一项检查命令
    {"cmd_idx": 0, "file": "path/to/file.rs", "kind": "Clippy(Error)", "raw": "raw report ..."},
    {"cmd_idx": 1, "file": "path/to/file.rs", "kind": "Clippy(Warn)", "raw": "raw report ..."},
    {"cmd_idx": 2, "file": "path/to/file.rs", "kind": "Lockbud(DoubleLock)", "raw": "raw report ..."}
  ]
}
```

# `rust_toolchain` 的格式

信息主要来自
* 主机默认的版本：`rustc -vV`；
* 查找并解析 `rust-toolchain.{,toml}` 文件，这是 os-checker 的做法。有一些其他方式，但并不采用它们：
  * `cargo rustc -- -vV` 需要编译才能得到 rustc 的版本信息，因此不使用此命令
  * `rustup toolchain list` 命令不需要编译，可以搜索出现 `(override)` 的那一行，但前提是安装了工具链才行，因此也不使用此命令
  * `rustup show` 会自动安装工具链，并报告当前所需的工具链版本，但缺点它直接打印清单，将来可能会变化，所以不建议依赖它的输出，此外它虽然输出当前工具链安装的 
     targets，但不一定适用于 package：比如默认工具链现在安装了所有 targets，当这个工具链作用于 package，并不意味着个 package 需要所有的 targets。（ [#29]）

```json
{
  "host": {
    "version": "...",
    "commit_hash": "...",
    "commit_date": "...",
    "host": "...",
    "release": "...",
    "llvm_version": "...",
  },
  "installed": [
    { // 第 0 个是 host 工具链
      "channel": "nightly",
      "profile": "minimal",
      "components": ["rustfmt", "clippy"],
      "targets": ["all"]
    },
    {
      "channel": "nightly-2024-05-02",
      "profile": "minimal",
      "components": ["rust-src", "llvm-tools", "rustfmt", "clippy"],
      "targets": ["x86_64-unknown-none", "riscv64gc-unknown-none-elf", "aarch64-unknown-none", "aarch64-unknown-none-softfloat"]
    }
  ]
}
```

[#29]: https://github.com/os-checker/os-checker/issues/29#issuecomment-2308639316

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
