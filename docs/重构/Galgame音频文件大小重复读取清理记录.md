# Galgame 音频文件大小重复读取清理记录

日期：2026-08-16

## BFS 覆盖范围

```text
GalgameTab.handleImportAudio
→ frontend importGalgameAudio
→ import_galgame_audio Host command
→ Server / Tauri / CLI dispatch
→ Runtime galgame audio adapter
→ import_galgame_audio_for_user_validated
→ import_galgame_audio_validated
→ 音频落盘
→ create_media_asset
→ Galgame 媒体绑定
```

已覆盖 `ImportGalgameAudioInput` 严格 schema、项目编辑权限、目标行上下文、MIME/Base64
规则、文件持久化、媒体 provenance、资产文件事实计算和绑定失败补偿。未发现 XML、DI、反射、
动态代理、AOP 或路由别名。

## 已执行清理

1. `PersistedGalgameAudioFile` 删除重复 `file_size` 字段。
2. 删除落盘后仅为请求快照执行的第二次文件元数据读取和 `u64 → i64` 转换。
3. 请求快照直接使用已经通过 256MB 上限校验且实际写入的 `source.bytes.len()`。
4. 正式 `media_asset.file_size` 和 checksum 继续只由 `create_media_asset` 从落盘文件计算，
   保持文件事实唯一 owner。
5. 音频大小常量与 MIME 规范化函数收窄为模块私有。
6. `ImportedAudioSource` 及其字段/构造函数收窄为 Galgame 父模块可见，删除无消费者的
   crate 外公开面。

未新增默认值、兼容、回退或占位逻辑。

## 验证状态

按资源约束，仅完成 BFS、引用闭包与源码静态核查，未运行测试或构建。
