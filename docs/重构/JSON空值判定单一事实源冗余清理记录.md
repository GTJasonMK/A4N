# JSON 空值判定单一事实源冗余清理记录

日期：2026-08-15

## 搜索起点与 BFS 覆盖范围

本专项从两个真实业务入口开始：

```text
蓝图生成/精炼与反向导入
→ blueprint character / relationship merge
→ 保留已有非空 JSON 字段

章节分析伏笔同步
→ chapter analysis output
→ foreshadowing related_characters resync
→ 保留已有非空 JSON 字段
```

BFS 追踪覆盖：

- `a4n-orchestrator` 蓝图生成完成、蓝图精炼和反向导入的持久化入口。
- `a4n-orchestrator` 章节分析输出与 `a4n-db` 伏笔事务同步入口。
- `a4n-db` 角色档案蓝图 seed 合并与其持久化入口。
- `a4n-orchestrator` 蓝图 JSON world/array 字段的非空校验入口。
- `a4n-db` blueprint sync 的 character/relationship 两条写入路径及 foreshadowing rules。
- `serde_json::Value` 的 `null`、空白字符串、空数组、空对象、布尔值和数字值语义。
- `a4n-core::json_contract` 现有 JSON 边界工具模块及其 crate 依赖闭包。

未发现这些字段由 XML、YAML、TOML、DI、反射、动态代理或 AOP 额外调用；动态边界仅为 Rust/Serde JSON 数据和数据库事务入口。

## 已完成优化

1. 将 blueprint character、blueprint relationship、foreshadowing、character blueprint seed 和 blueprint JSON non-empty validation 中完全相同的 JSON 空值判定收敛为 `a4n_core::json_contract::is_empty_json_value`。
2. 删除五个生产路径各自维护的局部副本，并让 seed 规则测试直接引用同一合同。
3. 保留原有语义：`null`、空白字符串、空数组和空对象视为空；布尔值与数字值不视为空。

## 清理后的唯一事实标准

```text
JSON 空值判定
→ a4n_core::json_contract::is_empty_json_value
```

未新增默认值、兼容转换、异常吞掉、适配器或占位逻辑。

## 静态验证状态

按当前资源约束，本专项未运行 Cargo、npm、数据库测试、迁移执行、构建或后台任务，也未在 `/tmp` 修改文件。已完成调用闭包和三处重复实现的源码核对；集中静态检查留待后续专项批次。
