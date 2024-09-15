# os-checker JSON 配置格式（设计稿）

## 快速指定一组仓库，并采用默认的检查方式

```json
{
  "user1/repo": {},
  "user2/repo": {},
}
```

## 在 `{}` 中指定仓库的配置选项

```json
{
  "user1/repo": {
    ...,
  }
}
```

### 如何配置单个仓库

#### `cmds`

使用 `cmds`，自定义某个检查命令。未指定的检查命令以默认方式进行检查。

```json
{
  "user/repo": {
    // 一组自定义检查命令：键为检查工具名称，值为 bool、字符串或者字符串数组
    "cmds": {
      "fmt": "cargo fmt ...",
      "clippy": [
        "cargo clippy --target x86_64-unknown-linux-musl",
        "cargo clippy --target x86_64-unknown-linux-gnu"
      ],
      "lockbud": false
    }
  }
}
```

注意：
* 检查命令字符串暂时只支持单命令，而不支持多命令，也就是不支持 `cmd1; cmd2`。
* 对于字符串数组，每个数组元素表示一次检查。对于上面的 clippy 检查命令数组，它表示进行
  2 次检查，分别在两个目标编译架构上编译和执行检查。
* 对于 bool 值，true 表示按默认方式检查（可以无需设置为 true），而 false 表示不要这项检查。
* cmds 里定义的每项检查是覆盖性质的。
* cmds 里定义的每项检查最终都在 package 的 Cargo.toml 所在的目录中执行，因此无需 cd。

### `packages`

使用 `packages` 指定某个 package 的检查方式（不限于检查命令和其他配置）。

```json
{
  "user/repo": {
    "packages": { // 键为 package name，值为检查配置
      "package1": { // 指定 repo 中名称为 package1 的包的检查方式
        "cmds": {
          "lockbud": false
        }
      }
    }
  }
}
```

## 其他检查配置

通常我们有上面几种配置就足够使用了。但为了简化编写检查命令，有一些额外的配置参数：

### `targets`

`targets` 接收一个字符串或者一个字符串数组。

```json
{
  "user/repo": {
    "targets": [
      "x86_64-unknown-linux-musl",
      "x86_64-unknown-linux-gnu"
    ]
  }
}
```

这表示 target 数组里每个元素都添加到检查命令参数上，它等价于

```json
{
  "user/repo": {
    "cmds": {
      "fmt": [
        "cargo fmt --target x86_64-unknown-linux-musl",
        "cargo fmt --target x86_64-unknown-linux-gnu"
      ],
      "clippy": [
        "cargo clippy --target x86_64-unknown-linux-musl",
        "cargo clippy --target x86_64-unknown-linux-gnu"
      ],
      ...
    }
  }
}
```

当 cmds 和 targets 同时指定时，cmds 具有更高的优先级：

```json
// 只在 x86_64-unknown-linux-gnu 上执行 clippy，
// 但对其他检查工具，依然应用指定的两个 targets。
{
  "user/repo": {
    "targets": [
      "x86_64-unknown-linux-musl",
      "x86_64-unknown-linux-gnu"
    ],
    "cmds": {
      "clippy": "cargo clippy --target x86_64-unknown-linux-gnu"
    }
  }
}
```

如前所述，`packages` 可以使用这些检查选项：

```json
{
  "user/repo": {
    "packages": {
      "pkg1": {
        "targets": "riscv64gc-unknown-none-elf"
      },
      "pkg2": {
        "targets": [
          "riscv64gc-unknown-none-elf",
          "x86_64-unknown-none"
        ]
      }
    }
  }
}
```

### `setup`

使用 `setup` 选项来设置编译环境，它只会执行一次。

它接收字符串或者字符串数组。

目前只适用于 repo，并且在仓库的根目录下执行，不作用于 `packages`。

```json
{
  "user/repo": {
    "setup": "make setup"
  }
}

{
  "user/repo": {
    "setup": [
      "apt install ...",
      "curl ..."
    ]
  }
}
```

Tracking: [#81](https://github.com/os-checker/os-checker/issues/81)

### `features`

这个暂时还在考虑是否要支持。我初步的想法是，只适用于 `packages`：

```json
{
  "user/repo": {
    "packages": {
      "pkg1": {
        "features": "feat1,feat2"
      }
    }
  }
}
```

它和 `targets` 类似，表示附加到每个检查命令上，所以等价于

```json
{
  "user/repo": {
    "packages": {
      "pkg1": {
        "cmds": {
          "fmt": "cargo fmt --features=feat1,feat2",
          "clippy": "cargo clippy --features=feat1,feat2"
        }
      }
    }
  }
}
```

它和 `targets` 选项一起设置的效果：

```json
{
  "user/repo": {
    "targets": ["t1", "t2"],
    "packages": {
      "p1": { "features": "xxx" },
      "p2": { "features": "yyy" }
    }
  }
}

// 等价于写
{
  "user/repo": {
    "targets": ["t1", "t2"],
    "packages": {
      "p1": {
        "cmds": {
          "fmt": [
            "cargo fmt --target t1 -features xxx",
            "cargo fmt --target t2 -features xxx"
          ],
          "clippy": [...] // 类似 fmt 的参数
        }
      },
      "p2": {
        "cmds": {
          "fmt": [
            "cargo fmt --target t1 -features yyy",
            "cargo fmt --target t2 -features yyy"
          ],
          ...
        }
      }
    }
  }
}
```

当 `features` 接收数组，与 `targets` 类似，表示附加每个元素到每个检查命令：

```json
{
  "user/repo": {
    "packages": {
      "pkg1": {
        "features": ["feat1", "feat2"]
      }
    }
  }
}

// 等价于
{
  "user/repo": {
    "packages": {
      "pkg1": {
        "cmds": {
          "fmt": [ // 注意，不是 --features feat1,feat2
            "cargo fmt --features feat1",
            "cargo fmt --features feat2"
          ],
          ...
        }
      }
    }
  }
}
```

# `meta.all_packages`

当它为 false 时，对所有 pkgs 禁用检查。

```json
{
  "user/repo": {
    "meta": { "all_packages": false }, // 先禁用所有检查
    "packages": { ... } // 这里罗列的 pkgs 是应该检查的
  }
}
```

见 [#80](https://github.com/os-checker/os-checker/issues/80)
