# A4N desktop and Web workflows

## Choose the host

- For an installed desktop app or an existing Web deployment, use its configured Server and sign in; do not require local database, JWT, or runtime binaries.
- For a source checkout or headless local deployment, follow [cli-operation.md](cli-operation.md) for prerequisites and lifecycle commands.
- Use the UI for browser-local drafts, drag/drop canvas positioning, visual timeline/player work, clipboard actions, and native file selection. Use CLI for the underlying saved business operations.

## First use

1. Log in or register.
2. Open **设置** and configure/test LLM.
3. Configure Embedding only for RAG/search, image service for portraits/images/manga, and video service for video generation.
4. Return to the project list and choose **AI 辅助创作**, **自由创作**, **导入分析**, or **视频画布项目**.
5. Track long operations in the progress dialog or **设置 → 请求队列**.

## AI novel path

**新建项目 → 灵感对话 → 生成/确认故事蓝图 → 世界观与角色档案 → 分部/章节大纲 → 写作桌 → 章节分析/拍摄剧本 → 图像或派生项目**

Preview model requests before expensive generation when the page offers **查看模型请求**. Save chapter edits explicitly; local drafts are recovery aids, not the canonical saved content.

## Import path

**导入分析/项目导入页 → 选择 TXT/Markdown/EPUB/DOCX → 导入审核 → 接受章节 → 项目分析 → 结构反推预览 → 按需应用蓝图/大纲 → 写作桌**

Review parsed chapters and deduplication before acceptance. Reverse-engineering “apply” changes project structure; preview first.

## Multimedia derivation

From a text project's menu, create a manga, video, or GALGame project with a source chapter range. Complete the domain pipeline, edit/bind generated assets, and export from that derived workspace. After source edits, use **同步来源 → 检查更新** before applying safe updates or resolving conflicts.

## Common safety rules

- **删除项目**, reset generated data, rollback, detach source, delete plans/assets, and bulk apply operations are irreversible or disruptive; inspect impact and confirm the exact target.
- Cancelling generation preserves completed stages where supported but does not imply success.
- Content dependency/status views show downstream effects before changing source text or deleting assets.
- If a feature is disabled, check project type/stage, account permission, runtime mode, and required provider configuration.

For detailed page/button behavior in a source checkout, consult sections 1–13 of `用户操作流程.txt` and only load the relevant section. Treat its CLI section as non-authoritative; use [cli-operation.md](cli-operation.md) and [cli-advanced.md](cli-advanced.md) for current commands.
