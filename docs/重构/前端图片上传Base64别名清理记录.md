# 前端图片上传 Base64 别名清理记录

日期：2026-08-15

## BFS 覆盖范围

从图片/立绘/视频素材上传功能开始，追踪：

```text
ImagesTab / SinglePortraitDialog / UploadPortraitDialog / VideoMaterialWorkspace
 / useVideoPromptGenerationActions
→ utils/imageUpload::readFileAsBase64（转发别名）
→ utils/fileUpload::readFileAsBase64（唯一实现）
→ FileReader data URL → base64 请求字段
```

已搜索 `readFileAsBase64` 的全仓导入、调用和导出链，覆盖所有前端上传页面；确认
`imageUpload` 没有改变参数、错误或返回值，也没有 Host、路由、注入、反射或配置入口
依赖该别名。

## 已执行清理

删除 `imageUpload.ts` 的原样 re-export，所有图片相关页面改为直接从 `fileUpload.ts`
导入唯一实现。图片 MIME 规则仍由 `imageUpload.ts` 负责，文件读取职责不再被别名层转发。

未新增 helper、fallback、兼容、回退或占位逻辑。

## 静态验证状态

已完成全仓引用闭包搜索和局部 `git diff --check`；按资源约束未运行 npm、Vitest、构建、
浏览器或后端验证，留待批量阶段。
