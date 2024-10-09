# 顶栏按钮

![截图_20241009155101](https://github.com/user-attachments/assets/064db9d9-7248-430b-9d62-16a34f148e8f)

顶栏目前有 6 个可交互的部件，根据编号依次介绍

1. 主页：即 https://os-checker.github.io ，用于展示仓库诊断数量的树状汇总表格；
2. 问题文件树：展示所有诊断的原始输出，但以 pkg 内的文件树结构展示；
3. 统计图：展示可视化的统计数据，目前仅显示一个 pass/defect 仓库计数
    * pass 表示诊断数量为 0 的情况，说明没有检查出问题
    * defect 表示诊断数量非 0 的情况，说明检查出问题
4. 编译目标明细表：与编译目标参数相关的表格；
5. 编译目标下拉框：用于展示应用在所有仓库的所有编译目标上的诊断数量的总计，并与某些组件联动交互
    * 与主页表格联动，可以筛选该编译目标上的诊断数量；与问题文件树联动，可以筛选该编译目标上的诊断输出
    * 该下拉框每一项都是 rustc/cargo 支持的 --target 参数（但除了 All-Targets，它是 os-checker 在编译目标统计中的汇总项，也是默认展示的选项）
6. 帮助说明：链接到此 wiki；
7. 主题切换：默认为系统明暗主题；但点击过该按钮的话，会记录切换后的主题，并在以后访问时，应用该主题。

# 主页诊断数量表

![截图_20241009161254](https://github.com/user-attachments/assets/b2c47a5a-6f6d-41e2-a951-d730da800276)

地址：https://os-checker.github.io

按照标注的顺序介绍功能
1. 仓库计数：pass 表示诊断数量为 0 的仓库数量；total 表示所有被检查的仓库数量；还有一个进度条，它显示 pass/total 计算得到的百分数；
2. 列选择：当表格过宽时，或者你只对某些列感兴趣，那么从这个组件中去掉不想展示的列；
3. 搜索框：在 user、repo、package 三列中搜索内容，并筛选掉不符合搜索条件的行；
4. 排序按钮：你可以单击列头进行升序、降序或取消排序；单击时按 Ctrl 或者 Meta 键，可以[多列排序](https://github.com/os-checker/os-checker.github.io/issues/6)；
5. 仓库详情链接：链接到 `https://os-checker.github.io/{user}/{repo}` 网址，目前它展示仓库级别的问题文件树[^1]；
6. 折叠按钮：当仓库只有一个 pkg 时，每行为完整的 user/repo#package 数据，因此无折叠按钮；但当仓库包含多个 pkg 时，Package 列折叠起来，并在折叠行汇总，此时你可以点击该按钮查看具体 pkg 的诊断数量。

[^1]: os-checker 的 WebUI 采用 SPA （单页应用程序），也就是说，点击链接实际在当前页面切换视图，而不刷新或者打开新的网页。


该表格列介绍
* 序号：由于默认按照 `报告数量` 降序，因此序号为 1 的仓库就是报告数量最多的仓库；最大序号 N 表示有 N 个非 0 诊断数量的仓库（pass + N = total）。
* User：Github ID，比如个人账户或者组织账户（os-checker 目前假设只对 github 仓库进行检查）。
* Repo：仓库名称。
* Package：os-checker 搜索该仓库内的所有 [Packages](https://doc.rust-lang.org/cargo/appendix/glossary.html#package) （os-checker 目前假设同一个仓库内不存在同名的 Packages）。
* 报告数量：所有检查工具报告数量的总计。
* Cargo 列直至最后：每列都是一个检查工具的检查结果数量统计（检查结果在 os-checker 中，简称诊断）。

# 问题文件树

![截图_20241009165605](https://github.com/user-attachments/assets/eaf51ee2-1428-44d5-becc-abd27aafdf60)

地址：https://os-checker.github.io/file-tree 或者 `https://os-checker.github.io/{user}/{repo}`

按编号顺序介绍
1. 编译目标下拉框：默认展示所有编译目标下的检查输出；点击选择查看某个具体的编译目标的诊断；数字表示诊断数量。
2. 文件树：文件树按照诊断数量降序排列；文件树的根是 pkg，而不是具体的某个目录；文件树的节点在大多数情况下是该 pkg Cargo.toml 所在目录的相对路径，但也有一些特殊情况，比如
    * 一个本机的绝对路径：有时诊断发生在该 pkg 之外；
    * `(virtual) CheckerName` 只适用于 Cargo 的诊断：它表示某个具体的检查工具具有 error 输出；导致 error 输出的原因是复杂的，因为它可能来自检查工具，也可能来自 Cargo 本身（编译问题、网络问题等等）；
    *  `Not supported to display yet.`：有些检查工具没有良好的解析格式，因此无法结构化呈现在哪个文件上有诊断；比如 lockbud 直接打印 debug 诊断信息进行报告，那么 os-checker 只能统计它报告了一个数量（即使你看到明细中它有自己的统计数字），而且不知道发生在哪个文件上。
3. 明细栏：按照（我认为的）检查工具的重要程度从左到右排列；右上角标识的数字标识该项的诊断计数；该数字具有颜色标识，红、橙、蓝为（我认为的）从高到低的诊断严重程度；每个明细都有文件位置，因此单击文件树依次处理。一般流程为
    * 首先查看 Cargo 栏发生 error 一行的位置，它表示错误报告；
    * 然后查看其他那些红色和橙色的检查结果，优先处理那些诊断报告；
    * 最后查看蓝色低优先级的检查结果；格式化是最基本的规范，在你的项目中运行 `cargo fmt` 就可以清除该诊断；如果有些诊断结果无法处理也没有关系...

# 统计图

![截图_20241009175251](https://github.com/user-attachments/assets/4d0f2771-17d5-4179-8857-710dc933a921)

地址：https://os-checker.github.io/charts

目前仅为仓库诊断情况计数，也就是主页进度条在编译目标维度上的可视化：
* 横坐标为仓库数量：具体划分为 pass （诊断数量为 0） 和 defect （诊断数量不为 0）两种；每个条形图末端有一个标签，显示了合计和通过率。
* 纵坐标为编译目标：目前有 18 个编译目标（除 All-Targets），数量最多的为 x86_64-unknown-linux-gnu，主要因为它是默认值。

该图是可交互的，可以不显示某类指标（单击图例），支持鼠标悬浮信息。

# 编译目标明细表

地址：https://os-checker.github.io/target

编译目标是 os-checker 必须明确的一个核心内容，因为每个检查命令由以下部分组成

* 检查工具的二进制文件名称：os-checker CLI 会检测和安装检测工具以及检查工具所需的工具链环境；
* 检查工具的命令行参数：每个检查工具有自己的命令行参数；
* 编译参数：检查工具可能与 rustc/Cargo 共用编译参数
    * 虽然不是所有检查工具都需要编译，但大部分必须编译代码才能检查，即便对于静态检查工具，可能也必须到达某个编译阶段（比如 MIR）才能开始检查代码。
    * 其中，一类棘手的编译参数是条件编译相关的参数，比如 target[^2]、features 和 rustc flags。后两种尚未支持。
    * 还有 Cargo 的工具链参数，即 `+toolchain`，os-checker 假设一个检查命令只在一个工具链上被运行，并且 CLI 有自己的方式应用和指定工具链参数。

[^2]: 注意：[target](https://doc.rust-lang.org/cargo/appendix/glossary.html#target) 在 Rust/Cargo 中是一个多义词，不同的上下文具有不同的含义。在 os-checker 中，它指编译目标三元组 (Target Triple)。

条件编译参数之所以棘手，是因为它很难由程序自动识别，从而无法确定一个 pkg 支持哪些编译平台或者条件组合。

os-checker 以混合方式处理 target，它会启发式地在 pkg 和仓库的配置或脚本中搜索 target 名称，然后假设它适用于这个 pkg；当我们观察到检查结果只是因为不适用于该 target，那么需要通过正确的配置来执行正确的检查。

由于大部分仓库或者 pkg 都不那么复杂，因此这种天真地搜索是足够的。不过，对于较为复杂项目布局的仓库，搜索的 target 只能作为一种参考。

指定错误的 target （或者编译条件），[可能在不重要的地方报告上千条错误](https://github.com/os-checker/os-checker/discussions/59)，因此，我们需要人为地编写配置文件来让 os-checker 在特定的 target （或者编译条件）上检查。

这种混合的方式天然地应该分成两个表，并且具有一些关联。也就是说，编译目标信息 (target) 分为两个角度：

* [Sources](https://github.com/os-checker/os-checker/blob/9196ccabf686096a45b0985f74aa927dfddad738/os-checker-types/src/layout.rs#L66-L79): 表示启发式搜索 targets 的路径来源，如 config.toml、rust-toolchain.toml、Cargo.toml、脚本等；
* [Resolved](https://github.com/os-checker/os-checker/blob/9196ccabf686096a45b0985f74aa927dfddad738/os-checker-database/src/targets/mod.rs#L10-L16): 表示生成和应用的检查命令，它表示最终在哪个编译目标上如何检查代码，因此包括 target、toolchain、checker、cmd 信息。[^3]

Sources 和 Resolved 中的 target 会存在差异，原因在于 JSON 配置文件可能部分或者完全指定编译目标，从而带来两个指标：
* used：一个 target 是否同时在 Sources 和 Resolved 中
  * true 表示搜索到的 target 应用到了检查命令上；
  * false 表示搜索到的 target 未应用到检查命令上（比如配置文件指定不要在这个 target 上检查）；
* specified：一个 target 是否在配置文件中被指定检查
  * true 意味着 target 一定在 Resovled 中，因为配置文件直接决定生成什么检查命令；
  * false 意味着未在配置文件中指定 target，通常是这个值，因为大部分情况下配置文件没有指定 target。

[^3]: 你可能会问，为什么不展示配置文件的 target，那是因为这个信息可以被压缩成一个字段，而无需一个表；此外更重要的是，我认为知道最终生成的完整命令比展示配置文件更有价值 —— 这个完整命令包含更多信息，而且如果有人想尝试重现检查结果，可以直接使用它。

## 示例：简单情况

![截图_20241009180605](https://github.com/user-attachments/assets/d0b9d8f3-2863-462c-900d-1e72b2de7abf)

上面展示了 `kern-crates/axalloc` 的 target 明细情况，当你下拉选择 user 和 repo 之后，直接得到这个结果。具体解读是：
* 所有下拉框在只有一个值的时候会自动填写，因此我们阅读下拉框，就可以知道该仓库只有一个 pkg，并只在 x86_64-unknown-linux-gnu target 和最近的一个夜间工具链上检查。
* 从上到下，第一个表为 Resolved，它表明在 axalloc pkg 目录下应用了 4 个检查命令（Cmd），并且附上了检查结果数量（Count） 和检查时间（ms，毫秒）。你可能会疑惑为什么有些 target 和 toolchain 与 cmd 内的并不一致。
    * 上面唯一没有指定 target 参数的是 fmt，因为 `rustfmt`/`cargo fmt` 并不编译代码，它在项目级别进行格式化检查，因此无需指定 target 参数，但它是工具链相关的，因为它是 rustup 的一个组件；这里 os-checker 强制调用主机工具链上的 fmt 进行检查。
    * 而在 toolchain 上，该仓库未指定工具链，所以采用主机工具链。但 cmd 是实际运行的命令，如果要完全复现它，安装最新的工具链是不行的，因为夜间工具链是每天更新的，因此我们需要固定工具链到检查日那天，os-checker 转写了主机工具链。对于 lockbud 和 mirai 这类静态检查工具，由于编译器驱动代码非常不稳定，它们需要固定自己的工具链，所以必须在那个工具链上检查，从而与主机工具链不一致。
* 第二个表为 Sources，它表示该仓库没有识别到任何有路径的 target 来源，因此只在默认的主机 target 上（即 x86_64-unknown-linux-gnu）检查，这被应用到了最终的检查命令上，而且 os-checker 的配置文件没有指定任何 target。

## 示例：embassy

embassy 是比较复杂的代码库，包含 90+ 个 pkgs，但 os-checker [只检查库代码](https://github.com/os-checker/os-checker/blob/4cfa7821513cbb1aadf473fdd6ebb03892f42832/assets/repos-embassy.json)，而不检查大部分示例代码。

它是目前唯一一个在 target 种类和来源方面最丰富的仓库，因此具有很多边缘情况，适合解释 Sources 的功能。

选择 embassy 仓库，我们可以看到工具链直接填上了 1.78，说明它的仓库工具链为稳定版的 1.78。

选择第二行 pkg 中的 embassy-basic-example，可以看到如下结果

![截图_20241009211615](https://github.com/user-attachments/assets/5ccad5ac-7de4-40ed-903d-efb66dc2f45f)

首先，Resolved 表没有任何数据，这是因为橙色下拉框虽然只适用于 Sources 表，但在 Pkg 和 Target 字段上，它们被设计作为共同的筛选条件，选择它们将作用于这两个表 —— 这种联动有利于查询数据差异，从而更大地发挥数据的价值。

如前所述，这是预期的结果，因为我们在配置文件中，禁用了所有示例 pkgs 上的检查，它们的 Resolved 表都没有数据 —— 换句话说，这也就是为什么有两个 Pkg 和 Target 筛选条件的原因，数据源头和差异导致它们必须分开选择。Used 一列的 ❌ 也表明，尽管 os-checker 识别到仓库在该 pkg 上指定了这些 targets，但最终没有应用到检查命令上。

其次，我们可以清晰地看到每个 target 来源及其文件路径，比如 ci.sh 和 ci-nightly.sh 两个脚本文件中包含 thumbv6m-none-eabi、在 rust-toolchain.toml 文件中指定了 `thumbv{6,7}m-none-eabi`、在 xxx/.cargo/config.toml[^4] 中指定了 thumbv7m-none-eabi 等等。这表明，os-checker 记录了每个 target 的搜索方式，如果有人想知道为什么在那个 target 上面检查代码，这个表就是很好的回答。

[^4]: 注意：所有来源路径都是仓库根目录的相对路径。


最后，作为 CargoTomlDocsrsInPkgDefault 来源示例，embassy 的 embassy-executor pkg 通过 Cargo.toml 配置的 [`[package.metadata.docs.rs]`](https://docs.rs/about/metadata) 表表达了一个默认 target 为 thumbv7em-none-eabi 的意图。

![截图_20241009220343](https://github.com/user-attachments/assets/6c615b1f-e8d0-458d-9892-cf2170c7e54f)


它带来的效果为 https://docs.rs/embassy-executor/0.6.0/embassy_executor/

![截图_20241009220131](https://github.com/user-attachments/assets/69da74b2-cf8d-4584-acf9-939b5c9bab37)


[这是一个小众但伪标准化的技巧](https://github.com/os-checker/os-checker/issues/26#issuecomment-2302201030)。
