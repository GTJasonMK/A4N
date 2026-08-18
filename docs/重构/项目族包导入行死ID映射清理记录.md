# 项目族包导入行死 ID 映射清理记录

日期：2026-08-15

## 搜索起点与 BFS 覆盖范围

本专项从 `project_family_import::import_package_rows` 的 `ImportIdMaps` 出发，按导入顺序展开所有映射生产者和消费者：

```text
项目族包 manifest
→ import_package_rows
→ 各实体 ActiveModel 插入
→ ImportIdMaps 写入
→ 外键/JSON/延迟目标 remap 消费
→ 事务内后续查询与项目返回
```

覆盖了 `a4n-db/src/project_family_import/` 下的根项目、文本核心、快照、档案、生成、媒体、Galgame、视频、成片和源同步导入模块，并用全仓搜索复核 `ImportIdMaps` 字段、JSON ID 名称、实体关系、Runtime 包边界、Orchestrator 校验及数据库模型/索引/触发器。

未发现 XML、依赖注入容器、反射、动态代理或 AOP 配置参与这些导入映射；导入 ID map 是 crate 内部 Rust 状态，不会通过 Serde 或 Host 边界隐式传递。

## 发现

以下映射均只有“字段声明 + 插入后写入”，没有任何读取、JSON remap、外键赋值或后续业务消费者：

- `scene_transition_ids`
- `scene_entity_state_ids`
- `source_snapshot_outline_ids`
- `source_snapshot_profile_ids`
- `profile_evidence_ids`
- `media_binding_ids`
- `galgame_choice_ids`
- `assembly_clip_ids`
- `source_sync_decision_ids`

这些实体的源 ID仍用于 manifest 唯一性/引用校验和导入时的其他字段校验；只有导入后不再被引用的“旧 ID → 新 ID”缓存是冗余的。

## 已执行清理

1. 从 `ImportIdMaps` 删除上述 9 个无消费者字段。
2. 对应导入块从 `insert_mapped_import_rows!` 改为 `insert_import_rows!`，删除无意义的 map 写入。
3. 保留每个实体行的完整插入、业务字段 remap、外键映射、延迟引用更新、manifest 校验和事务边界。
4. 同批次删除 8 个导入源文件中未被直接使用的 `NotSet` import；`NotSet` 仍只保留在实际定义该宏的模块中。

仍保留的映射只服务于后续真实消费者，例如场景出场、项目/快照/档案、文本锚点、生成意图、媒体资产、Galgame 场/行、视频片段、成片时间线以及源同步运行/冲突的 ID remap。

未删除任何包字段、数据库行、主键、索引、校验或触发器；未新增兼容、默认、回退、吞错或占位逻辑。

## 静态验证状态

已完成全仓字段引用闭包搜索、导入顺序审查、JSON/数据库边界检查和 `git diff --check`。本专项未运行 Cargo、npm、数据库、迁移或后台验证，也未写入 `/tmp`；集中验证留待累计批次，使用仓库现有 `/home/fufu/Code/A4N/target/`。
