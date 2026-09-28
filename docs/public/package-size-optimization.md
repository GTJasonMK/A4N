# 发布体积与依赖边界

## 保留的发行能力

桌面仍包含完整业务功能，各 Worker 仍是独立进程、独立可替换程序。没有合并 Worker，也没有引入 Rust 业务动态库 ABI。

默认 Host 使用 sidecar 执行文档解析、导出和媒体处理；`--features embedded-transforms` 保留本地实现，`embedded-capabilities` 自动包含它。选择 `minimal` profile 时必须显式启用 `embedded-transforms`；不支持的组合在初始化时失败，不转用隐藏 fallback。原生 Tauri 命令入口保持可用。

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

## 本次执行证据

本机详细日志在未跟踪目录 `data/package-size-20260928/`，包含冻结源码摘要、匹配的优化快照、独立依赖图及实际检查输出。接续验证位于 `data/package-size-resume-20260928/`。首批实现为 debug 提交 `4b7f59bb7`；按用户再次确认，业务修改直接位于 debug，原接续 agent 工作树和分支已回收。冻结快照只用于测量。

- 默认 Control/Tauri 与受影响服务的测试目标类型检查通过。
- `embedded-transforms` 的 Runtime/Control/Host Assembly 测试目标类型检查通过。
- 32 个独立生产依赖图通过新增边界检查。
- 9 组定向单测共 128 项通过，无忽略项；修正了一项引用已退休 wire 类型的既有测试。补充输入校验后，Import Core 的 38 项单测再次通过。
- Import/Export/Media 三个真实进程测试全部通过。导入测试初次因夹具正文误匹配章标题失败，修正夹具后通过，未改变解析行为。
- 冻结前端快照的重复 `useEffect` import 已在两份快照中应用相同修复并记录摘要；前端生产构建通过，此修复不计优化收益，未覆盖当前工作区前端修改。
- 首次 fat-LTO 完整基线在 Control 编译阶段收到 SIGKILL，未形成完整包，不能算作成功基线。
- ThinLTO 匹配基线已完成：44 个独立程序合计 502.01 MiB；Deb 压缩包 255.30 MiB，安装文件逻辑字节 652.55 MiB，Tauri 壳 129.19 MiB。
- 匹配优化 Deb 已构建并解包：压缩包 250.36 MiB（减少 1.93%），安装文件 638.26 MiB（减少 2.19%），Tauri 壳 126.67 MiB。证据为接续目录 `optimized/package-size.json`。这两份测量包均未包含后续桌面路径修复，不能宣称修复后的最终安装包已通过验收。
- 匹配优化快照的 44 个独立程序合计 490.24 MiB（减少 2.35%）；30 个 Worker 从 276.84 降至 268.03 MiB（减少 3.18%）；Control 从 109.61 降至 107.03 MiB（减少 2.36%）。这不是完整安装包收益。逐文件字节/SHA 证据为接续目录中的 `optimized/binary-comparison.json`。
- 前后完整进程栈均通过 CLI 基础合同、场景同步、LLM 生命周期三套验证。优化结果位于接续目录的 `optimized/functional-9751eb24`。测试使用独立数据库、Qdrant 和运行目录；与编译并行，耗时不计性能收益。
- Control 保留符号后用 GNU strip 剥离，剥离前后 `.text` 摘要一致。最大单符号是 config/RAG 命令分派（154.5 KiB），其次为视频分派（119.3 KiB）和内容分派（114.8 KiB）；这些单符号不足以代表整个 Control 的归因。原始符号与前 200 项在 debug 的 `optimized/control-symbols.txt` 和 `control-attribution.json`。
- 完整符号表按地址/长度去重后，符号名估算覆盖 81.96 MiB（`.text` 为 82.80 MiB）：Orchestrator 10.96 MiB、serde_json 7.93 MiB、DB 7.12 MiB、SeaORM 3.50 MiB、Runtime 3.16 MiB、Control 2.97 MiB、API Contract 2.57 MiB。泛型实例按名称中的首个业务 namespace 归组；LTO、别名和跨模块内联限制了归属精度，不能把这些数字当作可独立删除的 crate 大小。完整分组在接续目录 `optimized/control-namespace-estimates.json`。
- 实际基线桌面启动失败：Tauri 将资源目录错误地覆盖为服务程序目录，Deb 的服务实际位于 `usr/bin`。后续修复删除该重复推断，沿用 Runtime 的显式目录/可执行文件旁目录解析；资源目录仅用于定位包内 ONNX。修复后的桌面启动、包内动态库加载及最终包体积仍待验收。
- ONNX 资源配置统一移至线程启动前，原生模式与本地 Server 模式共用，覆盖 Linux/macOS/Windows 加载变量；显式目录优先，错误目录明确失败。Linux 定向单测通过，其他平台尚无运行验证。完整 Worker isolation 首次冷编译触发 60 秒超时，保留失败日志，预编译后补跑。
- 本地 Server 桌面实跑通过三套 CLI，并打开实际窗口、确认 Embedding Service 加载包内 ONNX，证据 `patched/functional-d1083279`。原生模式另发现固定三个 Worker 的启动清单已落后于业务迁移，现改用完整 catalog，在 Agent 启动前复用现有带认证的 Control 内部路由，保留进程实例 ID 核验和单独更新边界。
- Debug 接续编译和 5 项定向测试通过。真实 WebKit/Tauri 验证通过登录、项目创建/读取、Agent 会话创建/读取、两次模型调用并完成 Run、未认证/错误 token 拒绝、包内 ONNX 加载；进程清单含全部 30 个 Worker。证据 `patched/functional-43527bc2/report/native-ui.json` 和 `native-ui.png`。这次使用调试版桌面与当前 Chapter Worker v5 配合已冻结的其余程序，验证启动修复，不计入 release 体积或性能收益。
- 测量采用环境覆盖 ThinLTO；产品 release 仍使用原 fat-LTO。`opt-level=3/s/z` 与三轮关键性能对照尚未完成，不宣称参数优化或性能无退化。
- Windows/macOS 需要在相应平台执行发布和动态库加载检查，本机 Linux 检查不能代替它们。

成熟动态库按平台现有加载机制分发；已确认的 Linux Worker 中，普通 Project Access Worker 只依赖系统基础库，Agent Worker 使用系统 OpenSSL。不能把常量依赖清理声称为每个 Worker 都省掉了一份 OpenSSL。

Host 仍保留大媒体项目包和主题包所需的归档能力。项目包包含大型视频时，既有设计明确避免通过 sidecar JSON 传递全部媒体字节；本次没有为追求依赖图更小而引入这类复制和传输开销。
