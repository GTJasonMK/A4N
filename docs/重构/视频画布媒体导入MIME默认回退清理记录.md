# 视频画布媒体导入 MIME 默认回退清理记录

日期：2026-08-15

## BFS 覆盖范围

从视频画布“导入媒体”按钮开始，按生产调用链追踪：

```text
VideoStudioCanvas.importMedia
→ frontend/src/api/canvas.ts::importCanvasMedia
→ import_canvas_media Host command
→ Runtime canvas_workflow/media_import
→ image / video / audio 分支
→ 图片解码、视频上传 MIME 校验、音频 MIME 校验
→ 媒体记录与画布节点保存
```

已搜索 `fallbackMime`、`mime_type`、`import_canvas_media` 的全仓调用者，覆盖
前端路由、生成 Host 合同、command manifest、Server dispatch、Runtime adapter、
Orchestrator 校验和三类媒体导入实现。未发现第二个生产入口，也未发现 XML、依赖
注入、反射、动态代理、AOP 或其他隐藏配置入口。

## 已执行清理

删除 `VideoStudioCanvas` 中按媒体类型猜测的 `fallbackMime`，直接传递浏览器
`File.type`：

- 图片后端要求真实的 PNG/JPEG/WebP MIME，`application/octet-stream` 本来就会被拒绝；
- 视频后端要求 MIME 与文件扩展名一致，固定猜测 `video/mp4` 会错误处理 webm、mov、m4v；
- 音频后端已有自己的可接受 MIME 规则，不需要前端再复制一套默认映射。

缺失 MIME 现在由对应的后端合同显式报错或按既有规则处理，不再由 UI 伪造类型掩盖输入事实。
未新增 fallback、兼容、回退或占位逻辑。

## 静态验证状态

已完成全仓引用闭包搜索和局部 `git diff --check`；按资源约束未运行 Cargo、npm、数据库、
迁移、构建或浏览器验证，留待批量阶段。
