# RAG 向量库管理合同分叉 BFS 清理记录

日期：2026-08-11
编号范围：R-1429～R-1441

## 搜索起点与覆盖范围

以“用户执行 RAG 入库、检索、项目清理及 sidecar 健康检查”为功能起点，按 BFS 检查：

1. 前端入口：`frontend/src/api/rag`、生成 Host 命令清单及任务展示；活跃用户能力只有检索、预处理检索、带向量检索和全量入库。
2. Host 路由：API contract command manifest、Server dispatch、`rag_embedding` handlers 和 Runtime host actions。
3. Runtime/Orchestrator：RAG 入库、搜索、增量同步、项目/章节清理、稳定记录 ID 元数据读取和完整性检查。
4. 抽象与依赖注入：`Arc<dyn VectorStore>`、Runtime profile factory、`A4nCapabilities.rag` 与 `RagClient`。
5. sidecar 传输：`SidecarVectorStore`、`RagOperation`、HTTP capability trait/实现、请求 scope 与输出 DTO。
6. sidecar 服务：Axum router、management/search handlers、Qdrant backend、服务目录和 `/healthz`。
7. 测试替身与流程测试：Disabled/Memory/ShortWrite store、Recording client、独立进程 RAG 测试和写入失败断言。
8. 隐式入口：全仓搜索 operation label、HTTP path、trait method、生成 Host 命令、服务目录和 feature 配置；未发现 XML、反射、动态代理或 AOP 注册。

## BFS 结论与调用拓扑

活跃生产链为：

```text
Frontend RAG search / ingest
  -> generated Host command
  -> Server explicit dispatch
  -> Runtime action / rag_ingest
  -> Orchestrator VectorStore canonical trait
  -> embedded QdrantVectorStore
     或 SidecarVectorStore
       -> RagClient canonical HTTP methods
       -> Axum explicit routes
       -> QdrantVectorStore
```

`VectorStore`、`RagClient` 与 sidecar router 共同保留的唯一操作集合为：写入、向量/文本/混合搜索、条件删除、删除集合、按 ID/过滤条件读取元数据，以及按稳定 ID 生成完整性报告。

以下旧管理能力只残留在 Runtime 适配器和禁用实现中：

```text
collection_exists / count / list_collections / update_metadata
health_check / completeness_report(expected_count)
  -> Runtime RagOperation 与旧输出 DTO
  -> RagClient 无对应方法
  -> sidecar router 无对应路径
  -> service handler 无对应实现
  -> 前端与 Host 无入口
```

因此这些方法不属于任何活跃执行路径，且会让 Runtime 对已收敛的 canonical trait 产生编译级分叉。

## 横向实现结论

- collection existence、collection count 与 collection list 是 Qdrant backend 的内部建库/连通性细节；保留在 `QdrantVectorStore` 内部即可，不应穿透业务抽象和 sidecar 协议。
- metadata 写入的唯一事实标准是完整 `EmbeddingRecord` 重新入库；直接 `update_metadata` 会形成绕开正文、向量和稀疏索引一致性的第二写路径。
- sidecar 健康检查的唯一事实标准是 `CapabilityClient::health` 对 `/healthz` 的检查；不再在 `VectorStore` 上重复定义布尔健康方法。
- 完整性检查的唯一事实标准是 `completeness_report_by_ids`；仅比较 expected count 无法指出缺失记录，也无法识别等量错位数据。
- 失败写入测试只验证 canonical 可观察结果“稳定 ID 无记录”，不再依赖被抽象层刻意隐藏的物理 collection 是否存在。

## 优化记录

- R-1429：删除 Runtime sidecar 与 Disabled store 中未进入 canonical trait 的 `collection_exists` 实现及 transport operation。
- R-1430：删除无服务端路由、无 capability 方法的 `count` sidecar 分叉。
- R-1431：删除无活跃消费者的 `list_collections` sidecar 分叉；Qdrant 内部连通性检查保持封装。
- R-1432：删除绕开完整记录重建语义的 `update_metadata` sidecar 分叉。
- R-1433：删除 `VectorStore::health_check` 残留，健康检查只走 `/healthz` capability 合同。
- R-1434：删除仅按数量判断的 `completeness_report` 残留，统一使用按稳定 ID 的完整性报告。
- R-1435：删除只服务于旧 existence/health RPC 的 `BoolOutput`。
- R-1436：删除只服务于旧 count RPC 的 `CountOutput`。
- R-1437：删除只服务于旧 collection list RPC 的 `CollectionsOutput`。
- R-1438：请求 scope 测试改用真实存在的 `rag.drop_collection` operation label，不再保留虚构的 `rag.list_collections`。
- R-1439：Disabled store 测试改用 canonical `metadata_by_ids` 读取合同验证显式失败，同时保留无 RAG collection 时删除幂等语义。
- R-1440：单条 embedding 维度失败测试改为断言稳定 ID 无元数据记录，并按实际合同重命名。
- R-1441：批量 embedding 数量失败测试改为断言全部稳定 ID 无记录，并按实际合同重命名。

## 当前事实标准

业务代码只依赖 `a4n_rag_core::vector_store::VectorStore` 当前声明的方法；sidecar transport 只映射 `a4n_capabilities::RagClient` 实际公开且 Axum router 已注册的路径。向量库物理管理细节留在 backend 内部，健康检查走统一 capability，元数据更新走完整记录重建，完整性只按稳定 ID 判断。

## 集中验证

- `cargo fmt --all -- --check`：通过。
- `git diff --check`：通过。
- 旧管理 operation label、Runtime trait method 与专用输出 DTO 全仓残留搜索：除独立的 Host 管理员 `health_check` 外无残留；该命令不属于 RAG vector-store 合同。
- `cargo test -p a4n-rag-core --lib -- --test-threads=1`：31 项通过。
- `cargo test -p a4n-rag-service --bin a4n-rag-service -- --test-threads=1`：8 项通过。
- `cargo test -p a4n-capabilities --lib -- --test-threads=1`：26 项通过。
- `cargo check -p a4n-runtime --lib`：本专项原有的 `VectorStore` 额外方法和不存在 `RagClient` 方法错误全部消失；检查继续暴露认证、生成任务、图片、RAG sync 等下一批既有合同漂移。
- Orchestrator RAG 写入目标测试未执行到测试体：当前被 outline context re-export、`ClientGenerationRagInput: Debug` 和 character fixture `source_kind` 等既有测试编译错误阻断；修改后的生产 `a4n-orchestrator` library 已在 Runtime check 中完成编译，仅测试配置仍有上述无关错误。
