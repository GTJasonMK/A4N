# 发布体积与依赖边界

## 保留的发行能力

桌面仍包含完整业务功能，各 Worker 和 capability service 仍是独立进程、独立可替换程序。
发布 catalog 的 `launcher` 现在等于组件自己的 `binary`，不再把角色路由到
`a4n-component-host`。30 个 Worker 共同链接 `a4n-db` Rust `dylib`；Host 与迁移工具
静态链接 DB，以便它们在组件运行库路径建立前也能启动。`a4n-db-session` 静态并入
`a4n-db`，不单独打包。组件发布构建固定 Rust 1.95.0，使用
`-C prefer-dynamic -C lto=off`，并将 DB 动态库及匹配的 `libstd` 放入
`bin/services/`。macOS 便携二进制嵌入 `@executable_path/services` rpath，Tauri
externalBin 嵌入 `@executable_path/../Resources` rpath。Tauri/Server/CLI/迁移 Host
二进制保留仓库默认 fat LTO 和静态 DB。

默认 Host 使用 sidecar 执行文档解析、导出和媒体处理；`--features embedded-transforms` 保留本地实现，`embedded-capabilities` 自动包含它。选择 `minimal` profile 时必须显式启用 `embedded-transforms`；不支持的组合在初始化时失败，不转用隐藏 fallback。原生 Tauri 命令入口保持可用。

`a4n-db` dylib 只共享磁盘上的实现文件；每个 Worker 仍是独立进程并建立自己的连接。
Rust dylib ABI 不承诺跨版本兼容，因此同一发布包/增量部署必须携带与二进制匹配的
共享库和 Rust toolchain。DB 库不变时，单个 Worker 二进制增量发布由 Server supervisor
重启对应 Worker；DB/`libstd` 变化或 capability Sidecar 更新会重启 Host 进程组，保证
旧二进制不会加载不兼容的新库。要支持跨版本独立滚动更新，后续需把 Worker 持久化改为
领域操作 RPC；不能把 SeaORM connection/transaction 对象透传。

9 个 capability Sidecar 将 `a4n-sidecar-bootstrap` 编入各自程序；30 个 Worker 仍在各自
package 中实现 Axum/runtime 启动，当前没有共同依赖这个 helper。尝试把 bootstrap 变成
Rust dylib 时，Worker 与 bootstrap 的 Axum/Tokio crate 格式冲突；因此本次只共享 DB，
不宣称 HTTP/runtime 代码已去重。

## 已收敛的依赖

- Worker 认证头由 `a4n-worker-contract` 定义；28 个 Worker 不再仅为常量依赖 `a4n-worker-client`。Client 保留同名 re-export，不改变 HTTP 请求头。
- SimHash 和对应测试归 Import Review Core，Review 不再依赖包含 ZIP/EPUB/DOCX 解析器的 Import Core。
- Orchestrator 的导出端口必需注入。Export Service 和本地 adapter 复用 Export Core 的格式分派，保留逐章进度、取消和输出格式校验。
- 媒体探测结果及预览帧率归 Media Contract。Host 的媒体文件读取/上传校验层不再重新导出 FFmpeg 实现。
- Import Service 新增 `/import/parse_document`，原 v3 解包端点保持兼容。先更新服务，再更新调用方；新调用方访问没有新端点的旧服务会明确失败，不本地兜底。
- 文本解析共享原有 100 MiB 原始上传上限；新端点容量包含 Base64 扩张与既有 JSON envelope 额度。旧解包端点的限制不变。
- 已迁至 Worker 的 Theme/Admin/Config 宿主实现及仅为它们存在的 facade 已移除。生成仍需的配置读取、权限和用量记录保留；投影和测试复用对应 Core。
- 图片功能只启用业务支持的 PNG/JPEG/WebP；SeaORM 显式保留 PostgreSQL、chrono、JSON、UUID 和数组支持。ZIP 的既有导入兼容范围没有缩减。
- release bundle 禁用 devtools，debug bundle 保留调试能力。打包脚本使用调用方的 `CARGO_TARGET_DIR`，包括带空格的路径。

## 2026-10-06 依赖收敛实测

在相同 Linux x86_64、Rust 1.95.0、前端和发布清单下，固定
`qdrant-client = 1.19.0`，移除 Axum 默认 feature，并将 Tokio feature 改为显式集合。
候选完整 Tauri 包从压缩 `217,901,494` 字节降到 `217,140,317` 字节（减少
`761,177` 字节，`0.349%`）；解包文件从 `559,927,910` 字节降到 `557,920,214`
字节（减少 `2,007,696` 字节，`0.359%`）。包内仍有 30 个独立 Worker、9 个独立
Sidecar；30 个二进制依赖共享 `liba4n_db`，39 个依赖共享 `libstd`，所有包内 ELF
依赖在设置 `bin/services` loader path 后均可解析。

`cargo tree --workspace --duplicates` 已不再报告 Axum、Tokio 或 Tower 的多版本。
剩余重复项来自 Tauri、fastembed 及其它上游构建链，不能通过本仓库的 feature 收敛安全
删除。将 Host 也改成共享 DB 动态库的原型需要关闭 Host 的 fat LTO：`a4n-control`
单体仅减少约 2.5 MB，`a4n-tauri` 反而增加约 1.8 MB，因此不纳入发行配方。

## 验证与度量方式

依赖门禁在 `scripts/checks/check-worker-isolation.sh` 中检查默认 Host 不携带已迁出的 transform core，以及 Worker 不为协议常量反向依赖客户端实现。

对解包后的真实产物运行：

```sh
python3 scripts/checks/package_size.py \
  --installed /path/to/unpacked-package \
  --archive /path/to/package.deb > package-size.json
```

报告统计压缩包字节数、安装文件逻辑字节数及 SHA-256。实际分发的重复文件分别计数，符号链接不重复计入其目标内容；该指标不是文件系统块占用量。

体积比较必须固定源码、工具链、平台、features、LTO/优化参数和前端资源；不得把其他会话变化算入优化收益。依赖减少、源码删除和 Control 单体变小不等于完整安装包变小。

## Linux 组件对照

2026-10-06 使用同一份源码、Rust 1.95.0、Linux x86_64、release opt-level 3、LTO off，
对 30 个 Worker 和 9 个 Sidecar 分别构建静态依赖版与 DB dylib 版。动态版计入一份
`liba4n_db.so` 和一份 `libstd`：

| 产物 | 静态依赖 | 共享 DB dylib | 减少 |
| --- | ---: | ---: | ---: |
| 39 个组件二进制 | 381,990,480 B | 236,186,264 B | 145,804,216 B |
| 加共享 DB 与 libstd | 381,990,480 B | 317,030,344 B | 64,960,136 B (17.01%) |

此对照只覆盖组件程序和共享 Rust 运行库；不含 Host 工具、ONNX Runtime、前端、提示词或
压缩归档，因此不能把 17.01% 外推成完整安装包降幅。DB dylib 为 75,425,592 B，匹配的
`libstd` 为 5,418,488 B。动态版 39 个程序的 ELF 均能解析共享依赖；30 个 Worker 引用
`liba4n_db.so`，9 个 Sidecar 不引用它。

## Linux 对照结果

首批实现为 debug 提交 `4b7f59bb7`，原生启动和退出修复为 `7242c7ad9`、`577c68fb0`。按用户要求直接在 debug 完成，原 agent 工作树与分支已回收。

比较使用相同工具链、平台、优化等级 3 和 ThinLTO，以同一冻结源码为基础，仅叠加本任务修改。基线包含采样时已有的工作区内容，应以 `source.json` 的文件摘要复原，不能只依赖 Git 提交号。两份前端快照共同修复了重复 `useEffect` import，该修复不计收益。

| 实际产物 | 基线 MiB | 优化后 MiB | 减少 |
| --- | ---: | ---: | ---: |
| Debian 安装包 | 255.30 | 250.37 | 1.93% |
| 安装文件逻辑字节 | 652.55 | 638.62 | 2.13% |
| 44 个独立程序合计，不含 Tauri | 502.01 | 490.24 | 2.35% |
| 30 个 Worker 合计 | 276.84 | 268.03 | 3.18% |
| Control | 109.61 | 107.03 | 2.36% |
| Tauri | 129.19 | 127.03 | 1.67% |

最终包包含原生启动和退出修复。压缩包实际为 262,536,588 字节，安装文件合计 669,642,934 字节；证据为本机未跟踪目录 `data/package-size-resume-20260928/final/package-size.json`。原始基线保留于 `data/package-size-20260928/baseline-thin/`，先前中间产物没有替换为最终结果。

生产 release 仍保留原有 fat-LTO。首次 fat-LTO 完整基线在 Control 编译阶段收到 SIGKILL，未形成完整包；上述比例不能直接外推到默认 fat-LTO 或其他平台。

## 验证结果

- 默认 Host、全部 30 Worker、受影响服务，以及 `embedded-transforms` / `embedded-capabilities` 组合的相关测试目标检查通过。
- 原依赖收敛批次的 9 组单测共 128 项通过；Import/Export/Media 三个真实进程测试和 Runtime package IO 的配置/输入错误测试通过。导入夹具曾误把正文写成章标题，修正夹具后通过，未改解析规则。
- 新增原生启动、实例 ID、端口保留和退出路径验证；原生相关 6 项定向测试通过。退出时先停 Agent，再停能力服务，最后停其他 Worker，保持回调和记录服务在需要时可用。
- 最终 release 包通过真实 WebKit/Tauri 登录、项目创建/读取并刷新页面显示、Agent 会话创建/读取、两次实际模型调用并完成 Run、无 token/错误 token 拒绝、包内 ONNX 加载，以及仅向桌面主进程发送 SIGTERM 后全部 A4N 子进程退出。证据为 `final/functional-4e77c137/report/native-ui.json` 与截图 `native-ui.png`。
- 前后完整进程栈通过 CLI 基础合同、场景同步、LLM 生命周期三套检查。只替换原始基线中的 Project Read Worker、保留其余程序的独立更新检查也通过三套 CLI；组合摘要与结果在 `independent-update/composition.json` 和 `independent-update/functional-195c1465/`。
- Worker isolation、capability boundaries、30 Worker / 66 写入范围 ownership、sidecar security、全仓 rustfmt 和 diff 检查通过。隔离门禁首次因 Cargo feature 组合变化触发编译而超时；按单独调用预编译后重跑通过，超时日志保留，未计为成功。

除原始基线目录外，本节简写的证据路径均相对 `data/package-size-resume-20260928/`。测试使用独立数据库、Qdrant、端口和运行目录，未使用稳定调试实例的数据。

保存并校验最终包及全部程序摘要后，已清理本次专用的 `target/x86_64-unknown-linux-gnu/release` 测量缓存，防止旧源码快照被后续构建误复用；`target/debug` 和普通 `target/release` 未清理。

## 编译参数与性能

仅改变 Control 与 Project Read Worker 所属 package 的优化等级；依赖仍为 3，LTO 保持 thin。实际程序大小如下：

| opt-level | Control MiB | Project Read Worker MiB |
| --- | ---: | ---: |
| 3 | 107.03 | 7.84 |
| s | 124.93 | 9.68 |
| z | 127.68 | 9.91 |

`s/z` 均未通过代表程序的体积筛选，因此未继续测试这两个变体的运行性能，也未采用它们。产品优化等级保持 3。编译配方和逐文件摘要保留在 `final/profile-s/`、`final/profile-z/`。

原始基线与优化版在 CPU 18/19 上进行三轮交错比较，每次使用独立的相同数据夹具，并执行三套 CLI。导入耗时中位数为 79 → 81 ms，导出为 84 → 86 ms。初测部分单并发读取 P95 波动较大；仅对章节读取追加三轮、每轮 20 次预热和 100 次测量，P50 中位数为 46.06 → 44.74 ms，P95 为 49.38 → 50.74 ms（+2.76%），三组成对 P95 退化均未超过 5%。不据此宣称所有场景普遍提速。

完整数据与初测波动均保留在 `performance-summary.json`、`performance-rounds.json` 和 `focused-performance.json`。

## Control 符号归因与限制

保留符号后用 GNU strip 剥离，剥离前后 `.text` 摘要一致。按地址/长度去重后的符号名估算覆盖 81.96 MiB（`.text` 为 82.80 MiB）：Orchestrator 10.96 MiB、serde_json 7.93 MiB、DB 7.12 MiB、SeaORM 3.50 MiB、Runtime 3.16 MiB、Control 2.97 MiB、API Contract 2.57 MiB。

泛型实例按名称中的首个业务 namespace 归组，混合别名单独统计。LTO、跨模块内联和泛型会限制归属精度，这些数字不等于可独立删除的 crate 大小。完整分组在 `optimized/control-namespace-estimates.json`；原始符号保留于 `data/package-size-20260928/optimized/control-symbols.txt`。

Windows/macOS 的发布、动态库加载与签名仍需在相应平台验证，本机 Linux 检查不能代替它们。

成熟动态库按平台现有加载机制分发；已确认的 Linux Worker 中，普通 Project Access Worker 只依赖系统基础库，Agent Worker 使用系统 OpenSSL。不能把常量依赖清理声称为每个 Worker 都省掉了一份 OpenSSL。

Host 仍保留大媒体项目包和主题包所需的归档能力。项目包包含大型视频时，既有设计明确避免通过 sidecar JSON 传递全部媒体字节；本次没有为追求依赖图更小而引入这类复制和传输开销。
