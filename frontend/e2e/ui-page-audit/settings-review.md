# 设置、管理、认证与账户逐项审查

频率均按工作流程推定。用户路径是定位配置或问题 → 阅读当前值和状态 → 编辑/执行 → 确认结果；账号、权限和 API 行为沿用原实现。

默认页证据在 `.dev-runtime/ui-pages/settings/`。内页使用 `settings-final-2026-09-24T22-02-11-660Z` 的 31 项通过结果；独立审查者实际看过其中 96 张关键状态图，完整已看/未复看清单保留在 [独立复核记录](../../../.dev-runtime/ui-pages/settings-final-independent-review.md)。未复看同状态图片不算新增独立复核。新的模型发现、日志宽度与修正项目类型截图由主流程补审，单独记在本文末尾。

## 默认工作面与配置内页

| ID / 工作面 | 任务、推定频率与注意力顺序 | 问题与设计决定 | 浏览器证据 |
| --- | --- | --- | --- |
| S01 LLM 列表/编辑/新建/价格 | 按需；服务身份 → 模型/连接 → 容量 → 保存/测试 | 中宽两层侧栏挤压字段，按编辑器实际宽度排列；当前配置与系统默认区分；高级 JSON 按需展开，测试已保存配置直接可见 | `settings-closeout` 的 S01-editor，三宽默认页；发现列表另见 S01-discovery |
| S02 图片列表/编辑/新建/价格 | 按需；服务 → 尺寸名称及宽高 → 并发 → 保存 | 宽高曾参与 React key 导致输入中重新挂载；稳定行身份，尺寸独占一行，宽高有独立名称 | S02-size-edit 完整录入 1536 并保留焦点；S02-editor；S02-discovery |
| S03 视频列表/编辑/新建/价格 | 按需；服务 → 时长模式及范围 → 尺寸/FPS → 保存 | 原时长上下限被无关字段打断；连续排列模式、默认值、最短和最长，保留全部策略 | S03-editor；S03-discovery；默认三宽图 |
| S04 嵌入三后端/新建/价格 | 按需；后端 → 可用模型/下载 → 维度 → 计价 | 当前能力价格优先，其余价格可展开；API、FastEmbed、local_hash 分别进入；保留新建时原有“基础本地哈希向量”默认值 | `models-final` 三后端；`final-extras` S04-editor |
| S05 搜索/网页读取 | 按需；协议/地址/密钥 → 超时限额 → 共同保存 | 连接参数提前，共用表单和独立保存栏；避免让用户误以为只保存搜索 | 三宽默认；键盘 Enter 同时保存 75/45 秒；0 秒原生范围校验 |
| S06 Agent 技能 | 按需；能力域 → 技能用途/开关 → 草稿 → 保存 | 手机类别用现有选择器；dirty 从当前值导出，恢复原值即干净；慢保存不覆盖随后编辑 | 各类别内容；切回原值、延迟保存后继续编辑均通过 |
| S07 提示词目录 | 经常配置时使用；分类/检索 → 模板 → 正文 → 工具 | 手机列表和正文比例失衡，首次加载选首个模板；正文最小高度不再被通用移动规则覆盖 | 默认三宽；SD01–04 各工作面独立验证 |
| S08 主题目录 | 偶发；主题名称/色带 → 应用 → 编辑/导入 | 收敛色带宽度，长名称换行；仅可删除主题显示批量选择；默认暖书房色值不变 | 默认三宽；SD05–07 |
| S09 备份 | 偶发；备份范围 → 导出/选择导入 → 确认 | 默认布局清楚，保留；“最近导出位置”改为准确的“最近导出文件” | 三宽已渲染；文件入口与取消可达，未以 UI fixture 冒充真实备份恢复 |
| S10 管理概览 | 按需诊断；总体状态 → 异常服务 → 详情 | 停用服务不再显示正常绿色；加载失败保留作用域错误与原刷新入口 | 管理默认页与 SD08–15 |
| S17 请求队列 | 经常；活跃任务 → 阻塞/失败 → 恢复/取消 | 负载卡曾被 flex 压扁；禁止收缩，详情保留全部检查点和调用记录 | 默认卡片高度；SD19–20 |
| S18 调用统计/日志 | 按需；时段 → 成败/金额 → 趋势 → 具体调用 | 手机图表不整体缩小字号；保留完整时段和已返回记录；状态、失败及未定价明确 | SD16–18；日志筛选宽度收尾见下文 |
| S20 账户与应用 | 偶发；身份 → 资料/密码 → 设备/存储 | 展开入口补箭头；设备主行显示浏览器/系统，原 UA 留在诊断；表单支持 Enter/autocomplete | SD21–27 与三种身份分别验证 |

## 31 个内页与任务流程

证据简称 F 指 `.dev-runtime/ui-pages/settings-details/settings-final-2026-09-24T22-02-11-660Z/`；每行对应以该 ID 开头的实际状态图和 `settings-details-report.json` 中的动作轨迹。

| ID | 任务 / 频率 | 注意力顺序 | 审查结论、改动与验证 |
| --- | --- | --- | --- |
| SD01-prompt-editor | 编辑模板 / 经常 | 正文 → 未保存状态 → 保存 → 离开确认 | 移动正文恢复约 288px 可用高度；保存后再编辑并切换模板，取消保留新草稿。F selected/dirty/saved/discard |
| SD02-prompt-history | 比较模板版本 / 按需 | 版本对 → 改动 → 差异全文 | 两个选中版本与长差异正文清楚；原版本参数和完整差异均核对。F history/version-diff |
| SD03-prompt-override | 项目独立模板 / 按需 | 项目 → 覆盖内容 → 更新/恢复全局 | 作用范围、确认及取消明确；仅选中项目发生更新，移除后显示使用全局内容。F existing-override/confirmation/global-restored |
| SD04-prompt-validation | 校验与渲染 / 按需 | 缺失变量 → 输入 → 当前/原始输出 | 校验问题与渲染结果分区，长文换行；保留直接渲染和模板对照。F missing-variables/direct-render/render-comparison |
| SD05-theme-editor | 完整主题编辑 / 偶发 | 模式/基本值 → 色彩/效果/背景 → 高级项 → 保存 | 收敛双页头；按容器排列字段；固定保存/取消，流式美化独立展开；原主题合同及背景范围保留。F 六编辑区 |
| SD06-theme-create-import-package | 新建/导入/主题包 / 偶发 | 内容来源 → 字段/JSON/对象 → 确认 | 三个入口分别打开；JSON 输入约 224px，避免两行编辑区；创建与导入使用真实形状的响应。F create/import/package |
| SD07-theme-more-rename-bulk | 主题维护 / 偶发 | 当前主题 → 名称/选择数量 → 影响 → 确认 | 重命名支持 Enter；批量删除显示数量及取消，内置与当前主题保护保留。F menu/rename/selection/confirmation/result |
| SD08-admin-users | 用户与用量 / 按需 | 用户/邀请 → 令牌摘要 → 使用记录 | 摘要六项及诊断完整，行菜单对应所选用户；保留当前分组。F users/token-summary/usage |
| SD09-admin-create-user | 新增账号 / 偶发 | 账号字段 → 角色 → 提交反馈 | 四字段、角色与操作均可达；完整输入后模拟创建，列表出现指定记录。F create-user-filled/created-user |
| SD10-admin-role-disable-invite | 账号维护 / 偶发 | 对象 → 角色或禁用影响 → 确认 | 角色原自由文本能力保留；禁用确认、邀请码创建分别核对对象，未改变权限实现。F role/disable/invite |
| SD11-admin-projects | 检查归属 / 按需 | 类型/项目 → 所有者 → 诊断 | 归属信息有内容；旧“未知类型”来自混合样本，已校正为 novel_long 并在 final-extras 补三宽图，未记作产品修复 | 
| SD12-admin-tasks | 取消排队任务 / 按需 | 任务状态 → 诊断 → 对应行取消 | 菜单在手机视口内完整，取消只影响选中任务并反馈列表结果。F pending/actions/cancelled |
| SD13-server-llm | 公共 LLM 配置 / 偶发 | 服务器身份 → 模型 → token 价格 → 保存 | 与本机配置区分，输入/输出/缓存的币种和百万 token 单位完整。F server-editor/server-pricing |
| SD13-server-image | 公共图片配置 / 偶发 | 服务器身份 → 尺寸 → 按次价格 | 宽高成对且可读；图片按次价格优先；全部其他价格仍可展开。F server-editor/server-pricing |
| SD13-server-video | 公共视频配置 / 偶发 | 模式/时长范围 → 尺寸/FPS → 按秒价格 | 时长条件连续，移动字段不再挤压；保留全部配置参数。F server-editor/server-pricing |
| SD13-server-embedding | 公共向量配置 / 偶发 | 后端/模型 → 维度/批量 → 价格 | 自动维度的禁用状态清楚；价格单位与模型能力对应。F server-editor/server-pricing |
| SD14-admin-trends | 用量趋势诊断 / 按需 | 错误/区间 → 刷新 → 趋势 | 初次失败持续可见，刷新可恢复；7/30 天名称不再截断。F initial-load-error/recovered/seven-day |
| SD15-admin-packages | 功能包管理 / 偶发 | 生命周期 → 个人可见偏好 → 操作反馈 | 409 冲突不再被轮询抹掉；个人隐藏与 Host 启停是原有两类动作，互不回退。F package-states/lifecycle-error |
| SD16-runtime-logs | 定位运行问题 / 按需 | 范围/日期 → 日志 → 级别/路径 → 应用 | 日志等级、原始错误及路径完整；新修读取范围和 UTC 日期截字，独立补图；原过滤与刷新保留 |
| SD17-call-details | 追踪调用与成本 / 按需 | 工作流域 → 失败/待完成 → 金额/模型/时间 | 四个调用域分别展开；失败不伪装成成功，宽表可横向阅读完整金额和时间。F 四域及 cost-table |
| SD18-analytics-90-days | 长时段分析 / 按需 | 区间 → 全时段趋势 → 两端日期 | 不再只截末 30 天；45 日响应在 90 天范围内完整，手机文字保持本征字号。F full-range/chart-last-date |
| SD19-queue-trace | 运行详情排障 / 按需 | 工作流 → 检查点 → 调用 → 失败 | 第 8 检查点、第 10 次调用保留且可达，诊断不额外截断。F actions/trace/diagnostics |
| SD20-queue-resume-cancel | 恢复或停止任务 / 经常 | 所选任务 → 操作 → 活跃与持久状态 | 恢复 501 和取消视频任务分别执行；任务、进度、负载一致反馈。F resumed/menu/cancelled |
| SD21-account-profile | 更新资料 / 偶发 | 当前身份 → 邮箱/头像 → 保存 | 标签与保存归属清楚；原原生类型校验和登录修复说明保留。F expanded/draft/session-repair |
| SD22-account-password | 修改密码 / 偶发 | 原密码 → 新密码 → 结果/重试 | 503 就近可见，两个输入保留；键盘 Enter 与 autocomplete 正确。F filled/failure-retains-fields |
| SD23-account-logout | 退出及草稿保护 / 偶发 | 未保存影响 → 放弃或取消 | 取消保留资料草稿，确认前不撤销会话。F logout-draft-confirmation |
| SD24-account-storage | 查看路径/清理 / 偶发 | 应用信息 → 16 项路径 → 扫描 → 影响确认 | 路径自然换行，数量/大小与清理结果清楚；先取消不发送请求，后确认使用隔离模拟目录。F 五状态三宽独立复核 |
| SD25-auth-login-register | 登录/注册 / 偶发 | 必填输入 → 提交 → 错误/重试 | 两个表单分别真实提交；401/400 原因可见且输入保留，没有伪造成功凭证。F login/registration/failure |
| SD26-auth-oauth | 第三方登录 / 偶发 | 服务选择 → 等待 → 失败/返回 | 选择、等待、服务错误和无效回调各三宽独立复核；未跳转真实第三方。F 四状态 |
| SD27-account-role-user | 创作者账户 / 偶发 | 身份 → 可用偏好 → 管理边界 | 账户与页面 CLI 正常；管理及队列拒绝清楚，无不允许的读取调用。F 三状态 |

## 本次补齐与验证边界

- S01-discovery / S02-discovery / S03-discovery：任务为选择模型，频率为配置时按需；注意力顺序为当前模型 → 检测反馈 → 候选/检索 → 确认。新实屏发现长候选名称被截断，复用现有 Combobox 改自然换行。`final-dialogs.log` 9/9，通过 1440/900/390 的 loading/catalog/failure/manual-entry；检测及 503 均不覆盖已选值，键盘筛选/Enter 选择与失败后手动输入保留。
- SD11：校正样本后的三宽证据在 `final-extras-2026-09-24T23-50-14-728Z`，不把旧样本错误计为产品缺陷。
- 真实浏览器采用生产 React 页面、隔离账号和显式接口响应，未访问真实密钥、真实账号权限或数据库。模型调用、第三方授权、备份 ZIP 传输和原生宿主能力不由这些截图冒充端到端成功。
- 页面仍保留当前暖书房配色、iOS 控件与现有依赖。样式所有权和预算统一由 `frontend/DESIGN.md` 与 `check-design-budget.mjs` 维护。
