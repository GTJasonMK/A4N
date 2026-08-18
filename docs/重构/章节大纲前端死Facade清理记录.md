# 章节大纲前端死 Facade 清理记录

日期：2026-08-15

## BFS 覆盖范围

从章节大纲编辑功能开始，追踪前端 `api/outline.ts` 的导出与调用、Outlines 页面实际
写入路径、Host generated types、command manifest、Server/Runtime dispatch 及
Orchestrator 更新实现。确认页面使用 `save_chapter_outline`；前端没有调用
`updateChapterOutline`，而后端 `update_chapter_outline` 仍由 Host command 暴露，不能删除。

## 已执行清理

删除仅做 `apiInvoke("update_chapter_outline")` 转发且无前端消费者的 wrapper 及其局部类型
导入；保留生成合同和后端 command，避免把“前端死 facade”误判为“系统能力退役”。
未新增 fallback、兼容、回退或占位逻辑。

## 静态验证状态

已完成前端、Host manifest、Runtime、Orchestrator 的调用闭包搜索和局部 `git diff --check`；
按资源约束未运行 npm、Cargo、数据库、构建或浏览器验证。
