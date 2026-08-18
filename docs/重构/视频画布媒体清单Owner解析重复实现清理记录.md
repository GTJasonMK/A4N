# 视频画布媒体清单 Owner 解析重复实现清理记录

日期：2026-08-15

## BFS 覆盖范围

从视频画布项目包媒体文件导入功能开始，追踪了：

```text
Runtime ZIP payload reader
→ validate_media_entry_owner（原始路径合同）
→ validate_media_entry_bytes（大小/checksum 合同）
→ media/owner.rs、media/payload.rs
→ VideoStudioProjectPackage 三类媒体实体与 MEDIA_FILE_FIELDS
→ Host/Runtime/测试/manifest/配置入口
```

已搜索两个校验入口及其全仓调用者、`owner_kind + field` 组合、三类实体查找、
公开 re-export 和 ZIP manifest 路径；未发现 XML、依赖注入、反射、动态代理、
AOP 或其他隐式入口。

## 已执行清理

新增单一的 `resolve_media_entry_owner` 解析闭包，集中处理：

- owner kind 与媒体字段的合法组合；
- generated image、generated video、media asset 的实体查找；
- 无效字段和缺失实体的统一错误边界。

原始路径校验和字节校验改为复用该解析结果，各自只保留自身规则。删除了两套
重复的 kind/field 分支和实体查找；路径、大小、checksum 和缩略图处理行为不变。

未新增 fallback、兼容、回退或占位逻辑。

## 静态验证状态

已完成全仓调用闭包搜索和局部 `git diff --check`；按资源约束未运行 Cargo、npm、
数据库、迁移、构建或 ZIP/浏览器验证，留待批量阶段。
