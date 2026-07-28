# RAG 索引迁移至 Qdrant

## 背景

原实现使用进程内 **LanceDB（向量）+ Tantivy（BM25 全文）**，编译依赖极重且 LTO 易 OOM。  
现改为 **Qdrant** 作为唯一索引后端；业务真相源仍为独立 **PostgreSQL**（`A4N_DATABASE_URL`），**索引不写入业务库**。

## 架构

```text
业务 PG  ──rebuild/sync──►  embed  ──►  a4n-rag-service
                                         │
                                         │ qdrant-client
                                         ▼
                                      Qdrant（dense + sparse BM25）
```

- API 合同：`VectorStore` / HTTP `/rag/*` **不变**
- 逻辑 collection 名仍为 `project_id` 字符串
- 物理 Qdrant collection：`A4N_QDRANT_COLLECTION`（默认 `a4n_rag`）
- Point id：`UUID v5(固定 ns, chunk_id)`；payload 存原文 `chunk_id`

## 环境变量

| 变量 | 说明 |
|------|------|
| `A4N_QDRANT_URL` | **必填**，如 `http://127.0.0.1:6334`（gRPC）或 `http://127.0.0.1:6333` |
| `A4N_QDRANT_API_KEY` | 可选 |
| `A4N_QDRANT_COLLECTION` | 默认 `a4n_rag` |
| `A4N_RAG_SERVICE_ADDR` | sidecar 监听，默认 `127.0.0.1:47644` |

## 本地启动

**推荐（项目内官方 release，无需 Docker 镜像）：**

```bash
./scripts/dev/setup-qdrant.sh    # 已装过可跳过
./scripts/dev/run-qdrant.sh      # 前台运行；HTTP 6333 / gRPC 6334
# 另开终端：
export A4N_QDRANT_URL=http://127.0.0.1:6334
# 再启动 a4n-rag-service / 全栈
```

二进制目录：`tools/qdrant/bin/qdrant`（见 `tools/qdrant/README.md`）。

**或 Docker：**

```bash
docker compose up -d qdrant
export A4N_QDRANT_URL=http://127.0.0.1:6334
```

## 数据迁移

**不做** Lance/Tantivy 文件导入。对每个项目执行 **rebuild**（从业务库读源 → embed → 写入 Qdrant）。

旧目录 `data/lance`、`data/tantivy` 可删除。

若存在 `.a4n-rag-mutation-in-progress`，确认 Qdrant 一致后删除标记再 rebuild。

## 中文全文

客户端 **字符 bigram/trigram + BM25 sparse**（对齐原 Tantivy Ngram 2–3 意图），与 Qdrant 默认英文分词无关。  
模型/算法变更须全量 rebuild。

## 依赖说明

- 产品依赖：`qdrant-client` crate  
- 仓库内 `crates/qdrant`：上游源码参考克隆，**非 workspace 成员**，勿编入 monorepo  

## 验收要点

- `cargo tree -p a4n-rag-service` 无 `lancedb` / `tantivy`
- 缺 `A4N_QDRANT_URL` 时 service/embedded 启动失败
- ingest / query / search_text / hybrid / delete / completeness / rebuild 可用
