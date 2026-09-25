# 前端视觉与交互优化记录

本文件保留第一轮公共层及典型页面的结果，不代表全站逐页审查完成。用户指出的覆盖不足由[逐页审查台账](UI_PAGE_AUDIT.md)继续处理，新增页面结论以该台账为准。

本轮直接修改 `debug`，保留会话开始时全部业务 WIP。审查基线为 2026-09-25 会话开始的实际文件，不是 HEAD。最新确认的方向：**保留原有暖书房配色与 iOS 风格**。

## 设计约束

- 保留亮暗主题原有颜色、透明度、主题自定义和粒子 Logo；统一圆润控件、分段导航、卡片层级与系统字级。
- 保留所有路由、业务调用、权限、草稿保护、快捷键及菜单/弹窗焦点行为。
- 界面图标使用现有 Lucide；删除静态界面的 emoji 和字符图标，不修改用户作品文本。
- 不新增外部依赖；样式源码以全部 `frontend/src/**/*.css` 统计，基线 **4,698 行 / 137,890 字节**，增长上限 20%。同时检查打包 CSS，避免将样式挪进工具类绕过限制。

## 改动台账

| 范围与角色 | 发现的根因 | 已实施的优化 | 验证 |
| --- | --- | --- | --- |
| 高频公共控件 | 按钮尺寸/圆角/动效在 TSX 与 CSS 重复；输入框有两套定义 | 控件状态集中到 `controls.css`；保留原生按钮、禁用及无障碍属性 | 17 项公共控件浏览器回归通过 |
| 主题与应用基础 | token 文件混入应用布局；圆角有两份独立数值 | token、浏览器基础、页面布局分开；Tailwind 圆角引用已有主题 token | 原配色逐 token 比较、构建 |
| 首页定位与主操作 | 标题只对读屏可见；侧栏宽、创建入口低 | 显示 iOS 大标题；缩窄侧栏并提前创建/导入；保留粒子、引文及全部项目筛选 | 首页宽窄截图、菜单/只读回归 |
| 项目列表 | 父子项目层级与卡片层级混在一起 | 保留圆润卡片及类型配色，用一致间距呈现项目族；保留更多菜单、右键和独立入口 | 键盘、禁用操作、父子层级 |
| 写作台主编辑区 | 正文行过长、字号偏小、灰底容易像禁用面板 | 纸面编辑区、18px 正文及最大文字行宽；编辑面和滚动高度保持完整 | 章节切换、未保存草稿、恢复、窄屏 |
| 章节导航 | 小图标、搜索清除使用字符、选中效果重复 | Lucide 图标、搜索名称、清晰的单一选中底色和统一列表节奏 | 章节导航与键盘回归 |
| 设置 | 导航和配置标题层级相近；英文窄屏标题与按钮相撞 | 显示设置身份；统一配置标题和工具栏，小屏按标题/操作分行；保留全部深链和草稿拦截 | 模型检测、IME、草稿回归及文字矩形碰撞检查通过 |
| Agent 输入区 | 运行时发送按钮被停止按钮替换；手机固定会话高度把发送栏挤出首屏 | 同时呈现发送与取消；按剩余高度分配会话，保留较矮视口至少 180px 的历史区域 | 运行中追加、线程切换、失败草稿、历史高度与发送栏可见性通过 |
| 灵感失败与提示 | 错误内容被拼入 emoji，展示层缺少独立状态语义 | 原样保留错误文本，由 Lucide 和 alert 容器表达失败；替换其余静态字符图标 | 消息一致性测试、静态扫描 |
| 样式目录 | 响应式文件混入数百行页面常态样式；多个文件共同拥有同一组件 | 页面规则回归所属文件，共享层只保留公共规则 | 构建、导入检查、体积检查 |
| 加载失败 | 同一错误同时出现在 toast 和页面，缺少直接恢复入口 | 一处持续的错误提示，保留服务错误原文；重试复用现有 refresh 和筛选状态 | 错误显示、重试、筛选保留回归通过 |
| 细节与动效 | 粒子在卸载边界仍可能绘制；toast 位移动画短暂压到导航；部分图标过小 | 同步清理 canvas；toast 使用淡入；统一高频图标尺度，必填标记不冒充错误 | 粒子停止/卸载、通知遮挡、减少动态效果和暗色对比度复核通过 |

## 样式所有权

```text
frontend/src/
  components/ui/       复用交互控件
  components/shell/    页面框架
  pages/<feature>/     领域页面、控制器与局部组件
  styles/
    theme.css          原配色、主题参数、Tailwind token 映射
    base.css           应用画布、浏览器与基础无障碍
    controls.css       按钮、输入、选择器、Tab、分段控件
    surfaces.css       面板、浮层、列表、空状态
    page-layout.css    页头、内容留白、排版层级
    motion.css         共享动效
    scrolling.css      滚动、阅读宽度与范围控件
    responsive.css     跨组件窄屏、触控与减少动态效果
    home.css / auth.css / writing-desk.css
    agent-*.css / studio-flow.css / relationship-flow.css
```

保留现有 `pyqt-*` / `book-*` 类名与主题合同，避免为重命名扩大业务 diff。页面数据逻辑仍由原来的控制器和 API wrapper 负责。

## 验证结果

- 前端生产构建通过；Vitest **101 文件 / 385 测试通过**。
- 浏览器 **62 项通过**：22 项工作流程、17 项公共控件、23 项视觉与减少动态效果检查。
- **22 个场景 × 1440 / 1024 / 390 px × 亮 / 暗 = 132 张截图**；逐张校验实际 viewport 与 PNG 尺寸，横向溢出、控件越界和设置标题碰撞均为零。
- 7 类重点窄屏场景另有 14 张标注图；检查到的文字对比度、触控尺寸与横向溢出均通过。整卡点击层、浮层与其下方内容的有意覆盖由独立审查者核对，未将其误判为文字碰撞。
- Chrome 原生减少动态效果仿真通过，CSS 动效归零，粒子仍显示但不持续绘制。
- 独立视觉/UX 复审：**本次覆盖范围内无 major / blocker**。审查代理沿用本会话模型，工具未提供费用数据。
- 默认亮暗配色逐 token 比对通过；UI 源码 emoji 扫描为零；没有改动依赖清单或锁文件。
- `git diff --check` 通过。全库 `cargo fmt --all -- --check` 未通过：Control 日志、Runtime 设置导出、Tauri 本机状态及 Prompt 模块存在其他会话的格式 WIP；本轮未修改这些 Rust 文件。

| 样式度量 | 原始基线 | 当前 | 变化 |
| --- | ---: | ---: | ---: |
| CSS 源码行数 | 4,698 | 4,565 | −2.83% |
| CSS 源码字节 | 137,890 | 130,124 | −5.63% |
| 打包 CSS 字节 | 约 257,470 | 249,593 | 约 −3.06% |

打包基线取首次 Vite 日志的精度；源码基线是原始文件精确计数。两种口径均满足不超过 20% 的约束，未通过迁移到工具类规避体积限制。

可重复执行：

```bash
cd /home/fufu/Code/A4N/work/debug/frontend
mkdir -p ../.dev-runtime/ui25
npm run build
node e2e/ui-presentation/check-design-budget.mjs
TMPDIR=/home/fufu/Code/A4N/work/debug/.dev-runtime/ui25 npm run test:run
TMPDIR=/home/fufu/Code/A4N/work/debug/.dev-runtime/ui25 npm run test:browser-ui-presentation
```

本轮使用真实应用页面与隔离的接口 fixture，覆盖首页、写作、设置、主题、提示词、Agent、灵感、项目详情、漫画、视频、GALGame、画布、登录以及空/错/加载状态、菜单、弹窗、英文和长文本。媒体页面覆盖默认/空状态外壳；未执行真实模型生成或原生 Tauri/WebKit 实机回归。原有业务数据和其他会话 WIP 均保留，本轮未统一提交共享工作树。

## 截图与证据

- [首页：桌面亮色](../.dev-runtime/ui-design-20260925/visual/ios-20260925-home-1440-light.png)、[手机暗色](../.dev-runtime/ui-design-20260925/visual/ios-20260925-home-390-dark.png)
- [写作台：桌面](../.dev-runtime/ui-design-20260925/visual/ios-20260925-writing-1440-light.png)、[手机长文](../.dev-runtime/ui-design-20260925/visual/ios-20260925-writing-390-light.png)
- [英文设置手机页](../.dev-runtime/ui-design-20260925/visual/ios-20260925-settings-en-390-light.png)、[配置表单](../.dev-runtime/ui-design-20260925/visual/ios-20260925-settings-editor-390-dark.png)
- [Agent 手机页](../.dev-runtime/ui-design-20260925/visual/ios-20260925-agents-390-dark.png)、[创建项目弹窗](../.dev-runtime/ui-design-20260925/visual/ios-20260925-project-dialog-390-light.png)
- [视频画布](../.dev-runtime/ui-design-20260925/visual/ios-20260925-canvas-1440-light.png)、[登录页](../.dev-runtime/ui-design-20260925/visual/ios-20260925-auth-1440-light.png)
- [全部截图测量](../.dev-runtime/ui-design-20260925/visual/report.json)、[浏览器日志](../.dev-runtime/ui-design-20260925/browser-complete.log)、[样式预算](../.dev-runtime/ui-design-20260925/design-budget.json)

原始源码快照与首次测试日志位于 `.dev-runtime/ui-design-20260925/baseline/`。首次失败、临时目录配额/路径问题、旧截图等待逻辑导致的失败批次均保留在日志中，未计为通过。后续优化先查本台账与原配色基线，避免重复修改或遗漏已验证的交互。
