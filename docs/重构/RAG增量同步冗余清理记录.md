# RAG 增量同步 BFS 冗余清理记录

日期：2026-08-12
编号范围：R-1442～R-1458

## 搜索起点与覆盖范围

以“全量或局部同步章节、角色、主角、连续性事实和场景记录到 RAG”为功能起点，按 BFS 检查：

1. 前端与 Host：`rag_ingest_all`、prepare/upload 两阶段客户端入库、任务总量与进度事件。
2. Runtime：章节、蓝图角色、主角、状态、属性事件、伏笔、关系、场景和故事记录生产者，以及批量并发写入与 stale cleanup。
3. Orchestrator：`RagSyncContext` 构造、记录分类、embedding identity、批量写入、删除过滤器和章节派生清理。
4. Store：稳定 ID 元数据读取、条件删除、完整记录写入及 embedded/sidecar 两种注入路径。
5. 已删除 facade：从 Git 基线核对 `state/rag/facade.rs` 的全部旧转发方法，确认当前工作树已删除该层，不能恢复专用包装器。
6. 隐式依赖：检查 Runtime profile、`Arc<dyn VectorStore>`、Axum sidecar route、operation trace、任务 progress reporter、Serde metadata 和测试替身；未发现 XML、反射、动态代理或 AOP 入口。
7. 横向规则：比较全量同步、单章节同步、单记录连续性同步和客户端上传，确认它们共享同一 `RagSyncContext`、稳定 ID、metadata family 与写入 owner。

## BFS 结论与调用拓扑

```text
Host rag_ingest_all / client upload / internal resync
  -> Runtime record producers + progress
  -> RagSyncContext::for_ids / for_id_prefixes / for_data_type
  -> prepare_record
       root_text_family_metadata
       content hash + embedding identity classification
  -> OrchestratorContext::store_prepared_rag_records
  -> canonical VectorStore add_embeddings
  -> RagSyncContext::cleanup_stale
  -> canonical VectorStore delete
```

旧 `RagRuntimeFacade` 只做同名透传，其中 `rag_sync_context_for_*` 与 `cleanup_rag_sync_stale` 是对 `RagSyncContext` 自有方法的再次包装。facade 已退役后，Runtime 应直接使用 canonical owner；重新添加 Orchestrator 同名方法会制造第二层转发和新的公共 API。

## 伪安全与错误分叉结论

- embedding provider 的 batch size 已由具体 provider 构造边界校验；同步层不再把非法 0 静默改为 1，若自定义 provider 违反 trait 运行不变量则显式失败。
- Host task scope 已唯一负责将空项目的进度总量映射为 1；`RagIngestProgress` 不再重复 `.max(1)`。
- 进度超过声明总量表示 expected-record 计算与实际同步分叉；不再 `.min(total)` 后假装完成。
- metadata 中缺少 `chapter_id` 表示非章节记录，合法映射为 null；字段一旦存在，就必须是正整数，不能把 0、负数、字符串或 null 静默改为无章节。
- 章节进度标签只消费 canonical producer 生成的 metadata；缺少 `content_kind`、`chunk_index` 或 `chunk_count` 是内部合同破坏，不再显示通用标签或 `0/0`。
- `DeleteChapter` 只重包装通用删除错误且外层已有章节上下文；`ClearProject` 无任何构造点，二者均不应留在错误枚举。

## 优化记录

- R-1442：章节全量同步直接调用 `RagSyncContext::for_id_prefixes`，不恢复 facade 专用构造方法。
- R-1443：通用 data-type 同步直接调用 `RagSyncContext::for_data_type`。
- R-1444：蓝图角色同步直接使用 canonical data-type context 构造器。
- R-1445：主角档案同步直接使用 canonical data-type context 构造器。
- R-1446：stale cleanup 直接调用 `RagSyncContext::cleanup_stale`，删除对已退役 facade 方法的依赖。
- R-1447：删除 embedding batch size 的 `.max(1)`，非法 0 改为显式错误。
- R-1448：删除 `RagIngestProgress::new` 对 total 的第二次 `.max(1)`；空项目映射只由 task scope 拥有。
- R-1449：删除进度 `.min(total)` 静默截断，超过声明总量时显式失败。
- R-1450：更新进度合同测试，证明越界不会产生假完成事件，随后仍可按合法增量到达总量。
- R-1451：`chapter_id` 缺失与字段非法分离；存在字段必须解析为正 i64。
- R-1452：补充 0、负数、字符串和 null `chapter_id` 的拒绝测试。
- R-1453：删除从未构造的 `RagSyncError::ClearProject`。
- R-1454：删除只重包装通用删除错误的 `RagSyncError::DeleteChapter` 及 match 映射。
- R-1455：章节 RAG 标签缺少或空 `content_kind` 时显式失败，不再显示通用“RAG 记录”。
- R-1456：段落标签缺少或非法 `chunk_index` 时显式失败，不再使用 0。
- R-1457：段落标签缺少或非法 `chunk_count` 时显式失败，不再使用 0。
- R-1458：Runtime 将章节标签构造改为 `Result` 收集并传播错误；新增有效标签与三类损坏 metadata 的合同测试。

## 当前事实标准

RAG 增量同步上下文、分类、写入和 stale cleanup 只由 `a4n_orchestrator::rag::RagSyncContext` 与 Orchestrator 写入方法拥有；Runtime 只组织 Host 进度、并发批次和领域记录集合。进度总量、batch size、文本族 metadata 与章节标签均不再通过静默钳制、默认值或通用标签掩盖合同漂移。

## 集中验证

- `cargo fmt --all`：完成；随后 `git diff --check` 通过。
- 退役 facade 方法、错误变体和静默钳制残留搜索：只剩 `rag_ingest_progress_total` 对空项目总量的唯一 `max(1)`；该规则有专用测试并由 Host task scope 独占，不是重复 fallback。
- `cargo check -p a4n-orchestrator --lib`：通过；仅输出本批外既有 dead-code/private-interface warnings。
- `cargo check -p a4n-runtime --lib`：本专项原有的 4 个 `rag_sync_context_for_*`、1 个 `cleanup_rag_sync_stale` 错误全部消失；Runtime 当前继续被认证、配置、任务、图片、项目摘要、embedding 和 package-IO 等 89 个既有合同漂移错误阻断。
- Orchestrator RAG 目标测试仍未执行到测试体：`cargo test -p a4n-orchestrator --lib ...` 会先编译全 crate 测试配置，当前被 outline context re-export、`ClientGenerationRagInput: Debug` 和 character fixture `source_kind` 等本批外错误阻断。生产 library 已完整编译，新增测试源码已通过 rustfmt 与类型层静态复核。
