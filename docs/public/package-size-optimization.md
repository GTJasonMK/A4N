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

本机详细日志在未跟踪目录 `data/package-size-20260928/`，包含冻结源码摘要、匹配的优化快照、独立依赖图及实际检查输出。冻结快照只用于测量，业务修改直接位于 debug。

- 默认 Control/Tauri 与受影响服务的测试目标类型检查通过。
- `embedded-transforms` 的 Runtime/Control/Host Assembly 测试目标类型检查通过。
- 32 个独立生产依赖图通过新增边界检查。
- 9 组定向单测共 128 项通过，无忽略项；修正了一项引用已退休 wire 类型的既有测试。补充输入校验后，Import Core 的 38 项单测再次通过。
- Import/Export/Media 三个真实进程测试全部通过。导入测试初次因夹具正文误匹配章标题失败，修正夹具后通过，未改变解析行为。
- 冻结前端快照的重复 `useEffect` import 已在两份快照中应用相同修复并记录摘要；前端生产构建通过，此修复不计优化收益，未覆盖当前工作区前端修改。
- 首次 fat-LTO 完整基线在 Control 编译阶段收到 SIGKILL，未形成完整包，不能算作成功基线。
- ThinLTO 匹配基线、完整进程/CLI/安装验证和最终体积结果仍在执行；在结果齐全前不承诺减包比例。
- Windows/macOS 需要在相应平台执行发布和动态库加载检查，本机 Linux 检查不能代替它们。

成熟动态库按平台现有加载机制分发；已确认的 Linux Worker 中，普通 Project Access Worker 只依赖系统基础库，Agent Worker 使用系统 OpenSSL。不能把常量依赖清理声称为每个 Worker 都省掉了一份 OpenSSL。

Host 仍保留大媒体项目包和主题包所需的归档能力。项目包包含大型视频时，既有设计明确避免通过 sidecar JSON 传递全部媒体字节；本次没有为追求依赖图更小而引入这类复制和传输开销。
