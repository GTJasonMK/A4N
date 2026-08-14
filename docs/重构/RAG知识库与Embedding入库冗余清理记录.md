# RAG 知识库与 Embedding 入库 BFS 冗余清理记录

日期：2026-08-14

## 搜索起点与覆盖范围

本专项从三个用户可见功能和四类自动同步功能出发，按 BFS 逐层追踪所有活跃生产路径：

```text
NovelList / NovelDetail 的“资料索引同步”
WritingDesk / RagQuickPanel 的增强检索
WritingDesk 的章节生成 RAG 预取
章节正文与章节分析完成后的章节索引同步
大纲、蓝图、主角资料变更后的局部同步
场景档案、出场与位移变更后的局部同步
项目重置、删除与恢复流程中的 RAG 清理
```

Host 侧 7 条活跃命令全部纳入闭包：

```text
rag_enhanced_search
rag_prepare_enhanced_search
rag_enhanced_search_with_vectors
rag_ingest_all
rag_prepare_ingest_all
rag_upload_ingest_embeddings
prepare_chapter_generation_rag_embedding
```

完整调用拓扑如下：

```text
NovelList / NovelDetail / WritingDesk
→ frontend/src/api/rag、frontend/src/api/chapter、generated Host TypeScript
→ Web apiInvoke / Tauri Host command / CLI manifest
→ a4n-api-contract command manifest 与 serde/schemars typed contract
→ a4n-server 字符串 dispatch、认证身份注入、严格 envelope 解析、任务进度
→ a4n-runtime RAG search / ingest / chapter generation adapter
→ ExpectedRagRecords canonical 构建
→ a4n-orchestrator RAG record builders / RagSyncContext / enhanced search
→ a4n-db canonical source records
→ EmbeddingProvider 与 VectorStore
   ├─ Server/embedded embedding + embedded QdrantVectorStore
   ├─ 本机桌面 embedding prepare/upload
   └─ SidecarVectorStore → a4n-rag-service → QdrantVectorStore
→ Qdrant dense vector、sparse BM25、metadata filter 与 project collection
```

局部同步路径横向追踪为：

```text
outline mutation ───────────────┐
blueprint mutation ─────────────┤
protagonist/profile mutation ───┤
scene profile workflow ─────────┼→ 对应 canonical record builder
chapter content/analysis ────────┘  → RagSyncContext
                                      → 唯一 sync/store/cleanup 实现
```

同时检查了以下显式和隐式依赖：

- Rust 模块导出、Cargo crate 边界、trait 实现、函数调用、生成 DTO 和前端类型引用。
- Host command manifest、Server 字符串路由、Tauri/Web 身份来源、项目 Editor 权限和协作根作用域。
- Runtime config snapshot、Embedding capability、RAG capability、任务进度、取消语义和 operation trace id。
- `RagDataType`、chunk id/prefix、content hash、source generation、索引格式、文本索引版本和 embedding identity。
- `VectorStore` 的 embedded/sidecar 实现、sidecar service protocol、managed sidecar 环境和 Qdrant collection 生命周期。
- 章节生成第一章规则、增强检索过滤、全量重建、本机 prepare/upload、增量写入和 stale cleanup。
- 项目重置、删除、cleanup outbox、Workflow 恢复和章节派生同步中的 RAG 清理调用。

仓库不存在该功能对应的 XML DI、注解扫描、运行时反射注册、动态代理或 AOP 切面。动态入口只来自显式 command manifest、字符串 dispatch、capability trait、sidecar protocol、环境配置和 `VectorStore` trait 实现，均已纳入上述闭包。

## 清理后的唯一事实标准

- `a4n-db` 的章节、大纲、蓝图、人物、伏笔和场景记录是可重建数据的 canonical source。
- `a4n-orchestrator` 的各类 RAG record builder 是 chunk id、文本和 metadata 的唯一构造实现。
- `ExpectedRagRecords` 是全量与局部同步共用的唯一预期记录集合；只保留去重所需的 id 集和 records。
- `RagSyncContext` 负责已有 metadata、embedding identity、同步分类和 stale scope。
- `a4n-runtime::rag_ingest::sync_records` 是批量比对、写入、统计和成功后清理 stale records 的唯一执行实现。
- `a4n-api-contract` 是搜索、全量入库、本机 prepare/upload 和章节生成 RAG 输出的唯一 Host 合同。
- Web JWT 或 Tauri Host session 是用户身份的唯一来源；RAG payload 不再携带第二份 `userId`。
- Qdrant 只保存可重建派生索引，不是业务 source of truth。

embedded `QdrantVectorStore`、`SidecarVectorStore` 和本机桌面 Embedding 是不同部署/执行位置下的适配器，共用同一 canonical records、Host contract、identity 校验和向量存储语义，不是多套业务实现。

## 已完成优化

1. 将全量章节、伏笔、大纲、分部、蓝图、世界观、人物、主角、状态、属性事件、关系和场景同步统一为 canonical `ExpectedRagRecords` 构建。
2. 将全量入库和所有局部同步统一到一个批量 `sync_records` 执行器。
3. 删除 Runtime 旧分类型 ingest 体系，共删除 22 个旧模块/测试文件，包括 `chapters`、`characters/**`、`scenes/**`、`story_records/**`、旧错误聚合、旧 sync contract 和旧 cleanup helper。
4. 删除每类记录独立读取 DB、独立构造标签、独立创建 sync context、独立累计统计和独立清理 stale records 的重复流程。
5. 删除 13 类仅用于旧进度文案的 `*_rag_record_label` API 及其孤立测试。
6. 删除 `LabeledRagRecord`、`PreparedLabeledRagRecord` 和 label/error-context 包装层，错误直接携带 canonical chunk id。
7. 删除失败后继续处理的 catch-and-continue 与错误数组聚合；任一记录准备或批次写入失败立即显式返回错误。
8. 删除固定并发数的 unordered 批次写入；按 provider 的唯一 `embedding_batch_size` 顺序写入。
9. stale records 只在全部待写批次成功后清理，避免部分写入失败后继续删除旧索引。
10. `RagSyncContext::for_data_types` 一次建立多类型 stale scope，删除每种 data type 重复扫描和重复 cleanup。
11. 全量任务 scope 直接持有已经构建的 expected records，删除 scope 阶段与执行阶段的第二次查库和构造。
12. `ExpectedRagRecords` 删除 15 类重复计数、预计算 metadata hash map 和重复 id vec；总数直接使用 `records.len()`。
13. 同步统计删除 `item_count`、错误数组和 merge 复杂度，只保留真实返回的 chunk/add/update/skip/delete 计数。
14. RAG 成功结果删除固定 `status = completed`、`success = true`、`errors = []` 以及 15 类无人消费的分类计数。
15. 前端删除 `result.success === false` 不可达分支和错误数组拼接逻辑。
16. 搜索、prepare、带向量搜索、全量入库、本机 prepare/upload 与章节生成 RAG 输出全部改为 typed Host contract。
17. 删除前端手写搜索、入库、prepare 和章节生成 RAG DTO，统一复用生成的 `host.ts` 类型。
18. 删除所有 RAG Host payload 中重复且可伪造的 `userId`，只使用认证 Host 身份。
19. `rag_upload_ingest_embeddings` 删除错误的 file-input manifest 声明。
20. prepare/search 命令标记为真实的只读 effect，上传与全量入库继续保留写 effect。
21. `top_k` 改为必填；零值直接拒绝，不再由后端补缺省值。
22. 搜索 `chapter_number <= 0` 和章节生成 `chapterNumber <= 0` 直接拒绝。
23. `data_types` 拒绝空数组、空白名称、非 canonical 大小写和重复项，不再 trim、去重或把空集合放宽为全部类型。
24. 删除 Runtime 搜索 preparation 的第二套分段参数入口，统一从 typed `EnhancedSearchInput` 进入。
25. 删除搜索 identity 双向转换副本，复用现有 `dynamic_embedding` 转换 owner。
26. 本机 prepare 不再提前清空项目 collection；prepare 只读取 canonical records、计算 hash 并返回待嵌入内容。
27. `content_hash` 从客户端忽略字段改为强制上传并逐条校验，source text 变化时要求重新 prepare。
28. 上传记录数量和 chunk id 集必须与当前 canonical prepare 结果完全一致；缺失、重复或额外记录均拒绝。
29. 上传向量在写入前统一验证数量、维度和有限值。
30. 本机 rebuild 只在身份、canonical records、hash、id 集和全部向量校验完成后清空旧 collection。
31. 非 rebuild 上传只在新记录全部写入成功后删除 stale records。
32. RAG typed inputs 全部启用 `deny_unknown_fields`；未知 envelope 字段由 Server allowlist 拒绝。
33. 新增一个最小上传合同测试，覆盖缺失 `rebuild`、缺失 `content_hash` 和未知字段拒绝。
34. 重新生成 Host TypeScript，并增加 manifest 断言锁定认证身份、typed schema、必填字段、只读 effect 和无 file-input 合同。

## 明确保留的活跃边界

- `VectorStore` trait 同时有 embedded 与 sidecar 实现，是生产部署边界；sidecar 只转发同一存储合同，不拥有第二套 RAG 业务规则。
- Server embedding 与本机桌面 embedding 使用相同 records、identity 和向量校验；本机路径存在是为了桌面运行模式，不是旧实现适配器。
- 全量同步、按材料类型局部同步和单章同步共享同一 record builder/RagSyncContext 规则，但 stale scope 不同，不能错误合并为全项目删除。
- 任务 scope 与执行前均校验 Editor 权限：前者防止未授权任务注册，后者处理注册后权限变化的 TOCTOU，不是重复业务规则。
- 章节生成的 RAG 是活跃的可选增强；未配置本机 embedding 时不提交客户端向量，但合同、权限、身份、fingerprint 和向量错误仍显式失败，不做旧格式兼容。
- operation trace、任务进度和取消检查承担可观测性与任务生命周期职责，未复制 RAG record 或同步规则。

## 集中验证

本批在超过 10 项优化后集中验证。全部 Cargo 命令直接复用仓库 `/home/fufu/Code/A4N/target/`，未在 `/tmp` 修改源码；前端合同脚本自身创建的临时目录只存放生成校验中间文件。

实际结果：

- `cargo check -p a4n-api-contract -p a4n-orchestrator -p a4n-runtime -p a4n-server`：通过。
- `a4n-api-contract` RAG 合同与 manifest：2/2 通过。
- `a4n-orchestrator` RAG 相关单测：70/70 通过。
- Blueprint、Character、Continuity、Outline、Scene RAG 集成合同：22/22 通过。
- `a4n-runtime` RAG ingest：11/11 通过。
- `a4n-server` command routes：11/11 通过。
- Web API parity、generation schema、TypeScript 和 fixture 校验：通过。
- 前端 Vitest：57 个文件、195/195 通过。
- 前端 `tsc && vite build`：通过。
- `cargo fmt --all -- --check` 与 `git diff --check`：通过。

受后端单次 60 秒限制，Runtime 和 Server 首次运行分别停在增量编译阶段并返回 124，均未进入测试；继续复用同一 `target/` 后，目标测试实际执行并全部通过。
