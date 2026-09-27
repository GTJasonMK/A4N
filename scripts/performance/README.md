# 性能审查与优化台账

本台账随基准和源码提交，后续优化先核对 ID 与已有测量，避免重复处理。

日期：2026-09-27。基线：`88ee18431`，工作树：`work/debug`。用户授权直接修改 `debug`；开始时的 8 个前端页面 WIP 保持原样。原始 diff 和测量输出保存在 `data/performance-audit-20260927/`。

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

较小输入同样保留记录：OpenAI 1/32/256 事件分别下降 6.66%/6.74%/26.08%，Anthropic 分别下降 7.45%/10.30%/35.55%。1000 字符、995 次显示更新为 27.4 → 20.1 ms（26.64%）。这些没有达到 30% 的小输入结果不作为主要验收收益，不把压力批次的提升外推到单事件请求。

Intl 的四组对照（每组 2000 次）：中文数字 75.7 → 2.3 ms、中文相对时间 69.3 → 2.9 ms、英文数字 74.8 → 2.3 ms、英文相对时间 78.3 → 2.7 ms。自定义格式选项和日期时区路径没有改变。

## 审查覆盖与未修改项

| 范围 | 已核对事实与决定 |
| --- | --- |
| 首屏和路由 | `App.tsx` 已按路由和依赖图弹窗懒加载。生产入口约 1,009,944 字节、gzip 310,747 字节；两种语言字典静态加载占较大比例。暂不把无端到端测量的拆包算作优化。 |
| 项目列表 | `ProjectReadWorker::summary_for_projects` 已批量查询项目族、权限、进度和反馈间隔；不重新引入逐项目请求或重复批处理层。 |
| 章节列表 | 现有 `ChapterSummary` 合同包含正文、分析、编辑版本和同步状态。完整读取并非可以直接删除的多余列；要减小分页载荷需连同使用全文的消费者一起验证，未擅自改变合同。 |
| 数据库权限和协作 | 读取使用连接池、绑定参数和批量根项目查询。邀请／成员统计仍可能随大型协作人数增长；暂无代表性数据证明其为当前瓶颈，未为凑指标添加索引或改写权限查询。 |
| 工作流轮询和恢复 | 旧任务进度轮询间隔 500 ms，已有单次进行中保护和结束清理；新生成任务走事件订阅。未通过减少进度频率来换取表面收益。恢复快照的读取需求与公开进度投影分开评估。 |
| HTTP 与媒体 | Worker／service client 已持有并复用 HTTP client；Web 媒体使用 `ServeFile`，Edge 使用流式 body。桌面自定义协议要求字节响应，不能直接套用 Web 的传输方案。 |
| LLM 上游连接 | 网关 `provider_from_config` 仍按调用构造 provider。跨请求连接复用还需区分连接超时、流读取超时和配置差异，并测量真实 TLS 开销；本次没有为未量化的收益引入 provider 缓存。 |
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

以上量化覆盖指定的计算、查询和渲染场景。没有把这些比例宣称为整站 HTTP P95、模型生成时长或整进程常驻内存改善；首屏大包、真实并发容量、真实上游 TLS 和完整 CLI／桌面业务 E2E 仍未做本轮端到端验收。
