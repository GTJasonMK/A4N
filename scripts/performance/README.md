# 性能审查与优化台账

本台账随基准和源码提交，后续优化先核对 ID 与已有测量，避免重复处理。

日期：2026-09-27—2026-09-28。基线：`88ee18431`，工作树：`work/debug`。用户授权直接修改 `debug`；开始时的 8 个前端页面 WIP 保持原样。原始 diff 和测量输出保存在 `data/performance-audit-20260927/`。

## 验收口径

- 每项修改必须先确认调用链、输入规模和重复频率；以同输入、同构建参数的前后对照保留证据。
- 本次采用目标场景耗时减少至少 30%，或进程峰值 RSS 减少至少 20% 的保留标准。短输入不达该比例不隐瞒；明显退化必须复查。
- 局部 CPU、RSS、资源体积与整条 HTTP 请求的耗时分别记录，不把算法基准写成全项目响应时间提升。
- 保持 API、权限、错误、取消、输出内容与排序语义。没有新增业务缓存、预取、后台任务或依赖。
- 测量与正确性检查分开。Rust 测量用优化构建，前端用生产构建和真实 Chromium；不把开发服务器启动当作验收。

## 已完成的优化

| ID | 请求链路、规模与频率 | 根因与最小修复 | 验证状态 |
| --- | --- | --- | --- |
| [PERF-001](../../crates/llm/a4n-llm-http/src/sse.rs) | 模型 HTTP → OpenAI SSE → 生成事件；每个网络数据块；覆盖 1/32/256/2048 个事件 | 逐行 `drain().collect()` 重复搬移尾部并分配临时行；改为借用行切片，批次结束只整理一次尾部 | 2048 事件批次 **6.907 → 1.252 ms，下降 81.87%** |
| [PERF-002](../../crates/llm/a4n-llm-http/src/anthropic.rs) | Anthropic SSE → 生成事件；同上 | 同一逐行搬移问题；取出原缓冲区解析后归还，保持容量与解析顺序 | 2048 事件批次 **7.865 → 1.221 ms，下降 84.48%** |
| [PERF-003](../../crates/rag/a4n-rag-core/src/semantic_chunker/matrix.rs) | 章节资料同步 → 服务端语义分块 → 相似度 → 前缀和 → 动态规划；每次正文索引，1600 句 × 768 维 | 同时保留两个平方级矩阵；相似度矩阵预留零边界，原地转换前缀和。保持原来的浮点运算顺序，不改变评分算法 | 峰值 RSS **48,552 → 28,256 KiB，下降 41.80%** |
| [PERF-004](../../frontend/src/hooks/usePacedStreamText.ts) | 写作台／灵感对话／Agent 消息／进度弹窗；1千/1万/5万字符，每秒最多 120 次显示更新 | `useRef` 的初始化参数被每次渲染重新计算，造成无用的整段 `Array.from`；对已有字符 ref 做惰性初始化，保留原有字符推进和自适应速率算法 | 1万字符、1000 次更新 **117.7 → 17.6 ms（85.05%）**；5万字符 **527.8 → 15.1 ms（97.14%）** |
| [PERF-005](../../frontend/src/i18n/format.ts) | 全站列表、进度、图表 → 国际化格式化；每次渲染每个数字／相对时间 | 每次调用都构造 `Intl` 格式器；复用两种固定语言的不可变格式器，自定义数字选项仍独立处理 | 两种语言、两种格式的 2000 次调用分别下降 **95.82%–96.96%** |
| [PERF-006](../../crates/platform/a4n-db/src/workflow/runs/projections.rs) | 工作流列表／详情 → 公开进度投影；含 0/100/1000 个已完成单元的恢复快照 | 复制完整 `progress_json` 后立即丢弃恢复快照；现在只复制公开字段，并保留快照类型与进度字段校验 | 1,103,968 字节输入 **244.812 → 0.689 µs，下降 99.72%**；这是投影函数耗时 |
| [PERF-007](../../crates/platform/a4n-db/src/project/progress/shared.rs) | 项目列表／摘要 → 漫画、游戏、自由视频的流水线进度；每次读取，12 项目、每条快照 540 KB | 只显示消息却从数据库取回整个恢复快照；SQL 直接提取 JSON 消息值，保留最新记录选择、状态和错误优先级 | 真实 PostgreSQL 查询及投影 **20.237 → 6.309 ms，下降 68.82%** |
| [PERF-008](../../frontend/src/components/ui/MarkdownText.tsx) | Agent／灵感对话 → 历史 Markdown；新回复每次更新，50 条历史、40 次更新 | 未变化的历史文本反复解析、构建和协调 DOM；使用 React 原生 `memo` 隔离这个仅依赖两个字符串属性的纯组件 | **265.6 → 1.9 ms，下降 99.28%**；最终 DOM SHA-256 完全相同 |
| [PERF-009](../../crates/host/a4n-edge/src/lib.rs) | 浏览器冷启动 → Edge → 生产 JS/CSS/HTML；每次空浏览器缓存访问 | Edge 原先发送完整文本；[Vite](../../frontend/vite.config.ts) 在构建时生成 gzip，Edge 原生协商预压缩表示并设置 `Vary: Accept-Encoding`；Tauri 构建跳过额外表示 | 4 Mbps／50 ms／4 倍 CPU 条件下，登录表单可操作时间：1440px **2772.6 → 950.8 ms（65.71%）**，390px **2717.7 → 956.3 ms（64.81%）** |
| [PERF-010](../../frontend/src/api/generationEvents.ts) | 生成页面加载 → generationTasks → 协议校验器；每个浏览器文档首次使用 | 固定 schema 每次在浏览器编译；[合同生成器](../contracts/generate-generation-validators.mjs) 直接生成 ESM 校验器，保留相同 AJV 选项、Unicode helper、所有字段校验和错误 | 4 倍 CPU、冷模块导入并校验首批 3 个真实协议样本：**366.7 → 11.1 ms，下降 96.97%**；2387 组决策及完整错误对象对照一致 |
| [PERF-011](../../crates/platform/a4n-db/src/workflow/runs/queries.rs) | 章节列表／单章摘要 → Chapter Worker／Orchestrator → 工作流与检查点；100 章、每章 3 次历史、每次两个 64 KB 检查点 | 只使用最新运行却读取全部历史并深复制检查点；数据库按原排序直接选择所需章节的最新运行，检查点移交所有权 | 8 并发完整 HTTP 响应 P95 **174.54 → 82.08 ms（52.98%）**；整栈读取后 PSS **1,236,524 → 637,804 KiB（48.42%）** |
| [PERF-012](../../crates/llm/a4n-llm-http/src/errors.rs) | 模型网关／内嵌 provider → HTTP/TLS → SSE；同一超时配置下连续 45 次请求，每次更换模型和测试凭证 | 每次构造 provider 都重建 HTTP Client、证书库和连接；复用原生 Client，保留独立的连接／逐次读取超时，最多保留 16 种配置 | 本机 TLS/SSE 请求中位数 **10.770 → 0.603 ms（94.40%）**，P95 **14.658 → 0.750 ms（94.88%）**；45 次请求由 45 条连接降至 1 条 |
| [PERF-013](../../frontend/src/pages/writing-desk/ChapterSidebar.tsx) | 写作桌每次输入／流式正文更新 → 侧栏；100／1000 章，每章 4000 个 Unicode 码点，25 次更新 | 所有已保存正文重复计数且全部章节行重复渲染；字数随现有章节索引计算，当前章实时计数；[章节行](../../frontend/src/pages/writing-desk/ChapterCard.tsx) 通过原有稳定切章回调和 React `memo` 复用未变化的渲染 | 100 章 **182.4 → 10.1 ms（94.46%）**；1000 章 **1646.5 → 39.7 ms（97.59%）**；这是侧栏 25 次更新的累计 CPU 时间，完整 DOM 一致 |
| [PERF-014](../../crates/embedding/a4n-embedding-fastembed/src/provider.rs) | Embedding Service／动态 provider → FastEmbed → ONNX；已下载的默认中文模型，1／32 条文本、1／4 并发 | 每次 RPC 的 provider 释放后丢弃已初始化的模型；共享同模型／目录的模型资源，额外只保留一种空闲配置，阻塞推理持有模型锁至真正结束 | 单条查询中位数 **428.07 → 34.72 ms（91.89%）**；32 条、4 并发 **2340.93 → 1268.56 ms（45.81%）**；峰值 RSS **599,124 → 229,788 KiB（61.65%）** |
| [PERF-015](../../crates/embedding/a4n-embedding-providers/src/http.rs) | API 嵌入 → HTTPS `/embeddings`；1／32 条文本、384 维、连续 45 次请求 | 固定 60 秒总超时的 HTTP Client 随每次 provider 重建；原生单例 Client 保留连接池，模型／凭证／维度／批量参数仍逐次构造 | 单条中位数 **20.830 → 1.468 ms（92.95%）**；32 条 **30.788 → 13.299 ms（56.80%）**；45 条连接降为 1 条，完整响应一致 |
| [PERF-016](../../crates/host/a4n-edge/src/lib.rs) | Web → Edge → 大型 JSON；真实模板语料组成约 1.1／11 MB 响应，4 Mbps／50 ms | JSON 原先按完整字节交付；启用现有 Tower HTTP 的 gzip，最快压缩等级，只处理符合条件的大型 200 JSON 响应 | 三轮中位数 **2332.8 → 1073.3 ms（53.99%）／22225.3 → 9826.0 ms（55.79%）**；全部解压后字节摘要一致 |

较小输入同样保留记录：OpenAI 1/32/256 事件分别下降 6.66%/6.74%/26.08%，Anthropic 分别下降 7.45%/10.30%/35.55%。1000 字符、995 次显示更新为 27.4 → 20.1 ms（26.64%）。这些没有达到 30% 的小输入结果不作为主要验收收益，不把压力批次的提升外推到单事件请求。

Intl 的四组对照（每组 2000 次）：中文数字 75.7 → 2.3 ms、中文相对时间 69.3 → 2.9 ms、英文数字 74.8 → 2.3 ms、英文相对时间 78.3 → 2.7 ms。自定义格式选项和日期时区路径没有改变。

## 审查覆盖与未修改项

| 范围 | 已核对事实与决定 |
| --- | --- |
| 首屏和路由 | `App.tsx` 已按路由和依赖图弹窗懒加载。PERF-009 已通过真实 Edge 和生产登录页验证压缩交付；同一份入口文件从 1,010,285 原始字节变为 310,789 传输字节。两种语言字典仍静态加载，未宣称这一部分代码拆分已经完成。 |
| 项目列表 | `ProjectReadWorker::summary_for_projects` 已批量查询项目族、权限、进度和反馈间隔；不重新引入逐项目请求或重复批处理层。 |
| 章节列表 | PERF-011 已消除未使用的历史读取及检查点深复制。现有 `ChapterSummary` 合同仍包含正文、分析、编辑版本和同步状态；这些字段的消费者仍然需要验证，未通过返回空正文或删除字段换速度。 |
| 数据库权限和协作 | 读取使用连接池、绑定参数和批量根项目查询。邀请／成员统计仍可能随大型协作人数增长；暂无代表性数据证明其为当前瓶颈，未为凑指标添加索引或改写权限查询。 |
| 工作流轮询和恢复 | 旧任务进度轮询间隔 500 ms，已有单次进行中保护和结束清理；新生成任务走事件订阅。未通过减少进度频率来换取表面收益。恢复快照的读取需求与公开进度投影分开评估。 |
| HTTP 与媒体 | Worker／service client 已持有并复用 HTTP client；Web 媒体使用 `ServeFile`，Edge 使用流式 body。桌面自定义协议要求字节响应，不能直接套用 Web 的传输方案。 |
| LLM 上游连接 | PERF-012 保留每次构造 provider 的配置与凭证隔离，仅延长 HTTP Client 的生命周期。原连接／逐次读取超时、错误、取消与背压检查通过；不缓存模型响应。 |
| 嵌入模型生命周期 | PERF-014／015 已处理本地模型和 API 连接的重复初始化。模型文件删除／下载会使额外持有的资源失效；仍存活的 provider 保留原有拥有权，避免破坏已在使用的模型。没有缓存业务向量结果。 |
| 语义分块 | 已复用向量范数并利用相似矩阵对称性；本次只消除额外矩阵。将全矩阵换成局部带状算法可能改变浮点抵消和相同分数时的切分，未作为无风险替换。 |

## 可复现入口

环境：Intel i9-13900H、Rust 1.95.0、Node 24.18.0、Chromium 150.0.7871.186、PostgreSQL 16。前端最终对照将测试进程及子进程固定到 CPU 18、19，前后交错运行三轮；没有停止其他开发会话。每轮预热至少 250 ms 且至少三次，再取九个样本中位数，最终取三轮中位数。Rust SSE 与 SQL 也进行三轮交错对照，RAG RSS 每次使用独立进程。

逐字显示测试在同一个 React root 上测量，并在采样前结算挂载／重置 effect；控制计时器推进，不把刻意配置的逐字等待时间算成 CPU 改善。中英文、emoji、替换、截断、排空、`wasStreamed` 和主题切换的完整行为轨迹前后相同。

前端（从 `frontend/` 执行；只服务本地测试夹具，不请求真实模型）：

```bash
env -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u http_proxy -u https_proxy -u all_proxy \
  NO_PROXY=localhost,127.0.0.1 node e2e/performance/check.mjs
```

设置 `A4N_PERF_BASELINE_REF=88ee18431` 可将被测的三个前端模块从原提交载入同一个测试夹具，避免切换分支或覆盖工作区；`A4N_PERF_ARTIFACTS` 指定独立结果目录。`A4N_PERF_REUSE_BUILD=1` 可复用已经生成的生产测试包进行交叉复测。用 `taskset -c 18,19` 启动上述命令可复用最终 CPU 亲和性设置；换机器时选择该机器实际可用的 CPU。行为对照覆盖 1440px／390px。

Rust（从当前工作树执行，持有 `work/.debug-build.lock`；首次先用 `--no-run` 完成编译，实际测试单次限制 60 秒）：

```bash
flock -x ../.debug-build.lock env CARGO_TARGET_DIR="$PWD/target" CARGO_INCREMENTAL=0 \
  CARGO_PROFILE_RELEASE_LTO=false timeout 60s cargo test --release --locked --offline \
  -p a4n-llm-http --lib performance_ -- --ignored --nocapture --test-threads=1
flock -x ../.debug-build.lock env CARGO_TARGET_DIR="$PWD/target" CARGO_INCREMENTAL=0 \
  CARGO_PROFILE_RELEASE_LTO=false timeout 60s cargo test --release --locked --offline \
  -p a4n-rag-core --lib performance_semantic_chunking -- --ignored --nocapture --test-threads=1
flock -x ../.debug-build.lock env CARGO_TARGET_DIR="$PWD/target" CARGO_INCREMENTAL=0 \
  CARGO_PROFILE_RELEASE_LTO=false timeout 60s cargo test --release --locked --offline \
  -p a4n-db --lib performance_ -- --ignored --nocapture --test-threads=1
```

最后一条包含数据库基准，必须显式提供隔离的 `A4N_TEST_DATABASE_URL`，不要指向日常开发库。本轮使用单独的 `a4n_perf_test_20260927`；每次基准再通过项目现有 migration test support 建立独立 schema。

Rust 的 RSS 使用独立测试二进制的 `wait4().ru_maxrss`；不把 Cargo 编译器内存计入运行时。RAG 每次输出全部分块字节长度，六次对照完全相同；额外用原提交的真实矩阵源码对比当前实现，45 组配置的所有前缀和与候选分块分数逐位相同。原始和优化后的 Rust 基准二进制保留在本地证据目录，未提交二进制。

最终证据目录为 `data/performance-audit-20260927/`：

- `summary.json`：全部量化结果、保留门槛与被测生产源文件 SHA-256。
- `frontend-comparison.json`、`frontend-{baseline-final,final}/report-round-*.json`：三轮浏览器样本、完整行为轨迹和 DOM 摘要。
- `rust-comparison.json`、`comparison-*-a4n_*.log`：SSE 耗时、RAG RSS 与分块结果。
- `database-comparison.json`、`comparison-*-pipeline-*.log`：实际 PostgreSQL 查询对照。
- `baseline-db-performance.log`、`progress-after-perf.log`、`matrix-equivalence.log`：投影微基准和矩阵逐位对照。

早期试验目录不属于最终验收数据。证据、测试数据库配置和临时类型检查快照均留在忽略目录中，不提交密钥、运行数据或构建产物。

## 最终验证

- 优化构建和定向回归：LLM HTTP 54 项、RAG 29 项、数据库项目进度 12 项、工作流 28 项，共 **123 项通过**。数据库用例使用真实 PostgreSQL，包含最新 ID 选择、项目与 workflow kind 范围、错误优先级、非字符串 JSON 和快照保留。
- 前端 Vitest **432 项通过**；生产 Chromium 对照通过；Host API、generation schema、UTF-8 fixtures 一致性检查通过。
- `cargo fmt --all -- --check`、`git diff --check` 通过；最新当前工作树的完整 `npm run build`（TypeScript + Vite）与 432 项 Vitest 重查均通过。
- 基线已有一条过时测试把空进度 `{}` 判为错误，但生产实现一直返回“没有公开进度”；已保留原行为并修正测试。未知字段、非法快照、缺字段、越界进度仍明确失败。
- 21:01 的整树 `npm run build` 曾被并行视频页面 WIP 的五处 `Promise<boolean>`／`Promise<void>` 不匹配阻塞，失败日志保留在 `frontend-build-final.log`。本任务没有改动那些视频文件；先验证了已提交基线 `3a05cb9f` 加本次三个前端文件的临时快照。并行改动完善后，21:25 的整树重查通过，见 `frontend-build-current.log`、`frontend-tests-current.log`。

## 第二批：真实首屏与校验器初始化

基于 `0c83a6728` 继续，原始证据位于 `data/performance-audit-20260927/b02/`。

- `startup-comparison/report.json`：旧／新 Edge 运行同一份生产资源，9 轮交错冷加载；后端仅提供固定的空 OAuth 提供者列表，不替换页面或静态文件服务。两种宽度的页面文本摘要一致，用户名／密码键盘流程通过，截图保留在同目录。
- 原入口实际传输 1,010,285 → 310,789 字节（69.24%）。构建生成 163 个压缩表示，共约 1.05 MB；没有运行时压缩 CPU 开销。`TAURI_ENV_PLATFORM=linux` 的前端构建中压缩副本数为零，沿用 Tauri 已有的构建环境标识。
- 实际 HTTP 检查覆盖原始内容与解压内容一致、MIME、HEAD 长度、`gzip;q=0`、identity Range、304、SPA 回退；API 响应不进入静态资源层。`Vary` 同时保留 CORS 的原有值。
- `schema-load/report.json`：原始源码与预编译实现使用同样的 Vite 库构建方式、同一个浏览器与同一批协议数据对照；计入模块下载、解析、初始化和首批事件校验。另在真实 Vite 开发服务器中导入该模块验证，开发／生产两种入口均通过。
- 校验器没有改变持续运行时的校验算法：3000 次热事件校验为 73.8 → 77.1 ms，不计作性能收益。预编译模块原始库构建体积 230,109 → 270,530 字节；gzip 后 52,722 → 18,652 字节。实际应用生成任务 chunk 的 gzip 大小约 49.5 KB → 18.8 KB。明确保留体积／首用成本的取舍，未通过删校验换速度。
- 曾试验关闭引用内联以减少生成代码，但完整错误对象的 `schemaPath` 发生变化，已拒绝该方案并恢复原选项；失败证据为 `rejected-inline-refs.log`。最终 2387 组包含嵌套字段缺失、类型、Unicode、未知字段和安全整数边界的对照完全一致。
- 合同生成的 `--write/--check` 同时写入／核对 `generationValidators.js` 与 `.d.ts`。使用 AJV 自身的 Unicode helper 生成 ESM，浏览器不再依赖运行时编译或 CommonJS 兼容补丁。
- 本批 Edge 5 项、前端 432 项、合同生成／fixture、完整前端构建、Tauri 前端构建、格式检查通过。中途他方 Canvas WIP 引起的类型失败保留在 `frontend-build-final.log`，后续当前工作树重查通过，见 `frontend-build-recheck.log`。

复现入口（在 `frontend/`，同前文清除测试进程的 HTTP 代理环境）：

```bash
A4N_PERF_EDGE_BEFORE=/path/to/baseline-edge \
A4N_PERF_EDGE_AFTER=/path/to/current-edge \
A4N_PERF_STARTUP_DIST=/path/to/production-dist \
  node e2e/performance/startup.mjs
node e2e/performance/schema-load.mjs
```

Edge 基线二进制由改码前的当前源码构建并保留；不能用同一个已优化二进制充当旧版本。上述路径必须在工作树或 `work/` 内；生产资源通过 `npm run build -- --outDir ... --manifest` 生成，已有部署脚本的 `--outDir` 调用方式保持有效。

## 完整目标仍在进行

以上不是整站所有性能目标已经完成的声明。首屏交付、核心计算、真实 Host 进程树、1/4/8 并发读请求、LLM TLS/SSE、CLI 写作与恢复链路、千章侧栏、本地及 API 嵌入和大型 JSON 交付已经验证。完整正文列表的载荷仍随章节数量增长，下一步需区分目录所需字段与按需读取的单章内容，不能用压缩后的相对提升宣称所有大项目已即时加载。

第三批已建立真实运行栈：Server、Control、Edge、CLI、迁移程序、30 个状态型 Worker 和 9 个能力服务，共 44 个冻结程序。使用独立 `a4n_perf_runtime_20260927` 数据库、数据目录和端口；实际进程树为 42 个进程、872 个线程。所有构建、配置与原始样本在 `data/performance-audit-20260927/b03/`。

## 第三、四批的验收证据

- **章节读取：** `b03/chapter-controlled-comparison.json` 与 `chapter-controlled-{1,2,3}-{before,after}/report.json` 是最终三轮交错样本。100 章、300 次运行、600 个检查点（JSON 合计 38,428,800 字节），响应 1,160,181 字节；新查询只返回 100 次最新运行及其 200 个检查点。所有章节响应的完整 canonical JSON 摘要均为 `1c26fe8ac1d1cc89bab5a243f13843ec5b4aec88cd27329bdd7b5e0fc5950473`。
- P95@1/@4/@8 分别为 **74.97 → 65.88 ms（12.12%）／125.60 → 67.25 ms（46.46%）／174.54 → 82.08 ms（52.98%）**。单并发时延没有达到 30%，不计作主要验收收益。Chapter Worker 读取后 PSS 中位数 772,316 → 171,726 KiB；整栈 PSS 指上述 42 个进程的同阶段合计，**不是峰值或空闲内存**。空闲 PSS 392,473 → 385,450 KiB，仅变化约 1.8%。未修改读口的少量毫秒级波动不计收益。
- 原始 35 个 package 的 feature 选择用于重建两个受影响程序，发布参数为 release、LTO=false；`chapter-matched-feature-differences.json` 的依赖 feature／profile 差异为空。`frozen-before` 与 `frozen-chapter-matched` 的其余 42 个程序保持逐字节相同，SHA-256 见对应 `binaries-*.json`。早期 `chapter-comparison.json` 存在构建选择和负载干扰，保留供复核，不作为最终时延结论。
- **模型传输：** `b03/llm-transport-comparison.json` 与 `llm-tls-final-{1,2,3}-{before,after}/report.json` 使用真实 Gateway、完整系统 CA 集加测试根证书、HTTPS 和 chunked SSE。前后依赖 feature／profile 一致，270 个探测响应的完整 JSON 摘要相同。每次更换模型名与测试 API key，服务端逐次核对，确认连接复用没有混用请求配置。首次五次作为预热，报告重复调用的中位数与 P95；实际模型推理耗时没有计入收益。
- 传输测试同时覆盖：1 秒读取超时下持续输出 1.8 秒成功、空闲 1.2 秒失败、改为 2 秒超时后成功、失败后继续请求成功。LLM HTTP **55** 项、Gateway **19** 项、独立进程 **11** 项通过；其中包含取消释放、背压、超时恢复、凭证转发边界。Client 数量上限与配置唯一性也有可执行检查。
- **写作侧栏：** `b04/sidebar-comparison.json` 使用生产 React／真实 Chromium 153.0.8010.52，进程及子进程固定 CPU 18、19，前后交错三轮，每轮预热后取五个样本中位数。两个版本使用同一夹具，只从 `e915d0373` 替换旧版 `ChapterSidebar` 和 `ChapterCard` 源码。蓝图／角色 API 返回固定空数据，章节行、计数、样式、React 和 DOM 均为生产实现。100／1000 章最后 DOM 摘要相同。
- 侧栏检查包含中英文、1440／390 宽度、emoji／组合字符／CRLF、空正文、新章本地草稿、服务器正文变更、过滤、键盘 Enter 选章，以及行数据不变时替换回调。原有切章回调沿用 `safeSelectChapter → confirmCurrentEdit`，没有绕过未保存编辑确认。生产构建、合同检查与 **432 项 Vitest** 通过；截图和每轮报告在 `b04/sidebar-{before,after}/`。
- **组合回归：** `b03/frozen-b03-final` 只替换 Control、Chapter Content Worker、LLM Gateway 三个程序。最终 `runtime-cli-b03-final` 的 CLI 写作 **13**、场景同步／恢复 **7**、模型失败／取消／预算 **8** 项通过；数据库最新记录选择 **2**、章节纯规则 **2** 项通过。Worker isolation 门禁在预编译后通过；此前的 60 秒编译超时保留为未通过记录，未冒充测试成功。
- 本批汇总为 `b03-b04-summary.json`，包含三个对照结果、生产源码指纹与组合 CLI 回归结果。夹具正向检查和历史数量不匹配时的拒绝检查均通过，见 `b03/fixture-validation.json`；失败检查在性能采样前退出。

复现入口：先显式设置 `A4N_PERF_DATABASE_URL`（本机 PostgreSQL、数据库名以 `a4n_perf_` 开头），并准备 ONNX Runtime 与 Qdrant。保持本工作树内的程序、数据和结果路径。脚本记录实际运行／检查点数量与 JSON 字节数；已有数据与 `--chapter-history` 不符时明确失败，避免把历史累积后的数据库误当成空白基线。

```bash
python3 scripts/performance/runtime-baseline.py \
  --bin-dir data/performance-audit-20260927/b03/frozen-b03-final \
  --state-dir data/performance-audit-20260927/b03/state \
  --report-dir data/performance-audit-20260927/b03/new-runtime-report \
  --chapter-history 3 --qdrant-url http://127.0.0.1:48973
python3 scripts/performance/llm-transport.py \
  --gateway data/performance-audit-20260927/b03/llm-gateway-production-after \
  --report-dir data/performance-audit-20260927/b03/new-tls-report
```

侧栏从 `frontend/` 执行前文相同的代理清理命令并加上 `A4N_PERF_CASE=sidebar`；用 `A4N_PERF_BASELINE_REF=e915d0373` 载入旧版两组件。先构建两份夹具，再设置 `A4N_PERF_REUSE_BUILD=1` 交错复测，输出目录由 `A4N_PERF_ARTIFACTS` 指定。

没有保留的资源调优：默认启动空闲 PSS 约 375 MiB，普通读取后约 490 MiB。`TOKIO_WORKER_THREADS=2/4` 和 `MALLOC_ARENA_MAX=2` 没有稳定达到本任务的内存／时延门槛；2 线程还使章节 P95@8 从约 38–43 ms 增至 58–61 ms。未修改产品线程数、allocator 或 `ServiceManager` 的 `env_clear()` 安全边界。

环境中断后，既有 PostgreSQL 容器已经恢复；Qdrant 使用本任务独立的 `b03/qdrant` 数据和 48972／48973 端口。基准脚本支持 `--qdrant-url`。`--cli-checks` 会在读取测量结束后运行已有三套 CLI 测试，应使用独立的一次性数据库；它创建的业务夹具不能混入后续读性能对照。首次 CLI 场景测试因导入默认 384 维与后续 8 维模型不一致失败，原始错误和 RAG 保护标记保留；修正测试配置后使用新隔离库验证，未改产品校验。

## 第五、六批：嵌入资源生命周期

- 本地模型为应用默认推荐的 `BGESmallZHV15`（`Xenova/bge-small-zh-v1.5`，512 维，约 90.9 MiB）。使用现有显式下载 API 获取到本任务的 `b05/state/models`，模型权重 SHA-256 为 `69a0b846f4f116b5e6aabf9546ea6754d02264f3211a13a1bd69b31b8040749a`。所有对照使用相同权重、输入和 release／依赖 feature 配置；模型下载时间不计入推理收益。
- `b05/comparison.json` 是三轮交错、真实 ONNX 模型的完整 RPC 对照。1 条文本在 1／4 并发下的中位耗时分别 **428.07 → 34.72 ms／1257.28 → 153.06 ms**；32 条分别 **678.17 → 344.87 ms／2340.93 → 1268.56 ms**。所有身份字段与每个向量元素完全相同，包括额外的批量大小 1／8／32 对照。
- 单个 embedding 进程的负载后 RSS 为 **424,632 → 229,788 KiB**，`VmHWM` 峰值为 **599,124 → 229,788 KiB**。结束时线程数从 44 变为 62：已加载 ONNX 会话的工作线程随模型保留，不能宣称线程数也下降。该实现只额外保留一个配置，已有长寿命 provider 继续持有自己的模型；混合模型持续切换时可能重新加载，这是有界内存策略的已知取舍。
- `b06/native-idle-{before,after}` 在负载结束并稳定后取样，两版空闲 CPU 均为 0%；最终组合程序的向量仍与原版逐值相同。扩展传输夹具后，原有 LLM 模式的四项超时／恢复检查再次通过。汇总及生产源文件指纹见 `b05-b06-summary.json`。
- 初始化和推理继续使用原来的阻塞任务及错误映射。模型锁随阻塞任务持有，调用方断开不会提前释放锁、制造第二份正在初始化的模型；推理错误仍归还模型，线程 panic 不留下半完成模型。每次使用前仍检查下载文件，未引入隐式下载。
- 文件回归先成功推理，再经真实下载／删除 API 变更生命周期。测试临时原子替换本任务的 ONNX 文件为损坏文件，确认返回初始化错误而不是旧模型结果；随后恢复原始权重并确认完整向量一致。还验证未知模型／维度／批量参数拒绝、请求在途时断开、后续请求恢复。恢复操作位于 `finally`，原始文件摘要再次核对。回归在原版和修改版均通过。
- `b06/http-comparison.json` 使用真实 Embedding Service、完整系统证书集和 HTTPS 夹具；1／32 条文本各三轮、每轮 45 个请求，逐次更换模型和凭证，540 个完整响应按请求序号一致。500 错误、维度不匹配和错误后的正常请求保持原行为，固定 60 秒总超时未改变。此项测量的是 API 传输及本地处理，未包含外部模型推理时间。
- FastEmbed／Embedding Service／进程回归 **22 项通过**，embedding providers **4 项通过**。首次测试链接缺少 `ORT_LIB_PATH` 的失败保留；配置项目使用的 ONNX Runtime 1.24.2 动态链接路径后编译和测试通过。最终两个分支使用相同依赖 feature／profile，差异文件为空。
- `b06/frozen-final` 仅在上一批冻结程序基础上替换 Embedding Service。组合后的 `b06/cli-final` 写作 13、场景恢复 7、模型失败与取消 8 项通过。`b06/scale-1000-recheck` 验证 1000 章顺序和完整正文，响应 **11,601,686 字节**，单客户端 P50／P95 **86.73／103.30 ms**，读取后整栈 PSS **606,041 KiB**。这是本机传输结果，不是低带宽 Web 结果。

复现入口：

```bash
python3 scripts/performance/embedding-lifecycle.py \
  --service data/performance-audit-20260927/b06/embedding-service-after \
  --state-dir data/performance-audit-20260927/b05/state \
  --report-dir data/performance-audit-20260927/b05/new-report --verify-files
python3 scripts/performance/llm-transport.py \
  --gateway data/performance-audit-20260927/b06/embedding-service-after \
  --kind embedding --texts 32 \
  --report-dir data/performance-audit-20260927/b06/new-http-report
```

首次准备模型时显式加 `--download`。文件生命周期测试只允许使用本任务独立的模型缓存；结果目录必须是新的。`runtime-baseline.py --chapter-count 1000 --concurrency 1` 可复现大章节读取，其余夹具和隔离数据库要求不变。

## 第七批：Web JSON 交付

- `b07/comparison/report.json` 与 `b07/summary.json` 使用真实旧／新 Edge、同一 Control 夹具、实际生产登录页和 Chromium。语料来自项目已有 Markdown 提示词，整体超过 gzip 窗口，避免重复一个短句来制造压缩率。每组载荷在 4 Mbps 下载、50 ms 延迟下前后交错三轮。
- 1,131,054 字节 JSON 的传输字节中位数降到 495,776；11,027,654 字节降到 4,831,316。两组耗时分别下降 53.99%／55.79%。这项收益来自减少传输字节，动态压缩本身使用 CPU；没有缓存用户数据、改变字段或降低刷新频率。
- 只压缩大于 1024 字节的成功 JSON，保留原有 `Vary`；ETag 和 `Cache-Control: no-transform` 响应保持原样。原生中间件也拒绝重复压缩及 Range 响应。真实 HTTP 检查覆盖身份表示、gzip、`gzip;q=0`、HEAD、Range、已编码响应、小 JSON、ETag、no-transform 和授权头转发。
- SSE 没有进入 JSON 压缩层；测试首段约 1.2 ms 到达、尾段约 801.5 ms 到达。静态资源沿用 PERF-009 的构建期 gzip，不叠加动态压缩。
- Edge 原有 5 项测试、实际浏览器及协议检查通过。`b07/cli-final` 使用只替换 Edge 的最终冻结程序完成 28 项 CLI 业务回归。真实千章 API 在身份／gzip 协商下的完整 JSON 相同；这批合成正文高度重复，真实 API 的压缩率只作协议验证，不作为主要收益依据。
- 本次仅为已有 `tower-http` 启用 `compression-gzip`，锁文件新增其 3 个编解码依赖，既有依赖版本未更新。没有自建压缩协议或业务缓存。
- 浏览器基准首次因测试临时目录过长触及 Unix socket 长度限制而启动失败，见 `b07/startup-diagnostic/driver-verbose.log`；缩短本任务临时目录前缀后通过，没有修改用户浏览器设置或关闭其他会话进程。

复现：从 `frontend/` 执行前文的代理环境清理，再运行：

```bash
A4N_PERF_EDGE_AFTER=/path/to/new-edge \
A4N_PERF_ARTIFACTS=/path/inside/worktree/data/api-transfer \
  node e2e/performance/api-transfer.mjs
```

`A4N_PERF_EDGE_BEFORE` 可指定旧版 Edge；默认使用本任务 `b06/frozen-final/a4n-edge`。测试读取同一份已冻结的生产前端资源，并逐次核对解压结果摘要。
