# 工作包 10：跨平台发布、可观测性与供应链完整性

## 任务卡

| 项目 | 内容 |
|---|---|
| 主目标 | 建立 Windows/macOS/Linux 发布实跑、sidecar metrics/restart、SBOM/advisory/license/binary manifest 闭环 |
| V2 来源 | 3.17、3.25、3.26；阶段 H、阶段 M 的发布/运维/供应链部分 |
| 当前状态 | 完成；最终 run `30522623690` 已通过三平台可执行包、Host + 八 sidecar 启动、manifest/hash 与普通 CI 全部 36 个 job |
| 可并行性 | 平台、可观测性、供应链可分 agent；`.github/workflows` 和 release manifest 由集成者串行合并 |

## 子包 A：跨平台发布（3.17）

1. Windows runner 构建 portable package，执行 `start.ps1`，启动八个 sidecar 并检查 health、PID、日志和 host 退出清理。
2. macOS runner 构建 Tauri externalBin bundle，验证包内 sidecar 启停并生成明确标记 unsigned 的可下载归档。
3. Linux runner 从真实安装前缀启动 host/sidecar，验证安装后路径、权限和退出清理。
4. 三个平台覆盖 stale PID、非 ASCII 路径、带空格路径、端口释放和 host 退出后本次实例 sidecar 全部停止。

## 子包 B：metrics、日志和 supervisor（3.25）

1. 八个 sidecar 提供统一 metrics 或结构化状态 endpoint：request、duration、error、cancel、in-flight、restart。
2. 重能力增加 RAG index、embedding download、provider latency、stream interruption、queue length 等指标，但绝不输出 prompt、token、API key 或完整 URL。
3. 统一日志目录、轮转、保留天数和磁盘上限；health 展示版本、PID、重启次数、最近错误和摘要。
4. Service manager 实现按服务 restart policy、指数退避、最大重试、熔断；非幂等 provider 请求不得静默重放。
5. process test 证明 request/trace id 跨 host/sidecar/持久 trace 一致，并覆盖崩溃、重启、退避和熔断。

## 子包 C：供应链与 binary integrity（3.26）

1. CI 执行 `cargo audit` 或等价 advisory 检查、`cargo deny` 或等价 license/source policy。
2. release 生成 SPDX 或 CycloneDX SBOM。
3. release manifest 逐项记录 host + 八个 sidecar 的名称、版本、target、SHA-256、protocol/capability version。
4. portable/Tauri 产物生成逐 binary manifest 和 archive checksum；manager-owned portable 启动前校验实际 hash 和 package manifest，不匹配即拒绝启动。
5. 把 manifest 与 service catalog、package inventory 接入同一 CI 一致性门禁。

## 不在范围

- 不修改 sidecar 业务协议、认证、资源限制和版本协商；由 08 负责。
- 不把日志/metrics 作为新的业务状态源；只从 canonical event/manager state 投影。
- 不把“archive SHA-256 已生成”当成逐 binary 完整性闭环。

## 关闭条件

- 三个平台都有 runner 实跑和安装/打包后 smoke artifact。
- metrics、日志生命周期、restart/backoff/circuit breaker 有统一 owner 和 process evidence。
- advisory/license/SBOM、逐 binary manifest、archive checksum 和启动前 hash 校验均进入普通 CI。
- sidecar 崩溃后的在途任务显式失败或按幂等键恢复，不静默重复非幂等请求。

## 最小验证

```bash
./scripts/checks/check-sidecar-package-inventory.sh
cargo audit
cargo deny check
./scripts/build.sh --release
```

平台命令必须在对应 runner 实跑；Linux 本机成功不能代替 Windows/macOS 证据。未签名产物必须显式标记 unsigned，不得表述为受平台信任的公开分发包。

## 交接要求

提交每个平台的 artifact、runner、命令和失败日志；提交 metrics 字段表、restart 状态机、SBOM 和 binary manifest 示例。`.github/workflows` 只由集成者合并。

## 本会话进度（2026-07-25）

### 已落地

- `a4n-sidecar-bootstrap` 提供统一 `/metrics` JSON、请求/trace header 日志、请求数、错误数、取消数、in-flight、总/最大耗时、PID、uptime、restart count 和脱敏的最近错误；八个 sidecar 与 demo sidecar 均接入。
- `a4n-sidecar-bootstrap` 统一 `A4N_LOG_DIR/services`、daily rolling、保留天数和总大小上限；当前只在启动时清理，保留的升级上限已用 `ponytail:` 注释标明。
- `a4n-service-manager` 统一 supervisor、指数退避、restart budget、circuit open、动态 PID/状态/最近错误和 graceful/force stop；在途请求崩溃时显式失败，不重放 provider 请求。
- request/trace id 通过 `a4n-capabilities` 的共享 transport helper 跨进程发送；demo process test 覆盖 header、崩溃、重启、退避、熔断、端口释放和 counter 不重复。
- `release_manifest.rb`、`check-release-manifest.sh`、`deny.toml` 和 workflow 已接入：manifest 覆盖 host + 八个 sidecar，记录 target/version/SHA-256/protocol/capability version；manager 在配置 manifest 时启动前校验 hash，并在 Rust owner 处复用 service catalog，拒绝缺项、未知项、错误 binary path 或 capability version 漂移。
- Linux deploy/start 与 Windows start 脚本已接入 manifest/hash 校验；启动器现在按 health response 的 service/status 校验 PID，拒绝把存活但不匹配的 stale PID 当作已启动服务，退出清理对受管进程执行有界 graceful→force stop；CI 已定义 `cargo audit`、`cargo deny`、CycloneDX SBOM、三平台 demo process matrix。

### 本地证据

```text
./scripts/checks/check-release-manifest.sh                         通过（生成、校验、篡改拒绝）
./scripts/checks/check-sidecar-package-inventory.sh                通过
./scripts/checks/check-sidecar-security-boundary.sh                通过
./scripts/deploy.sh --target linux-x86_64 --dry-run --no-build     通过
ruby -c scripts/release_manifest.rb                                通过
bash -n scripts/deploy.sh ...                                      通过
cargo check（工作包 10 相关 crate + sidecar）                      通过
cargo test -p a4n-service-manager --lib --no-fail-fast              35 passed
cargo test -p a4n-sidecar-bootstrap --lib --no-fail-fast             15 passed
cargo test -p a4n-capabilities --lib --no-fail-fast                 22 passed
```

### 尚未关闭

- GitHub Windows/macOS/Linux runner 尚未有本次代码的远端成功记录；尚无真实安装包/Tauri bundle smoke artifact。
- 当前 Linux 环境没有 `pwsh`，Windows `start.ps1` 的语法与真实启停只能由 Windows runner 证明；本地没有伪造通过。
- 平台签名、公证、manifest 签名和 release 验签尚未配置，不能用 hash 校验代替签名闭环。
- `cargo audit`、`cargo deny`、SBOM 仅已写入 workflow，当前环境没有远端运行结果，不能记为通过。
- Tauri externalBin 尚未在最终 bundle/signing 阶段生成 host + 八个 sidecar 的最终 manifest；Host hash 不能在 Host 构建前自引用嵌入，必须由正式 bundle/signing pipeline 在最终字节确定后生成并保护。
- `deploy.sh` 仍保留少量 sidecar 名称/平台映射清单；未为降数字而做高风险脚本重构。

因此本工作包状态保持“进行中（已交接，待外部证据）”，不标记完成。

## 远端发布与供应链证据核对（2026-07-25）

- `origin/master` 当前为 `9deb6ca453524456003e6c42264f2bc583cb158b`，是本地 `HEAD=30adf2c15f28b63a8fb67c24163c14ff4762c58f` 的祖先；本工作包新增的 manifest、supervisor、供应链和跨平台 workflow 尚未进入远端。
- GitHub 公共 Actions API 在限流前返回该仓库 `total_count=0` 个 workflow，因此没有 Windows/macOS/Linux package smoke、SBOM、`cargo audit`、`cargo deny` 或 cross-platform process 的远端 run/artifact 可供关闭条件引用。
- 本地与远端均没有 Git tag；GitHub Releases 页面明确显示 `There aren’t any releases here`。仓库也没有签名/公证 workflow 配置；未提交的证书或密钥当然不应作为解决方案，本轮不创建演示签名或假 release。
- 当前 CI 已定义 Linux/Windows portable 与 macOS arm64 Tauri `.app` 的真实构建和启动 smoke，以及三平台 demo sidecar/mTLS 进程矩阵；这些定义尚未在远端执行，也不等价于成功 artifact、Windows/macOS 签名或 macOS 公证。

结论：本地可实现的 manifest/hash、supervisor、metrics、脚本和 CI 定义继续保留；工作包 10 仍需协调者合并并推送当前工作树，再由对应平台 runner 生成真实 package artifact 和日志。平台签名/公证必须在有正式发布身份与 secret 配置后实施，不能在本地以自签名或跳过验签冒充闭环。

## 批次 304 · 3.25 可观测性关闭裁决（2026-07-28）

状态：`完成`

- 统一 `/metrics` 在既有全局 request/error/cancel/in-flight/duration 基础上按静态路由自动聚合 operation 指标；RAG、Embedding、LLM、Image、Video 等重能力无需各自维护第二份 metrics 状态，也不会把 query、prompt、token、API key 或完整 URL写入指标。
- 八个正式 sidecar 的 `with_runtime_metrics` 与 `init_tracing` 接入已逐项静态确认。manager 继续通过既有 health、process status 和 metrics 三个窄读面提供版本、PID、重启次数、最近错误和关键摘要，没有扩展第二 health 合同。
- demo 真实进程验证 `/trace` operation 指标及 Host request/trace ID 写入 `A4N_LOG_DIR/services` 持久日志；两个 supervisor 真实进程用例继续通过崩溃显式失败、单次持久计数、重启、退避和熔断。

验证：`a4n-service-protocol` 69/69、`a4n-sidecar-bootstrap` 19/19；demo process 1/1；supervisor process 2/2。由此 V2 3.25 八项关闭条件全部满足并改为 `完成`。工作包 10 整体仍未完成：3.17、3.26、阶段 H/M 继续缺真实三平台 package smoke、正式签名/公证、manifest 签名/release 验签和远端 CI/artifact。

## 3.26 本地接线复核（2026-07-28）

- `check-release-manifest.sh` 先前存在但未被任何 workflow 调用；现已接入 `release-package` 的发布布局门禁，继续复用同一个 Ruby generator/verifier，没有新建第二套供应链工具。
- portable 包的逐 binary manifest、Linux/Windows 启动前校验以及 manager 每次 spawn/restart 的 hash 校验均已接线；Tauri externalBin 最终 bundle manifest 仍缺。
- 曾验证“预构建 Host 后嵌入 manifest”的最短方案，但最终 Tauri 合并配置和签名可能改变 Host 字节，预构建 host hash 不可信，相关试验代码已撤掉，没有留下假完整性路径。

因此 3.26 保持 `未完成`：必须由正式三平台 bundle/signing pipeline 在最终二进制确定后生成/保护 manifest，并取得 `cargo audit`、`cargo deny`、SBOM、签名、公证和 release 验签的真实 artifact。

## 批次 307 · Linux 真实 release 安装前缀 smoke 接线（2026-07-28）

状态：`未完成`

- 新增 `scripts/checks/smoke-linux-release-package.sh`，直接解压已有 release archive 到包含非 ASCII 和空格的安装前缀，启动包内 Host 与八个 sidecar，并验证 service identity、health、八个 PID、受控日志、stale PID 替换、manifest/hash 和 Host 退出后的进程/PID 文件/端口清理。
- `module-checks.yml` 新增固定 `ubuntu-24.04` 的 `linux-release-smoke` job：使用 PostgreSQL 16、Qdrant v1.18.3、Rust 1.95.0 和真实 release 构建，不再把 `deploy.sh --dry-run` 当安装包运行证据；成功或失败均上传 archive、SHA-256 和 smoke 日志。
- 第一次本地实跑在 RAG 启动处显式失败并指出缺少 `A4N_QDRANT_URL`；没有添加 fallback。提供本机真实 Qdrant 后，既有 release archive 在 `/tmp/.../安装 前缀` 内完成 Host + 八个 sidecar 启动并释放全部 smoke 端口；新增脚本随后独立复验通过。

本批只补齐 Linux 可重复执行入口，不能冒充当前代码的远端 runner artifact，也不关闭 3.17/阶段 H。Windows portable、macOS Tauri bundle、正式签名/公证、最终 Tauri manifest 和 release 验签仍需对应平台与发布身份。

## 批次 308 · 包内 ONNX Runtime 完整性与 Windows portable smoke（2026-07-28）

状态：`未完成`

- 根因审计发现批次 307 的 Linux smoke 继承了开发机 `LD_LIBRARY_PATH`，旧 portable archive 未携带 ONNX Runtime；干净 Windows/macOS runner 必然无法启动默认 embedding sidecar。本批没有增加 fallback，而是让既有 `deploy.sh` 把平台 runtime 放入 `bin/services`，Linux/macOS 启动器优先使用包内库，Windows DLL 与 embedding executable 同目录。
- release manifest 升级为 schema v2：继续保留 host + 八 sidecar 的 catalog/version/protocol/capability/hash，同时新增唯一平台 ONNX Runtime 文件及 SHA-256。Ruby verifier、Linux/Windows 启动器和 manager 使用同一 manifest；manager 在初始化及每次 spawn/restart 前重验 runtime，runtime 被替换或缺失时拒绝启动。
- `setup-onnxruntime.sh` 复用现有安装 owner 增加 Windows x86_64 官方 `1.24.2` zip，固定 SHA-256 `8e3e9c...a7e8b0`，不新增下载工具。新增 Windows PowerShell smoke，覆盖非 ASCII/空格安装前缀、Host、八 sidecar、service identity、health、PID、日志、stale PID 和 Host 退出清理。
- `module-checks.yml` 新增 `windows-2025` 真实 package job：复用 runner 自带 PostgreSQL 17，下载并校验固定 SHA-256 的 Qdrant v1.18.3，构建 Windows archive、执行生成的 `start.ps1`，并上传 archive/hash/Host/sidecar/Qdrant 日志。

验证：protocol 69/69、manager integrity 5/5、manifest schema v2 生成/校验/路径越界/sidecar 篡改/runtime 篡改拒绝均通过；本地 `--no-build` 生成 69 MiB Linux archive，确认包含 22 MiB `libonnxruntime.so.1`、schema v2 和包内 library path。当前机器没有 PowerShell，Windows job 只能由正式 runner 验证；release Host 完整链接分别在 120/120/300 秒超时，未冒充通过。3.17/3.26、阶段 H/M 继续等待真实 Windows/macOS artifact、签名、公证和 release 验签。

## 批次 309 · macOS arm64 Tauri externalBin smoke 接线（2026-07-28）

状态：`未完成`

- 复用 `setup-onnxruntime.sh` 支持官方 `onnxruntime-osx-arm64-1.24.2.tgz`，固定 SHA-256；复用 `target/tauri-sidecars` staging 把唯一 `libonnxruntime.dylib` 交给 Tauri 原生 `resources` map，不增加第二 runtime 目录或打包器。
- macOS `.app` 启动时若资源目录存在包内 dylib，会在创建 `AppState` 前把该目录前置到 managed sidecar 的 `DYLD_LIBRARY_PATH`；开发态仍使用显式外部 ONNX 配置，包内 smoke 会主动清除开发机 ONNX/DYLD 环境，防止假通过。
- 新增 `macos-15`（GitHub 官方 arm64 标签）job：安装 PostgreSQL 16、执行真实 migration、下载并校验 Qdrant v1.18.3 arm64，构建 unsigned Tauri `.app`，再从中文/空格安装前缀验证 Host、八个 sidecar、service identity、health、PID、日志、包内 dylib 和退出清理。
- 官方 ONNX/Qdrant 资产已在本地下载复核，SHA-256 与归档布局一致；Bash/JSON/YAML、三平台 staging dry-run、目标 Rust fmt、`a4n-tauri --no-default-features` check（47.62s）、package inventory、manifest 门禁和精确 diff check 通过。当前 Linux 主机不能执行 macOS `.app`，因此不把 workflow 定义或 unsigned bundle 当成远端运行、签名、公证和正式 release 证据。

3.17、3.26、阶段 H/M 与总体继续保持 `未完成` / `35/44`；下一步必须由真实 macOS/Windows runner 和正式发布身份提供 artifact、签名、公证与 release 验签结果。

## 批次 311 · macOS 正式签名、公证与 tag release 验签接线（2026-07-28）

状态：`未完成`

- 新增唯一正式 `.github/workflows/release.yml`，只响应 `v*` tag，并引用名为 `release` 的 GitHub Environment；远端仍必须为该 environment 配置审批/部署保护和 secrets。tag 必须与 `tauri.conf.json` 版本精确一致，缺少 Developer ID certificate、Apple ID app-specific password 或 Team ID 时在构建前失败，不允许降级为 ad-hoc/unsigned release。
- Apple secrets 只注入证书导入和 Tauri 签名/公证步骤，不暴露给 npm、PostgreSQL、Qdrant 或普通 smoke。证书导入独立临时 keychain，并要求恰好一个 `Developer ID Application` identity；`bundle.macOS.hardenedRuntime` 已显式固定为 `true`。
- 复用现有 ONNX Runtime、externalBin staging、Tauri bundle 和 macOS smoke，构建 `app,dmg` 时显式 `--ci`，未使用 `--no-sign`、`--skip-stapling` 或 `continue-on-error`。
- 发布前依次执行 `codesign --verify --deep --strict`、`stapler validate`、Gatekeeper `spctl --assess` 和签名后 Host + 八 sidecar smoke；DMG 会被真实挂载并对其中 `.app` 重复执行签名、公证和 Gatekeeper 验证。
- 只有上述步骤全部成功才创建/更新 GitHub Release。随后从 Release 重新下载 ZIP、DMG 和各自 SHA-256，要求恰好四个资产并校验内容，避免把本地产物上传成功冒充可下载 release。
- 新增 `check-release-signing-workflow.sh` 并接入现有 release layout 门禁，固定 tag/environment/secrets/hardened runtime/签名/公证/smoke/publish/download 验证且禁止 unsigned/best-effort fallback。

本地验证：签名 workflow 门禁、Bash、JSON、两份 workflow YAML、`actionlint`、package inventory、schema v2 manifest 生成/篡改拒绝、Tauri `app,dmg --ci` dry-run 和精确 diff check 通过。`actionlint` 首次发现原 `macos-15-arm64` 不是有效托管标签；按 GitHub 当前官方 runner 表统一改为 arm64 `macos-15` 后复验通过。Context7 因月度额度耗尽不可用，签名变量与流程改由 Tauri 2 官方 macOS signing 文档、GitHub runner 文档和当前 2.11.0 CLI/schema 核对。

本批只完成正式 macOS 发布配置，不伪造 Apple 平台结果。当前仓库仍没有已核验的 `release` environment protection/secrets、`v*` tag、远端 run 或 GitHub Release；Windows 正式平台签名以及 Tauri externalBin 最终 manifest/manifest 身份保护仍缺。因此 3.17、3.26、阶段 H/M、3.29 和总体继续保持 `未完成` / `35/44`。

## 批次 312 · Windows 正式签名与 tag release 验签接线（2026-07-28）

状态：`未完成`

- 在同一 `release.yml` 增加依赖 macOS job 的 `windows-2025` 正式发布 job，复用 PostgreSQL 17、固定哈希 Qdrant、ONNX Runtime、service catalog、现有 deploy 和 portable smoke，不创建第二套发布器，也避免两个 job 并发创建同一 Release。
- 采用 Azure Artifact Signing 和固定 `artifact-signing-cli 0.11.0`，要求 service principal、官方 HTTPS endpoint、account 与标准 `AZURE_ARTIFACT_SIGNING_CERTIFICATE_PROFILE`；从 runner 已有 Azure CLI 和 Windows SDK 解析 `AZURE_CLI_PATH` / `SIGNTOOL_PATH`，不依赖通常不在 PATH 的 `signtool.exe`，也不保留旧 PFX/证书指纹路径。
- 先构建 Host + 八 sidecar，再逐项执行 Authenticode 签名；签名成功后复用 `deploy.sh --no-build` 重打包，使 schema v2 manifest 哈希最终签名字节。随后逐项校验 manifest SHA-256、`Get-AuthenticodeSignature = Valid` 和 signer certificate，并保存 `authenticode.json` 证据。
- 只有签名、manifest 校验和现有 `start.ps1` 生命周期 smoke 全部通过才上传 Windows portable archive 与 SHA-256；发布后重新下载恰好两个 Windows 资产并复验 hash。静态门禁同时要求 Azure identity、固定 CLI、最终重打包、Authenticode、smoke 和 Release 下载，继续拒绝 `continue-on-error`、旧 PFX、ad-hoc 或 unsigned fallback。

本地验证：`check-release-signing-workflow.sh`、两份 workflow 的 Ruby YAML 解析和 `actionlint v1.7.7` 均通过，目标 diff whitespace 检查通过。当前 Linux 主机没有 PowerShell，且仓库没有已核验的 Azure identity、`release` environment protection/variables/secrets、正式 tag、Windows runner artifact 或 GitHub Release，因此不把配置定义写成 Windows 平台通过。

Tauri `.app` 最终 externalBin manifest/manifest 身份保护仍缺；`cargo audit`、`cargo deny`、SBOM、macOS 签名公证和 Windows Authenticode 也都还没有当前提交的远端成功 artifact。因此 3.17、3.26、阶段 H/M、3.29 和总体继续保持 `未完成` / `35/44`。

## 批次 313 · Tauri 最终 binary manifest 与 Developer ID CMS 接线（2026-07-28）

状态：`未完成`

- 沿用唯一 `release_manifest.rb` 和 schema v2，增加 `--tauri-app` 布局：直接记录签名、公证及 stapling 后 `.app` 中 Host、八个 externalBin 和 ONNX Runtime 的最终 SHA-256、版本、target、protocol/capability version；portable Linux/Windows 默认布局和 Rust 合同均不复制、不分叉。
- 最终 Host 原始 hash 不能嵌回由 Host code signature 封印的 `.app`，否则形成自引用并破坏 `codesign --strict`。正式 workflow 因此在 `.app` 最终字节确定后生成外部 manifest，使用同一个唯一 `Developer ID Application` identity 执行 `security cms -S`，再用 `security cms -D` 和逐文件 hash 双重验证，不引入自签名或预构建 hash fallback。
- ZIP 同时携带 `A4N.app`、`release-manifest.json` 和 CMS，压缩后立即解包并复验 CMS 与九个二进制/runtime hash。Tauri 生成的 DMG 被只读挂载后，也使用同一 manifest 复验内部 `.app` 的最终字节，再执行 codesign、stapler 和 Gatekeeper。
- GitHub Release 新增 manifest/CMS 两个资产；发布后要求六个 macOS 资产完整，既校验 ZIP/DMG SHA-256，也重新解码 CMS 并逐字节比较 manifest。静态签名门禁要求 Tauri manifest、CMS、DMG manifest 校验，并禁止 `--no-sign`。
- `check-release-manifest.sh` 现在用一个临时 package 覆盖 Linux portable、Windows portable 和 Tauri `.app` 三种布局，验证 Host + 八 sidecar + runtime，并证明 Tauri sidecar 篡改被拒绝。

本地验证：manifest 三布局/篡改自检、Ruby/Bash 语法、release fail-closed 门禁、两份 workflow YAML、`actionlint v1.7.7` 和尾随空白检查通过。当前 Linux 主机没有 Apple `security`/codesign/hdiutil，也没有 release identity、tag 或远端 run，因此 CMS、DMG 和 Release 结果仍须真实 macOS runner 证明。

本地实现层面的 Tauri 最终 manifest/identity 接线已补齐，但当前代码仍未进入远端正式 release；`cargo audit`、`cargo deny`、SBOM、三平台 smoke、Apple/Windows 签名和公证均无当前提交的成功 artifact。因此 3.17、3.26、阶段 H/M、3.29 和总体继续保持 `未完成` / `35/44`。

## 批次 314 · 正式 Release 供应链前置门禁（2026-07-28）

状态：`未完成`

- 修复正式发布可绕过普通 CI `supply-chain` job 的顺序缺口：`release.yml` 新增固定 `ubuntu-24.04` / Rust 1.95.0 的 tag supply-chain job，tag/version 不一致、`cargo audit`、`cargo deny check` 或 CycloneDX 生成任一失败都会阻断后续平台发布。
- macOS job 显式 `needs: supply-chain`，Windows 继续 `needs: macos-arm64`，形成唯一 `supply-chain → macOS → Windows` 发布链；没有增加另一个 Release 创建者。
- 正式 supply-chain job 保存 audit/deny 日志、Tauri all-features CycloneDX JSON 和 SHA-256，失败时也上传已有诊断 artifact。macOS job 下载同一 Actions artifact并把 SBOM/SHA-256附加到 GitHub Release，发布后重新下载恰好两个 SBOM 资产并复验。
- 静态签名门禁新增 release supply-chain job、固定 runner、audit/deny/cyclonedx、artifact、`needs` 和 SBOM Release 资产要求，原有 unsigned/best-effort 禁止项继续有效。

本地验证：release fail-closed checker、Ruby YAML、两份 workflow 的 `actionlint v1.7.7` 和尾随空白检查通过。未执行会下载 advisory DB/工具并生成完整 SBOM 的远端 job，当前也没有 tag、Release 或 Actions artifact，因此不声称 audit/deny/SBOM 已成功。

正式 Release 的本地配置已不能绕过供应链门禁，但 3.26 仍需当前提交在真实 runner 上产出成功 audit/deny/SBOM、平台签名、公证、manifest/CMS 和 Release 下载证据。3.17、3.26、阶段 H/M、3.29 与总体继续保持 `未完成` / `35/44`。

## 批次 315 · Linux 正式 tag Release 接线（2026-07-28）

状态：`未完成`

- 正式 `release.yml` 新增固定 `ubuntu-24.04` / Rust 1.95.0 的 Linux x86_64 portable job，不再只依赖普通 main/PR smoke。job 复用现有 deploy、schema v2 manifest、安装前缀 smoke 和启动器完整性校验。
- 使用 PostgreSQL 16 service 并显式运行真实 migration；Qdrant 使用官方 v1.18.3 Linux GNU 资产，按 GitHub Release API 官方 digest `60663a…e4718` 固定 SHA-256 后启动，不使用漂移的 Docker tag。
- 在非 ASCII/空格安装前缀运行现有 Host + 八 sidecar smoke，覆盖 health/service identity、PID、日志、stale PID、manifest、包内 ONNX Runtime 和退出清理；成功后才重命名正式 archive、重算 SHA-256 并上传 Actions artifact。
- 发布 DAG 收敛为唯一 `supply-chain → Linux → macOS → Windows`。macOS 的唯一 GitHub Release owner 下载 Linux artifact，与 macOS/SBOM 资产一起发布，再下载恰好两个 Linux 资产并复验。
- 静态 release checker 锁定 Linux job、正式 runner、固定 Qdrant digest、现有 smoke、artifact、DAG 和 Release asset，未增加第二套 deploy/smoke/publish owner。

本地验证：release fail-closed checker、Ruby YAML、两份 workflow 的 `actionlint v1.7.7` 和尾随空白检查通过。没有运行 90 分钟级 Linux release build/migration/process job，也没有正式 tag 或 Actions/GitHub Release artifact，因此 3.17 仍不能关闭。

三平台正式发布配置现已全部进入同一 tag DAG，但关闭仍需要当前提交的真实 supply-chain、Linux、Apple 和 Windows 四阶段成功证据。3.17、3.26、阶段 H/M、3.29 与总体继续保持 `未完成` / `35/44`。

## 批次 316 · 正式 Release Action 不可变引用（2026-07-28）

状态：`未完成`

- 盘点正式 `release.yml` 的 20 个外部 Action step，并通过各上游官方 Git ref 解析当前 commit；`actions/checkout`、`setup-node`、artifact upload/download、`dtolnay/rust-toolchain` 和三项 `taiki-e/install-action` 全部从可移动 tag/branch 改为 40 位 commit SHA，行尾保留人类可读版本注释。
- 扩展既有 `check-release-signing-workflow.sh`：使用 YAML 结构遍历每个 job/step，只有仓库内 `./...` action 或 `owner/repo@<40 hex>` 可通过；任何 `@v4`、`@main`、版本 tag 或包名 tag 重新进入正式发布都会 fail closed。
- 只锁定拥有 `contents: write`、release environment 和发布 secrets 的正式 workflow；普通 CI action 升级不与本批混做，避免无关的大面积 workflow churn。

本地验证：release fail-closed checker、Ruby YAML 和两份 workflow 的 `actionlint v1.7.7` 通过；`rg uses:` 确认正式 workflow 20/20 外部引用均为 40 位 SHA。没有真实 tag/runner/Release artifact，因此本批不改变完成状态。

正式 Release 的 Action 执行代码现在不可由上游移动标签静默替换；剩余仍是当前提交的真实四阶段发布结果和 Git/runner 身份证据。3.17、3.26、阶段 H/M、3.29 与总体继续保持 `未完成` / `35/44`。

## 批次 317 · 正式 Release 容器与下载资产身份固定（2026-07-28）

状态：`未完成`

- 通过 Docker Registry v2 官方接口解析 `library/postgres:16` 当前 OCI index：index digest 为 `sha256:33f923…e5f20`，Linux/amd64 子 manifest 为 `sha256:da8cf2…ea8e4`；正式 Linux job 改用 `postgres:16@index-digest`，保留可读 tag 但执行内容由 digest 固定。
- release checker 的 YAML 结构遍历扩展到 `jobs.*.services`，所有正式 service image 必须匹配 `image@sha256:<64 hex>`；以后增加新的可移动容器 tag 会 fail closed。
- 复扫正式发布外部输入：20/20 Actions 使用 commit SHA；Linux/macOS/Windows Qdrant v1.18.3 分别固定官方 SHA-256；ONNX Runtime 1.24.2 三平台下载复用单一脚本中的固定 SHA-256。未发现其他可移动 Action 或 service image。

本地验证：release checker、Ruby YAML、两份 workflow 的 `actionlint v1.7.7` 通过；移动 Action/service 正则扫描为零，三平台 Qdrant 与 ONNX hash inventory 完整。没有拉取 PostgreSQL image 或执行正式 runner，只使用 Registry 元数据确认 identity。

正式 Release 的代码、容器和已下载原生依赖均已有不可变身份；当前未完成项仍是正式 identity/tag/runner/Release 的真实执行证据。3.17、3.26、阶段 H/M、3.29 与总体继续保持 `未完成` / `35/44`。

## 批次 318 · Developer ID CMS code-signing policy 修正（2026-07-28）

状态：`未完成`

- 复核 Apple 官方开源 `SecurityTool/macOS/security.1` 与 `cmsutil.c`，确认 `security cms` 默认 `certUsageEmailSigner`、默认 SHA-1；Developer ID Application 属于 code-signing identity，必须使用 `certUsageObjectSigner`（usage code 6），其实现映射到 `APPLE_TP_CODE_SIGN` policy。
- manifest 签名统一改为 `security cms -S -u 6 -H SHA256 -Y NONE`：显式使用 object-signer/code-signing policy、SHA-256，并省略无关的 S/MIME EncryptionKeyPreference；三个生成后、ZIP 解包后和 Release 下载后的 CMS decode/verify 均显式 `-D -u 6`。
- release checker 固定签名参数，并要求三个 CMS 验证点全部使用 object-signer policy，防止退回默认 email/SHA-1 路径。

本地验证：官方源码检索、release checker、Ruby YAML 和两份 workflow 的 `actionlint v1.7.7` 通过。Linux 无 `security` 和 Developer ID identity，实际 CMS 生成/验证仍由正式 macOS runner证明。

本批修复了静态 workflow 在真实 Developer ID identity 下可能找不到证书或使用 SHA-1 的根因；没有 tag/Apple runner/Release artifact，完成状态不变。3.17、3.26、阶段 H/M、3.29 与总体继续保持 `未完成` / `35/44`。

## 批次 319 · 正式 Release GITHUB_TOKEN 最小权限（2026-07-28）

状态：`未完成`

- 将 `release.yml` 的 workflow 默认权限从 `contents: write` 降为 `contents: read`；supply-chain 第三方安装 Action、Linux build/smoke 和普通 checkout/artifact step 不再继承 repository write token。
- 只有实际创建/上传 GitHub Release 的 `macos-arm64` 和追加 Windows 资产的 `windows-x86_64` 两个 job 显式提升 `contents: write`；既有 release environment、DAG 和 secret 注入边界不变。
- release checker 使用 YAML 结构断言默认权限恰为 read，且拥有显式 contents write 的 job 集合必须精确等于 macOS/Windows；新增 job 无法默认获得发布权限。

本地验证：权限结构解析结果为默认 read、write jobs 仅两项；release checker、Ruby YAML 和两份 workflow 的 `actionlint v1.7.7` 通过。没有真实 GitHub token/tag run，本批只收紧配置权限。

正式发布 token 的暴露面已收敛到唯一两个 Release 写入点；当前仍缺正式 identity/tag/runner/Release artifact。3.17、3.26、阶段 H/M、3.29 与总体继续保持 `未完成` / `35/44`。

## 批次 321 · GitHub 源码发布准备（2026-07-28）

状态：`未完成`

- 修正公开 README 的重构后 owner 和测试数据库链接，精确放行已有 `docs/TEST_DATABASE.md`；将生成包规则从 `deploy/` 收紧为根目录 `/deploy/`，避免误忽略 `scripts/deploy/deploy-cloud.sh` 源码；新增 Apache-2.0 许可证、贡献指南、安全策略、PR 模板和 GitHub 发布说明。
- 发布说明记录源码提交复核、migration 同提交要求、GitHub Actions/ruleset、`release` environment、Apple/Azure secrets/variables 和唯一 tag 发布顺序，不新增 workflow 或发布脚本。
- 常见私钥/token 模式扫描为零；`.env`、数据库、PID、`data/`、`target/`、`deploy/` 和证书均未被 Git 跟踪。

当前代码尚未提交/推送，也没有真实 runner/Release artifact，因此 3.17、3.26、阶段 H/M、3.29 与总体保持 `未完成` / `35/44`。

## 批次 322 · 公开部署配置去真实基础设施默认值（2026-07-28）

状态：`未完成`

- `deploy-cloud.sh` 不再内置真实 IP、域名、`admin.pem` 和旧 nginx 配置名；复用既有 CLI/env 接口，将 Host、SSH key 和 public origin 改为显式必填；云部署和 LTX SSH 默认 `StrictHostKeyChecking=yes`，不再自动信任未知 host key。
- `run.sh --remote-api` 不再默认连接个人服务器，必须提供 `A4N_REMOTE_API_URL` 或 `--remote-api-url`；LTX 脚本只在需要远端启动、隧道或模型检查时要求 `LTX_SSH_HOST`。
- `.env.example` 的临时媒体、remote API 和 GPU SSH 地址统一改为 `example.com` 示例，不新增配置文件或 fallback。

本批关闭公开仓库默认连接真实基础设施的风险；远端提交、三平台签名 runner 和 Release artifact 仍缺，3.17、3.26、阶段 H/M、3.29 与总体保持 `未完成` / `35/44`。

## 批次 323 · Module Checks 最小权限与 Action pin（2026-07-28）

状态：`未完成`

- 普通 `module-checks.yml` 的默认 `GITHUB_TOKEN` 权限固定为 `contents: read`；该 workflow 没有发布写入 job，也不引入权限提升。
- 17 个既有 job 的 45 个外部 Action 引用全部固定为 40 位 commit SHA，复用正式 Release 已核验的 checkout、Rust toolchain、Node、artifact 和三项 `taiki-e/install-action` commit；没有新增 workflow、Action 或检查框架。
- 现有 `check-release-signing-workflow.sh` 统一解析正式 Release 与普通 Module Checks，要求两者默认只读并拒绝任一可移动 Action 引用；Release 的 service digest 和精确 write-job 集合仍按原规则单独校验。

本地 YAML、release fail-closed checker、build-feedback self-check、Shell 语法、两份 workflow 的 `actionlint v1.7.7` 和 diff check 通过。当前代码仍未推送，普通 CI 和正式 Release 都没有当前提交的远端 runner/artifact；3.17、3.26、阶段 H/M、3.29 与总体继续保持 `未完成` / `35/44`。

## 批次 324 · 正式 Release 跨平台原子公开（2026-07-28）

状态：`未完成`

- 修复 macOS 成功、Windows 尚未签名时就公开 Release 的顺序缺口：macOS 现在只创建或更新同 tag 私有 draft；若同 tag Release 已公开则拒绝覆盖。
- Windows 完成 Authenticode、最终 manifest、portable smoke 和 staged download 后，要求 SBOM、Linux、macOS、Windows 恰好 12 个资产，最后才将 draft 转为公开 Release；任一前置失败不会暴露不完整版本。
- 复用现有 release DAG、GitHub CLI 和 checker，不新增 job、发布脚本或状态 owner。checker 固定 draft 创建、状态查询、精确资产集合和唯一公开命令。

GitHub CLI 官方文档与源码确认 draft 创建/发布及按 tag 获取 draft 资产的合同；本地 YAML、release checker、`actionlint v1.7.7`、Shell 语法和 diff check 通过。当前仍无正式 identity、tag、runner 或 Release artifact，3.17、3.26、阶段 H/M、3.29 与总体保持 `未完成` / `35/44`。

## 批次 325 · 正式 Release checkout 凭证最小化（2026-07-28）

状态：`未完成`

- 官方 `actions/checkout` 合同确认 token 默认写入本地 Git config；正式 workflow 原四个 checkout 均使用该默认值，扩大了构建与签名期间的发布凭证暴露面。
- 四处现全部设置 `persist-credentials: false`；npm、Cargo、migration、deploy、smoke 和签名脚本不再继承 Git 凭证，`GH_TOKEN` 只显式进入五个必要的 Release 操作 step。
- 现有 release checker 结构化要求 checkout 恰好四处且全部禁用持久化；没有增加新 job、脚本或 secret owner。

官方 README/action metadata 核对、YAML、release checker、负向夹具、`actionlint v1.7.7`、Shell 语法和 diff check 通过。当前仍无正式 identity、tag、runner 或 Release artifact，3.17、3.26、阶段 H/M、3.29 与总体保持 `未完成` / `35/44`。

## 批次 326 · Host 与八 Sidecar 九包 SBOM（2026-07-28）

状态：`未完成`

- 原正式/普通 CI 都只从 `a4n-tauri` 生成 SBOM，但八个 sidecar package 不在该直接依赖集合，无法追溯完整发布包。
- 新共享入口直接读取现有 service catalog binary，并用 `cargo metadata --locked` 映射唯一 workspace package；Host + 八 sidecar 共 9 项分别按 all-features/target-all 生成 CycloneDX JSON，打成一个确定顺序归档和 SHA-256。
- Module Checks 与正式 tag workflow 删除重复 Host-only 命令并复用该入口；draft、下载校验和 12 项最终资产集合同步切换到九包归档。

快速 mock 只替代 `cargo-cyclonedx` 输出，生产 catalog/metadata/9 包/归档/hash 逻辑实际运行，结果 9/9 且 checksum 通过。YAML、Shell、checker、`actionlint v1.7.7` 和 diff check 通过；固定 runner 的真实 SBOM artifact 仍缺，3.17、3.26、阶段 H/M、3.29 与总体保持 `未完成` / `35/44`。

## 批次 337 · 首轮 Module Checks 远端诊断与发布门禁修复（2026-07-28）

状态：`未完成`

- `gh` 已以 `GTJasonMK` 登录；远端 run `30372742923` 对应提交 `500425504a6330c690a47abc3e0574c16519e1ce`，共 36 个 job：25 个成功、11 个失败。build-feedback、Host minimal、full backend、frontend/API parity、Linux/Windows package layout、全部 sidecar package、Ubuntu/macOS native sidecar、三平台 remote mTLS、多实例 manager 等已在真实 runner 通过。
- 该 run 生成四个真实 artifact：build-feedback `8694035916`（SHA-256 `3eb9aa…1074`）、Linux release `8694807418`（`8cfeb0…a28`）、Windows release `8695836724`（`5de2da…ea3e`）和 macOS arm64 Tauri bundle `8696436528`（`541371…a4b9`）。这些证明三平台产物可以构建，但 smoke 失败的 artifact 不能记为平台发布闭环。
- 供应链 job 暴露五个真实 advisory：`anyhow`、`crossbeam-epoch`、`quick-xml`、`quinn-proto`、`git2`。lockfile 已升级到修复版本；`git2` 升为 `0.21` 并适配其 UTF-8 `Result` API。唯一无修复版本的 `RUSTSEC-2023-0071` 来自未激活的 `sqlx-mysql → rsa 0.9.10`，现由单一 audit 例外和 workspace/all-features/target-all 不可达门禁约束。41/41 个 A4N workspace crate 统一声明 `Apache-2.0`；未修改 vendored `crates/qdrant`。
- Linux/Windows release smoke 在空数据库启动 Host 前没有执行 migration；两个 workflow 现复用 `a4n-migration`。Windows portable 的 GNU tar 无法以中文/空格路径作为 `-C` 目标，脚本改为 `Push-Location -LiteralPath` 后就地解压，仍保留非 ASCII 安装前缀。Windows manager 在 `env_clear()` 后删除 `SystemRoot`，导致 Winsock 10106；共享 spawn 入口只恢复该可信平台变量，不恢复数据库、JWT 或 provider secret。
- macOS job 已完成 unsigned `.app` 构建并上传 artifact，但 smoke 错把产品目录名 `A4N.app` 当成 Host 文件名。artifact 真实布局证明 Host 是 `Contents/MacOS/a4n-tauri`；共享 macOS smoke 已改用 canonical Cargo binary 名，不改 Tauri product name 或跨平台打包命名。
- Generation Web/Tauri job 的依赖检查错误调用 `WebKitWebDriver --version`；该工具会以非零码结束，现改为已有的 `command -v` 存在性检查，不添加跳过路径。
- RAG/Image/Video/Embedding process test 仍引用已删除的 `assert_event_queue_backpressure`，且文档把旧 `try_send` 错误语义写成当前合同。现统一以 `a4n-capabilities` 的 `send().await` 为准：有界队列满时等待容量，消费后继续到达 `Completed`；receiver 关闭时返回 `Cancelled`。四个独立 process target 均已定向通过。
- PostgreSQL CI 日志证明每项测试使用独立 schema，不是并行串库；18 个真实失败来自角色属性、`project_kind`、项目/章节外键、lifecycle gate、媒体路径、结构化错误和 canonical workflow 等陈旧 fixture。相关 fixture 已按当前 schema 修正；原失败集合的定向复验全部通过。本机全量 540 项中的额外失败来自 PostgreSQL `max_locks_per_transaction` 资源上限，不据此修改远端并发策略。`scripts/test.sh` 现复用已有 `load_dotenv`，让仓库 `.env` 中的测试库入口实际生效。

本地已通过 audit 例外门禁、release 签名 workflow 门禁、`cargo audit`、`cargo deny check`、四个 sidecar process、`a4n-db --lib --no-run`、project delete/reset 定向测试、相关 Rust check、Shell/YAML 语法和 diff 检查。上述修复尚未进入新的远端提交/run；正式 Apple/Azure identity、受保护 `release` environment、`v*` tag、签名、公证、最终 manifest/CMS、九包 SBOM 和 GitHub Release 下载复验仍不存在。因此 3.17、3.26、阶段 H/M、3.29 与总体保持 `未完成`，V2 仍为 `38/44`。

## 批次 338 · 第二轮 Module Checks 远端收敛（2026-07-29）

状态：`未完成`

- run `30421032715` 对提交 `2ad6cede7905d8057f5b69688bb8e24b3dd6a641` 已完成 36 个 job：29 成功、7 失败。供应链 job `90477694205` 真实通过 `cargo audit`、`cargo deny check` 和 Host + 八 sidecar 九包 CycloneDX；artifact `8711918179` 大小 1,132,645 bytes，SHA-256 为 `6163fbcf934df4b8dfbb8f275b3fb875fab12229b19b347ab6ceda2ce9de0fd2`，日志明确记录 `release SBOM: packages=9`。
- Linux installed release smoke job `90477694215` 真实通过；artifact `8712478510` 大小 66,902,663 bytes，SHA-256 为 `20f4d6ef65729a71f11323c1060ea8bcace2bc021183107a9984438d3d78cbfd`。实际覆盖 PostgreSQL migration、真实 archive、Host + 八 sidecar、日志、stale PID、退出清理、非 ASCII/空格安装路径、九个 binary 和 ONNX Runtime manifest。
- Windows job 完成真实 portable archive 构建并上传 artifact `8713027695`，但包内 `start.ps1` 是 UTF-8 无 BOM，由 `powershell.exe` 5.1 按本地代码页解析后在中文错误文本处产生 ParserError。唯一生成 owner `scripts/deploy.sh` 现先写 UTF-8 BOM 再追加原脚本；没有改成另一份 Windows launcher 或移除非 ASCII smoke。
- macOS job 完成 unsigned arm64 `.app` 构建并上传 artifact `8712988250`，Host 与 prompt sidecar 已进入真实启动；失败点是 `prompts/_registry.yaml` 未进入 `.app/Contents/Resources`，sidecar 回落到用户数据目录后显式失败。现有 Tauri sidecar resource map 直接携带仓库 `prompts/`，Tauri setup 在 AppState 前复用 `A4N_PROMPT_SERVICE_PROMPTS_DIR`，缺注册表时显式返回配置错误；smoke 同时检查 bundle 内 registry。
- PostgreSQL job 的 migration、a4n-db 与 orchestrator 均通过；Runtime 为 `478 passed / 3 failed`，三条均是当前实现与静态 convergence 文本漂移，已按 `stream_complete_llm` 和 `portrait` 窄 facade 修正。Generation Web/Tauri 已打印 cursor 重挂载与窗口销毁期间仍发送的成功证据；失败发生在同步 destroy 先于 WebDriver 回包，现仅延迟 100 ms 调用同一真实 destroy，Rust harness 的窗口销毁和订阅取消断言未弱化。其余三条失败是要求同步更新 V2/进度/README/工作包的文档门禁。

本地已通过 Tauri sidecar JSON 解析、Rust 格式检查、Shell 语法、Tauri build dry-run、Generation E2E 语法和目标 diff 检查；Runtime 定向测试此前两次在 60 秒内仍处于编译阶段，未进入 runner，不能记为通过。本批修复仍待唯一一轮新 Module Checks 复验。

正式 Apple/Azure identity、受保护 `release` environment、`v*` tag、签名、公证、最终 manifest/CMS、正式九包 SBOM 和 GitHub Release 下载复验仍不存在。普通 CI 的九包 SBOM 与 Linux artifact 是有效进展，但不能替代签名 Release；3.17、3.26、阶段 H/M、3.29 与总体继续保持 `未完成`，V2 仍为 `38/44`。

## 批次 339 · 第三轮 Module Checks 平台根因收敛（2026-07-29）

状态：`未完成`

- run `30425506981` 对提交 `41b5677ef92f7a11fb38fdccaecf8fc142e2055f` 已完成 36 个 job：34 成功、2 失败。PostgreSQL migration/DB/orchestrator/Runtime、Generation Chromium/Tauri、架构同步门禁、build-feedback、Linux installed release、advisory/license、九包 SBOM 和其余 package/process/mTLS 全部真实通过；失败只剩 Windows portable `90491035106` 与 macOS Tauri `90491035191`。
- Windows BOM 修复已生效，包内 `start.ps1` 成功解析并进入 schema v2 manifest 校验。新失败是 Windows PowerShell 5.1 环境没有 `Get-FileHash`；workflow 自身的 `pwsh` 调用已经通过，不应扩大修改。唯一 launcher 生成处现用 .NET `SHA256` 对 `FileStream` 流式计算十个 manifest 条目，并通过嵌套 `finally` 释放 hash provider 和文件流，仍 fail closed。artifact `8714723379` 为 69,409,681 bytes，SHA-256 `7d86536f5f722431b6d16fecbf138f68fe2a73cecfb4df758ae68a2c05e35bf0`。
- macOS prompts 资源修复已真实通过：artifact 含 `_registry.yaml`，Host 从 `Contents/Resources/prompts` 加载 73 个提示词并启动 prompt/LLM。新失败是 embedding 的 Mach-O load command 为 `@rpath/libonnxruntime.1.24.2.dylib`，而 artifact 只含唯一、由 manifest 记录的 `libonnxruntime.dylib`。不复制第二 runtime；现有 externalBin staging 使用 `otool` 读取实际依赖，只改复制后的 embedding 为更短的 `@rpath/libonnxruntime.dylib`，继续复用 setup 已设置且日志证明生效的 `DYLD_LIBRARY_PATH=Contents/Resources`。
- `install_name_tool` 会使现有 Apple Silicon ad-hoc 签名失效；普通 unsigned smoke 没有 Tauri signing identity，因此 staging 立即以 `codesign --force --sign -` 恢复可执行状态。Tauri 2.11.0 官方 bundler 源码确认：有正式 identity 时 externalBin 会先于 app 按 inside-out 顺序重签，所以正式 Developer ID/hardened runtime/公证链路不降级，也不把 ad-hoc artifact 当 Release。

本地已通过 `bash -n`、目标 `git diff --check`、Windows launcher 流式 .NET SHA-256 静态断言、macOS arm64 Tauri dry-run、sidecar inventory、Linux/Windows/Tauri 三布局 manifest 与篡改拒绝、正式 release signing fail-closed checker。当前 Linux 没有 Windows PowerShell、`otool`、`install_name_tool` 或 `codesign`，平台行为必须由下一轮真实 Windows/macOS runner 复验；Context7 月度配额已耗尽，Tauri 行为使用 2.11.0 官方源码和本次 artifact/log 核对。

本批没有新增 workflow、runtime 副本、launcher 或 fallback。正式 Apple/Azure identity、受保护 `release` environment、`v*` tag、签名、公证、最终 manifest/CMS、正式 tag 九包 SBOM 和 GitHub Release 下载复验仍缺；3.17、3.26、阶段 H/M、3.29 与总体继续保持 `未完成`，V2 仍为 `38/44`。

## 批次 341 · Qdrant 启动时序与 Tauri Host 退出清理（2026-07-29）

状态：`未完成`

- run `30436627213` 的 Windows artifact 已证明 package、migration、PowerShell 5.1 manifest hash 和 prompt registry 路径修复生效。Qdrant 服务端没有源码编译或进入 A4N 包；唯一问题是官方预编译 1.18.3 在长时间 release 编译前启动后被 runner 终止。普通 CI 与正式 Release 均已改为 Smoke 紧前面启动，并检查 HTTP `6333` 和 gRPC `6334`。
- 同一 run 的 macOS `.app` 已成功启动 Host 与八个 sidecar；失败只剩 Host 收到 `SIGTERM` 后一个 managed sidecar 存活。`kill_on_drop`/Drop 只排队发送 supervisor 命令，不能保证进程退出前执行。AppState 现从唯一 managed-sidecar owner 原子取出进程并并发等待现有 graceful→force stop；Tauri 只把 Unix `SIGTERM` 转为正常退出，并在 `RunEvent::Exit` 调用该 owner，不在 Smoke 增加强杀兜底。
- run `30442639338` 的三个早期失败都是 workflow 变化未同步架构文档，既有门禁按设计失败。本批同步 V2、进度记录、README 与工作包；平台 lifecycle 仍由下一轮真实 Windows/macOS runner 证明。

本地只执行格式、目标 crate check、文档/发布静态门禁和 diff 检查，不重复跑整仓测试。正式签名、公证、受保护 environment、tag、最终 CMS/manifest 与 GitHub Release 下载证据仍缺，因此工作包 10、3.17、3.26、阶段 H/M 和总体保持未完成。

## 批次 342 · macOS lifecycle 通过与 Windows 同 step Qdrant（2026-07-29）

状态：`未完成`

- run `30444186712` 已以 35/36 结束。macOS job `90550391253` 成功完成 unsigned `.app`、Qdrant、Host + 八 sidecar、日志、非 ASCII 路径和 Host 退出清理；artifact `8722886896` 为 65,882,736 bytes，SHA-256 `6914364f5c420747ae2d964803c2a264907ccc2e1c125f67d5ff0b8d6c860332`。批次 341 的 managed-sidecar shutdown 已取得真实 arm64 runner 证据。
- Windows job `90550391318` 的构建和 Qdrant 双端口就绪均成功；artifact `8722575532` 的 Qdrant 日志先记录 `gRPC listening on 6334`，独立下一 step 的 RAG 随后返回 Winsock `10061`。因此普通 CI 与正式 Windows Release 直接在同一 PowerShell step 内启动 Qdrant 并运行既有 lifecycle smoke，消除跨 step 临时进程依赖。
- 正式 Linux Release 也已把 Qdrant 从长构建前移到构建后、Smoke 前，并显式检查 `6333/6334`。现有 release checker 锁定 Linux/macOS 顺序和 Windows 同 step 合同；YAML、Bash、checker、目标 diff 通过，没有新增脚本、依赖或强杀兜底。

Windows 同 step 修复仍待下一真实 runner。正式 Apple/Azure identity、受保护 `release` environment、`v0.1.0` tag、签名、公证、最终 manifest/CMS、正式 tag 九包 SBOM 和 GitHub Release 下载复验仍缺；工作包 10、3.17、3.26、阶段 H/M、3.29 与总体保持 `未完成` / `38/44`。

## 批次 343 · Windows 受控日志落盘竞态（2026-07-29）

状态：`未完成`

- run `30450022988` 的 Windows job `90569594589` 已证明批次 342 的同 step Qdrant 修复有效：官方预编译 v1.18.3 的 HTTP `6333`、gRPC `6334` 均就绪，Prompt、LLM、Embedding、RAG、Image、Video、Import、Export 八个 package sidecar 全部健康并生成有效 PID，RAG 不再返回 Winsock `10061`。
- 唯一失败是 sidecar health 后没有非空受控日志；artifact `8724923326` 在 `finally` 清理阶段包含同一批日志。本批先在唯一 `smoke-windows-release-package.ps1` 中复用 100ms/300 次有界等待，后续 run `30455406608` 证明该等待仍会完整耗尽，初步时序判断由批次 344 的显式 flush 修复取代。

run `30450022988` 的 macOS job 后续成功，最终证据见批次 344。正式 Apple/Azure identity、受保护 `release` environment、`v0.1.0` tag、签名、公证、最终 manifest/CMS、正式 tag 九包 SBOM 和 GitHub Release 下载复验仍缺；工作包 10、3.17、3.26、阶段 H/M、3.29 与总体保持 `未完成` / `38/44`。

## 批次 344 · Windows 活跃日志显式 flush（2026-07-29）

状态：`未完成`

- run `30450022988` 最终为 35/36；macOS job `90569594566` 成功，artifact `8725461727` 为 65,882,744 bytes，SHA-256 `5042a29da3f24abf08d5e66d4e2f71afd9da3ae091673e60247389da09c60a4f`。unsigned macOS lifecycle 再次取得真实 runner 证据。
- run `30455406608` 的 Windows job `90587590559` 已完成真实 archive 并保持 Qdrant、八个 sidecar health/PID 正常，但 30 秒日志轮询完整耗尽。artifact `8727595947` 为 69,438,948 bytes，SHA-256 `c2e6afe7866e75dc639711ab5ee5579fae546fa2a2dc2fdeb3e61d3824ca0ba9`；进程清理后同一目录立即出现非空日志，定位为 Windows 活跃 rolling file 未显式 flush。
- 唯一 `a4n-sidecar-bootstrap::logging` 复用现有 `tracing-appender 0.2.5` 非丢失 worker，每批消息后 flush 底层文件，`WorkerGuard` 保持到进程结束；没有新增依赖、平台特例或第二日志合同。单一回归测试证明进程存活时文件已可见，结果 1/1，目标 fmt/diff 通过。

Context7 月度额度已耗尽，库行为由当前锁定 crate 源码核对。修复仍待下一 Windows runner；正式 Apple/Azure identity、受保护 `release` environment、`v0.1.0` tag、签名、公证、最终 manifest/CMS、正式 tag 九包 SBOM 和 GitHub Release 下载复验仍缺；工作包 10、3.17、3.26、阶段 H/M、3.29 与总体保持 `未完成` / `38/44`。

## 批次 345 · Windows 活跃日志内容观测与 Generation 首次重连（2026-07-30）

状态：`未完成`

- run `30462394301` 对提交 `11fdb8b` 最终完成 36 个 job：34 成功、2 失败。macOS arm64 job `90611527922` 已完成 unsigned `.app` lifecycle，artifact `8731213620` 为 66,143,566 bytes；Windows job `90611528087` 的 archive、migration、manifest、prompt registry、Qdrant 双端口和八个 sidecar health/PID 均通过，仍在 `FileInfo.Length` 日志断言失败。artifact `8730424257` 为 69,671,571 bytes，SHA-256 `fd6cf1378954f9fe10f5af434ee9c9a78858db519d1b88c6c918aa56f4458dd3`。
- 同一 run 的 Generation job `90611527856` 只有真实取消预算失败：样本 `[25,26,26,26,26,27,27,27,28,29,31,33,56,65,69,70,143,220,550,560]` 的 p95 为 550ms。生产 Web transport 的首次 SSE 重连原为 `500ms + 20% jitter`，与 550/560ms 离群完全吻合；提交 `a59f72e` 只把初始退避改为 100ms，保留指数退避、15 秒上限和 500ms 测试预算，前端定向测试 13/13 通过。
- 同一提交把既有 demo sidecar 真实进程测试从“停止后读日志”收紧为“进程存活时读取 request/trace 持久日志”，并加入三平台原生 process matrix。新 run `30470664612` 的 Windows job `90639731767` 已在 4 秒内通过该步骤，证明当前 `a4n-sidecar-bootstrap` rolling 日志内容在原生 Windows 活跃进程中可读；批次 344 把 portable 失败继续归因于日志 owner 的判断由此纠正。
- 唯一 Windows portable smoke 继续保留 100ms/300 次上限和非空日志要求，只把 `Get-ChildItem | Where-Object Length -gt 0` 改为读取 `*.log.*` daily rolling 文件首行。该变更检查实际日志内容，不接受空文件、不增加等待、依赖、第二日志 owner 或平台 fallback，仍待当前提交的完整 portable runner 复验。
- 2026-07-30 复核远端仍为 0 个 environment、0 个 Actions secret、0 个 variable、0 个 tag、0 个 GitHub Release。正式 Apple/Azure identity、受保护 `release` environment、签名、公证、最终 manifest/CMS、正式 tag 九包 SBOM 和公开 Release 下载复验仍缺；3.17、3.26、阶段 H/M、3.29 与总体保持 `未完成`，V2 仍为 `38/44`、还差 6 条。

## 批次 346 · 普通三平台收口与 hosted runner 基线迁移（2026-07-30）

状态：`未完成`

- Module Checks run `30471463443` 对提交 `9506698` 的普通平台证据已全部通过。Linux job `90642476425` 输出 `host + 8 sidecars, logs, stale PID, cleanup, non-ASCII path`，artifact `8732929908` 为 67,105,080 bytes、SHA-256 `acd378ff616e658f2e7ab20dc55a7f289886508e9f07563eea2bdd8ea5c55946`；Windows job `90642476386` 输出同一生命周期断言，artifact `8733932063` 为 69,668,220 bytes、SHA-256 `901b1bcd5239382fcbaeaca584ba2845161391da23c690f13280a7229477611c`；macOS job `90642476466` 完成 unsigned `.app` Host + 八 sidecar、包内 ONNX Runtime、日志、清理和非 ASCII 路径，artifact `8734696884` 为 66,143,234 bytes、SHA-256 `852846c1102296e85d20d274df8b9fe60173bd3e06b9c2d2a0074dd7df23bc5e`。Windows portable 的日志首行内容读取已在真实 runner 通过。
- Generation job `90642476555` 通过真实 Chromium、Tauri process、1 MiB 十次 SSE 重连和 Tauri Channel；Web 取消样本为 `[25,26,26,26,26,27,27,27,27,27,27,27,27,27,27,28,28,29,29,29]`，p95 从上一轮 550ms 降到 29ms，500ms 预算未放宽。
- run attempt 1 的唯一失败是 build-feedback：artifact `8732250538` 在 AMD EPYC 9V74 / 4 CPU 上记录八包中位数 `755 / 614 / 707 / 805 / 631 / 638 / 5570 / 34837 ms`，Tauri 仅比旧 EPYC 7763 基线的 `25%` budget 高 84ms。只重跑该 job 后，attempt 2 artifact `8747368956` 在同一 CPU 型号、同一提交、同一 `package-rebuild-v1` 方法和完全一致的依赖图上记录 `789 / 642 / 768 / 888 / 681 / 700 / 5728 / 36311 ms`，证明 hosted runner 硬件/负载整体迁移而非 A4N 代码回退。
- `build-feedback-baseline.tsv` 因此按第一次完整 EPYC 9V74 artifact 整体刷新八包数值，继续使用单一 baseline、三次中位数和统一 `25%` ceiling，不增加硬件分支或放宽百分比。提交 `688ac0908ff6796b3347b5098b85be64741e2161` 的后续 run `30512222556` 中，job `90774535660` 已在 Intel Xeon Platinum 8573C / 4 CPU、Rust/Cargo 1.95.0 和同一 `package-rebuild-v1` 方法下通过：workspace cold 为 `209216ms`，八包中位数为 `704 / 556 / 658 / 777 / 585 / 606 / 5404 / 30218 ms`，全部低于 `944 / 768 / 884 / 1007 / 789 / 798 / 6963 / 43547 ms` budget。artifact `8747756421` 为 1,890,529 bytes，SHA-256 `1402a61adf3b11a64b4e7db90919a52277846b32d84abafa7f222a2817c37990`；跨 CPU 仍通过，证明无需建立硬件专用 baseline。
- 远端再次核对仍为 0 个 environment、0 个 Actions secret、0 个 variable、0 个 tag、0 个 GitHub Release。普通三平台 CI 已收口，但正式 Apple/Azure identity、受保护 `release` environment、签名、公证、最终 manifest/CMS、正式 tag 九包 SBOM 和公开 Release 下载复验仍不存在；3.17、3.26、阶段 H/M、3.29 与总体保持 `未完成`，V2 仍为 `38/44`、还差 6 条。

## 批次 347 · LLM manager drop 停止完成屏障（2026-07-30）

状态：`未完成`

- Module Checks run `30512222556` 已通过 build-feedback、Generation 及 Windows/Linux/macOS 三平台 smoke。Linux artifact `8748127282` 为 67,104,959 bytes、SHA-256 `2ce02822c2cbc5219e5db73afba71cbfbd8eea6b850bc2e20af55f7df1250604`；Windows artifact `8748620213` 为 69,668,426 bytes、SHA-256 `aebc3bdbabc89d19b94650c7958e976385e3e9ee527dce40e7fd6b027e356ec0`；macOS artifact `8748655062` 为 66,143,219 bytes、SHA-256 `e8456716700752e392370a46726232f561b72127b1fbb424467faed026b5ec90`。三项日志分别确认 Host + 八 sidecar、日志、清理、非 ASCII 路径，以及 portable stale PID/包内 ONNX Runtime 合同。
- LLM process job `90774535663` 的 11 项中 10 项通过，唯一失败是 `dropping_manager_owned_handle_kills_gateway_and_releases_port`：handle `Drop` 发送 force-stop 后，测试只等 health 请求失败就立即绑定同一端口，撞上 supervisor 尚在执行 `child.kill().await` 的短窗口。Tokio 1.52.1 当前锁定源码确认 `Child::kill()` 会发送 kill 后等待 child exit；生产 `Drop` 和 AppState 显式 shutdown owner 无需改变。
- 既有共享 `wait_until_unavailable` 现在要求 health unavailable 且 manager status 已为 `Stopped` 才返回，使用真实 supervisor 状态作为完成屏障，不增加固定 sleep、重试 fallback、生产平台分支或第二停止 owner。目标 rustfmt/diff 通过；该失败用例单次 1/1、连续 20/20 通过。提交 `90bae35` 后，run `30516382423` 的 job `90787088605` 明确记录原失败用例 `ok`，完整 process target 为 11/11。
- 正式 Apple/Azure identity、受保护 `release` environment、`v*` tag、签名、公证、最终 manifest/CMS、正式 tag 九包 SBOM 和公开 Release 下载复验仍缺；3.17、3.26、阶段 H/M、3.29 与总体保持 `未完成`，V2 仍为 `38/44`、还差 6 条。

## 批次 348 · 单人维护 release environment 落地（2026-07-30）

状态：`未完成`

- 用户确认采用单人维护审批。GitHub `release` environment 已创建，required reviewer 为 `GTJasonMK`（user id `149306821`），`prevent_self_review=false`；deployment branch policy 使用 custom policy，唯一规则为 tag `v*`。正式 workflow 现有两个签名 job 的 `environment: release` 引用与远端设置一致。
- 只读复核确认 environment secrets 与 variables 均为 0，tag 与 GitHub Release 仍未创建。本批不创建无凭证 tag、不触发签名 workflow，也不把 environment 配置冒充 Apple Developer ID、notarization 或 Azure Artifact Signing 证据。
- run `30516382423` 检查时为 `in_progress`：36 个 job 中 32 个成功，LLM process 已 11/11；Windows、Linux、macOS 和 Generation 四个长任务仍运行，当前没有已结束失败 job。

受保护 environment 缺口已经关闭，但正式 Apple/Azure identity、environment secrets/variables、`v0.1.0` tag、签名、公证、最终 manifest/CMS、正式九包 SBOM 和公开 Release 下载复验仍缺；3.17、3.26、阶段 H/M、3.29 与总体保持 `未完成`，V2 仍为 `38/44`、还差 6 条。

## 批次 349 · 未签名三平台可执行包范围收口（2026-07-30）

状态：`未完成`

- 用户明确最终交付只要求可构建、可启动的三平台可执行包，不需要 Apple/Azure 签名、macOS 公证或公开 GitHub Release。run `30471463443` 与 `30512222556` 已在 Windows、Linux、macOS 真实 runner 通过完整 package/bundle lifecycle，因此 3.17 与阶段 H 改为 `完成`。
- 删除未配置凭证且不再属于交付范围的 `.github/workflows/release.yml` 与 `scripts/checks/check-release-signing-workflow.sh`，并从普通 CI 移除相应静态调用。保留唯一 `module-checks.yml`、`deploy.sh`、Tauri bundle、SBOM 和 manifest owner，不增加替代 release workflow。
- 普通 macOS job 在 `.app` 真实 smoke 后生成 `A4N-<version>-macos-arm64-unsigned.zip`、Host + 八 sidecar + ONNX Runtime manifest 和 SHA-256，并解包复验；GitHub artifact 不再直接上传松散 `.app`。

当前只待本批唯一远端 run 证明新增 macOS 归档步骤；通过后即可关闭 3.26、阶段 M、3.29、整体状态和工作包 10。V2 暂为 `40/44`、还差 4 条。

## 批次 351 · 三平台可执行包最终收口（2026-07-30）

状态：`完成`

最终 Module Checks run `30522623690` 对提交 `f5dcfe6` 的 36 个 job 全部成功。Linux artifact `8752230805` 为 67,105,325 bytes，Windows artifact `8752693959` 为 69,666,559 bytes；两者均已通过 migration、manifest/hash、Host + 八 sidecar、日志、stale PID、清理和非 ASCII 路径 smoke。

macOS arm64 job `90806231469` 成功构建并启动 unsigned `A4N.app`，artifact `8753955808` 为 63,566,561 bytes，上传 ZIP digest 为 `b397d5a2e7b7c9ee1c2a26363db46a1c1a02f947432dbd1cd1b56f0aea7b9c4f`。下载复核确认 `A4N-0.1.0-macos-arm64-unsigned.zip` 的 SHA-256 为 `f45db0e506eee9f4e606f9f0544e9c222e5e566ee5acac29f99da08ac3b6de45`，manifest 记录 `A4N.app/Contents/MacOS/a4n-tauri`、八个 sidecar 和 `Contents/Resources/libonnxruntime.dylib`；校验与解包复验均通过。

用户要求的三平台可构建、可启动可执行文件已经满足；签名、公证和公开 GitHub Release 不在范围。3.26、阶段 M、3.29、最终整体状态和工作包 10 全部关闭，V2 为 `44/44`。
