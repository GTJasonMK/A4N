# RAG 文档写入与向量入库 BFS 冗余清理记录

日期：2026-08-11
编号范围：R-1409

## 搜索起点与覆盖范围

以“项目正文及派生事实写入 RAG，并可由浏览器或 Server 侧生成 embedding 后入库”为功能起点，按广度优先顺序检查：

1. 前端入口：`frontend/src/api/rag/ingest.ts`、新建项目和项目详情页的全量入库任务。
2. Host 合同：command manifest 中 `rag_ingest_all`、`rag_prepare_ingest_all`、`rag_upload_ingest_embeddings`，以及 Server dispatch/handler。
3. Runtime：全量入库、客户端 prepare/upload、章节/大纲/蓝图/角色/场景记录生产者、operation trace 和 sidecar vector-store adapter。
4. Orchestrator：`context/rag_store.rs`、`rag/write/*`、写入输入、metadata 标准化、embedding 批量校验和写入数量校验。
5. Store 与 sidecar：`a4n-rag-core` 的 `EmbeddingRecord`/`VectorStore`、Memory store、Qdrant store、RAG service ingest 合同和事件流。
6. 注入与配置：Runtime RAG store factory、embedding provider factory、full/sidecar/minimal profile；确认未通过 XML、反射、动态代理或 AOP 构造其他写入入口。
7. 横向搜索：全仓检查 `EmbeddingRecord` 构造、`content: Some/None`、正文读取和序列化边界，覆盖生产代码、测试支持和 live/sidecar 测试。

## 活跃调用拓扑

```text
browser full ingest / client embedding ingest
  -> Host command manifest + Server handler
  -> Runtime rag_ingest
  -> OrchestratorContext rag write owner
  -> canonical EmbeddingRecord
  -> injected VectorStore
       embedded -> Qdrant
       sidecar  -> RAG service -> Qdrant

internal chapter/outline/blueprint/character/scene sync
  -> canonical RagEmbeddingInput
  -> embedding provider
  -> canonical EmbeddingRecord
  -> same VectorStore path
```

## 优化记录

- R-1409：`EmbeddingRecord.content` 已是 BM25 所需的必填 `String`；删除 Orchestrator 三个生产构造点和五个测试/sidecar 构造点残留的 `Some(...)` 旧可空包装，所有入库路径直接消费唯一必填合同。

## 当前事实标准

RAG 入库正文只由 `a4n-rag-core::vector_store::EmbeddingRecord.content: String` 表达；上游在 Orchestrator 边界拒绝空正文，Memory/Qdrant/sidecar 共用同一记录合同，不保留可空正文、默认正文或兼容转换。
