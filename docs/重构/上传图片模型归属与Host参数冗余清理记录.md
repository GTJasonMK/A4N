# 上传图片模型归属与 Host 参数冗余清理记录

日期：2026-08-12
编号范围：R-1770～R-1781

## BFS 搜索覆盖范围

从“用户上传外部图片”功能入口逐层广度优先展开：

1. 页面入口：图片列表上传、单角色肖像上传、视频提示词参考图上传。
2. 前端 API：`uploadImage`、生成 `UploadImageInput`、认证用户存储和 Web/Tauri `apiInvoke`。
3. Host 合同：command manifest、生成 Host 类型、Server dispatcher/handler、认证用户与项目 context。
4. 应用链路：Runtime 文件上传 → Orchestrator 项目编辑权限、图片解码与导入记录准备 → DB generated image 记录。
5. 横向导入路径：视频 Canvas 图片导入复用同一 `ImportedImageUploadInput` 与记录创建链。
6. 固定事实：API Contract 已显式拒绝调用者提供 `model`；所有外部导入图片的持久化模型统一为 Orchestrator 常量 `external-import`。
7. 隐式入口：Server cleanup recovery 进程测试中的正常失败清理与进程终止窗口请求夹具。

搜索未发现 XML、反射、动态代理、AOP 或依赖注入容器配置。只有两个内部调用者构造 `ImportedImageUploadInput`：Runtime 永远传 `None`，Canvas 永远硬编码同一 `external-import`；该可选字段没有真实分叉需求。

## 优化记录

- R-1770：完成页面、前端 API、Host、Runtime、Orchestrator、DB、Canvas 与进程夹具的全链 BFS。
- R-1771：`upload_image` 顶层 Host 参数改为拒绝未知字段，只接受 `projectId` 与严格 `input`。
- R-1772：前端上传 API 删除请求体中无效的 `userId`，身份只来自认证 transport。
- R-1773：第一个 Server 进程上传夹具删除无效顶层 `userId`。
- R-1774：第二个 Server 进程上传夹具删除无效顶层 `userId`。
- R-1775：记录创建失败恢复夹具删除已退役的 `input.model`。
- R-1776：进程终止窗口恢复夹具删除已退役的 `input.model`。
- R-1777：内部 `ImportedImageUploadInput` 删除无消费者的可选 `model` 字段。
- R-1778：Runtime 删除永远传入的 `model: None`。
- R-1779：Canvas 导入删除硬编码的 `model: external-import`。
- R-1780：Orchestrator 删除 optional model 校验/default 分支，直接使用唯一 `IMPORTED_IMAGE_MODEL`。
- R-1781：删除第二个进程夹具随之无用的 `user_id` 局部赋值。

## 唯一实现结论

- 调用者只描述文件、图片用途、提示词和可选关联信息，不能伪造生成模型。
- 认证身份只来自 Web Bearer/Tauri Host dispatcher，不进入 upload command 参数体。
- 所有外部导入图片的模型字段由 Orchestrator 唯一写入 `external-import`。
- Runtime、Canvas 和测试不再维护 model 默认、硬编码或空值三套表达。

## 集中验证

- `cargo fmt --all`：通过。
- 使用仓库默认 `target` 重建生成器，执行 `--write` 与 `--check`：通过。
- `cargo test -p a4n-api-contract`：库测试 28 项、生成器测试 3 项全部通过；覆盖上传图片拒绝 caller model。
- `cargo check -p a4n-server`：通过，同时覆盖 Orchestrator 与 Runtime。
- `npx tsc --noEmit`：通过。
- 使用独立临时 PostgreSQL 数据库执行两项 Server 进程测试：全部通过，真实覆盖登录、上传、记录/清理双失败，以及数据库提交前进程终止恢复链。
- 临时数据库删除确认、退役字段残留搜索和 `git diff --check`：通过。
