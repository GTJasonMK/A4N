# Manga 身份合同冗余清理记录

## 搜索覆盖范围

本专项从漫画项目工作区的章节提示词、流水线和参考图功能出发，按 BFS 追踪完整生产链路：

```text
ComicProjectWorkspace
→ MangaTab / MangaChapterDetailPanel / ReferenceBindingsPanel
→ frontend api/manga pagePrompts / pipeline / status
→ apiInvoke
→ Web authenticated command / Tauri invoke_host_command
→ Server manga dispatch 与 records/pipeline/preview handlers
→ Runtime manga_records / manga_pipeline actions
→ Orchestrator Manga application
→ text project 或 derived project family permission
→ DB chapter_manga_prompt / workflow / project membership records
```

横向覆盖 12 条活跃命令：页面提示词列表、参考图绑定、提示词更新与优化、单页提示词删除、章节漫画删除、流水线启动与恢复、两类请求预览、状态查询和取消。隐式依赖覆盖 command manifest context/schema、生成 Host 类型、Web Bearer token、Tauri 外层 `user_id`、后台 workflow task、runtime config、下游图片清理、prompt registry，以及文本项目和派生项目家族的 Viewer/Editor 权限。仓库中没有 Manga 相关 XML DI、反射、动态代理或 AOP；动态入口仅为 Host 命令分派、异步 workflow 和 prompt registry，均已纳入闭包。

## 清理前分叉

```text
Web JWT / Tauri invoke_host_command.user_id
→ Server user_id
→ Runtime
→ Orchestrator project/family permission

frontend api/manga
→ 12 个命令再次发送 userId
→ Server handler 不读取该业务字段
→ 宽松 input envelope schema 继续接受退役身份参数
```

Manga 的读取、编辑、流水线、下游清理和 LLM 调用始终使用 Host 认证身份。业务 `userId` 不进入 input DTO，也不参与任何权限或数据路径。

## 优化记录

1. `list_manga_page_prompts` 删除重复 `userId`。
2. `bind_manga_reference_image` 删除重复 `userId`。
3. `update_manga_prompt` 删除重复 `userId`。
4. `optimize_manga_prompt` 删除重复 `userId`。
5. `delete_manga_prompt` 删除重复 `userId`。
6. `delete_manga_by_chapter` 删除重复 `userId`。
7. `start_manga_pipeline` 删除重复 `userId`。
8. `preview_start_manga_pipeline_request` 删除重复 `userId`。
9. `resume_manga_pipeline` 删除重复 `userId`。
10. `preview_resume_manga_pipeline_request` 删除重复 `userId`。
11. `get_manga_pipeline_status` 删除重复 `userId`。
12. `cancel_manga_pipeline` 删除重复 `userId`。
13. 删除 3 个 Manga API 模块对 `requireCurrentUserId` 的零剩余依赖。
14. 7 个 Manga input envelope schema 改用既有 `strict_typed_input_schema`，覆盖全部 12 条命令。
15. Galgame 与 Manga 的 manifest 身份断言合并到唯一测试 helper，删除第二套循环实现。
16. 新增一项 Manga manifest 回归，一次验证 12 条命令均不声明或接受业务 `userId`。

## 清理后唯一事实标准

```text
用户身份
→ Web：Bearer token 认证出的 user_id
→ Tauri：invoke_host_command 外层 user_id

业务范围
→ input 中的 project_id / chapter_id / prompt_id

权限
→ 文本项目：verify_project_permission
→ 派生项目：root 与 derived project family 双重 permission
→ Viewer：列表与状态
→ Editor：流水线、提示词、参考图、删除与取消
```

未新增旧参数兼容、身份覆盖规则、默认用户、权限回退或 transport 适配器。

## 集中验证

- `npx --no-install tsc --noEmit`：通过。
- `npm run check:web-api-parity`：通过；API 类型、generation schema、TypeScript 输出和 fixtures 均无漂移。
- `cargo test -p a4n-api-contract commands_use_one_authenticated_identity`：主题、Galgame、Manga 和 session 共 4 项目标测试通过，复用仓库现有 `target/`。
- `rg` 退役字段残留搜索：通过，3 个 Manga API 模块中无 `currentUserId` 或业务 `userId`。
- `git diff --check`（仅本专项文件）：通过。
