# os checker

针对 Rust OS crates 的代码检查工具合集和应用，用以督促和提高代码库的质量。

## 二进制工具：`os-checker`

`os-checker` 生成最常见的 Rust 代码检查工具的运行结果报告。

| 工具/命令                                    | 重要性 | 说明                                                                           |
|----------------------------------------------|--------|--------------------------------------------------------------------------------|
| [`cargo fmt`][fmt]                           | 最高   | 代码格式化工具；良好的格式既赏心悦目，又防止无聊的提交改动或合并冲突           |
| [`cargo clippy`][clippy]                     | 最高   | 捕获常见的编码错误，来使 Rust 代码更加地道                                     |
| [`cargo miri`][miri]                         | 最高   | UB 动态检查工具；由 rustc 核心开发者维护，所以几乎不存在误报[^miri-limit]      |
| [`cargo semver-checks`][cargo-semver-checks] | 高     | 检查是否违反版本语义；一个严肃的发版应该遵循语义化版本控制 |
| [`lockbud`][lockbud]                         | 中     | 用于常见内存和并发错误的静态检查工具；[见其论文][tse]                          |


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

## 报告方式（待定）

由于我认为必须保存历史检查结果，来获得更好的分析（时间维度和便于展示特定的提交），这是我能想到的一些办法

* （如果没有的话，自动）fork 原仓库到组织下，然后自动提交一些推送，分析这些推送的结果：这能很好地集成到其他地方，比如一个链接就能指向特定的仓库的分析结果
  * os-checker 可以保存一份统计结果
  * 【假想的高级操作】操作系统仓库可以向 fork 仓库提交某种修正/反馈：比如这些代码分析结果的处理过程（或者说明工具原因并不准确）
* 把检查结果放到 os-checker 仓库下：monorepo 不好管理，数据很乱（无论是分支、子模块还是其他方式），也非常难拓展

## 一些 Github Action

### fork/sync 仓库

* [aormsby/Fork-Sync-With-Upstream-action](https://github.com/aormsby/Fork-Sync-With-Upstream-action) 将 forked 的仓库与上游仓库同步
* 自动 fork 仓库

```yaml
name: Fork Repository

on: [schedule]  # 或者其他触发条件

jobs:
  fork-repo:
    runs-on: ubuntu-latest
    steps:
    - name: Fork the repository
      run: |
        curl -X POST \
        -H "Authorization: token ${{ secrets.GITHUB_TOKEN }}" \
        -H "Accept: application/vnd.github.v3+json" \
        https://api.github.com/repos/ORIGINAL_OWNER/ORIGINAL_REPO/forks
```

### 缓存

* [actions/cache](https://github.com/actions/cache) 缓存编译工件或者任何文件
* [缓存本地 docker 镜像示例](https://docs.docker.com/build/ci/github-actions/cache/#local-cache)
  * [官方阿里云服务器 docker 使用说明](https://help.aliyun.com/zh/ecs/use-cases/install-and-use-docker-on-a-linux-ecs-instance)

```yaml
name: Docker Build & Deploy

on:
  push:

jobs:
  docker:
    runs-on: ubuntu-latest
    steps:
      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3
      
      - name: Cache Docker layers
        uses: actions/cache@v4
        with:
          path: /tmp/.buildx-cache
          key: ${{ runner.os }}-buildx-${{ github.sha }}
          restore-keys: |
            ${{ runner.os }}-buildx-
      
      - name: Login to Docker Hub
        uses: docker/login-action@v3
        with:
          username: ${{ secrets.DOCKERHUB_USERNAME }}
          password: ${{ secrets.DOCKERHUB_TOKEN }}
      
      - name: Build and push
        uses: docker/build-push-action@v6
        with:
          push: true
          tags: user/app:latest
          cache-from: type=local,src=/tmp/.buildx-cache
          cache-to: type=local,dest=/tmp/.buildx-cache-new,mode=max
      
      - # Temp fix
        # https://github.com/docker/build-push-action/issues/252
        # https://github.com/moby/buildkit/issues/1896
        name: Move cache
        run: |
          rm -rf /tmp/.buildx-cache
          mv /tmp/.buildx-cache-new /tmp/.buildx-cache
```

* ~~[rustup docker 镜像](https://hub.docker.com/r/rustdocker/rustup/tags)：（不知道是否官方维护）由于很多时候会固定 rustc 的版本号，所以需要通过 rustup 安装它们~~
* [rustlang/rust](https://hub.docker.com/r/rustlang/rust/)：官方维护的夜间工具链；由于内核代码使用较多 nightly features，fmt/clippy 之类的代码主要用最新的夜间版本
  * 更多使用说明参见 <https://hub.docker.com/_/rust/>
  * `docker pull rustlang/rust:nightly`


### 其他

* [buhenxihuan/Starry/auto_test.yml](https://github.com/buhenxihuan/Starry/blob/x86_64/.github/workflows/auto_test.yml)：利用 pytest 和 allure 报告测试结果
  * <https://buhenxihuan.github.io/Starry/>：静态网页部署示例
  * [kern-crates/testing](https://github.com/kern-crates/testing)：测试脚本仓库


```yaml
# Edit YAML here

foo: 42

a: aaa
c: "casdasd"
d: |
  ada
  dasd
" sadasd ": "sda"

os-checker/os-checker:
  fmt: true
  clippy: cargo clippy -F a,b,c
  miri: |
    cargo miri --test a
  semver-checks: false
  lockbud: aaa

e:
- a
- b: c
- d:
  - e
```

而 toml 的等价写法会啰嗦很多

```toml
["os-checker/os-checker"] # 特殊符号需要使用双引号符号
fmt = true
clippy = "cargo clippy -F a,b,c" # 字符串需要双引号
miri = """
cargo miri --test a
""" # 多行字符串需要三个引号
```
