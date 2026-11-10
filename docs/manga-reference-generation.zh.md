# 为漫画发现需求生成参考图

在漫画的“分镜与参考图”页，每条系统发现需求提供“生成参考图”。选择当前图片模型支持的尺寸后，生成一张图片并自动绑定到该需求。角色、场景、物品和风格需求使用相同入口；仅有已有素材匹配、没有实际需求的条目不提供生成按钮。

该操作复用统一图片任务，不额外调用文本模型。提示词由后端读取已保存需求的名称、视觉描述和连续性用途生成；角色需求还会使用已绑定的该角色参考图。图片保存在当前漫画项目的素材库，类型为 `chapter_reference`，不会替换漫画整页成图或角色主立绘。费用、进度、取消和调用记录沿用图片生成系统。

CLI / Host API 使用已有 `start_image_generation`，新增 `manga_reference` 操作：

```json
{
  "input": {
    "meta": {"start_request_id": "本次操作的唯一 UUID"},
    "project_id": 19,
    "operation": {
      "kind": "manga_reference",
      "options": {
        "prompt_id": 43,
        "target_kind": "object",
        "target_name": "断裂树根",
        "expected_updated_at": "从最新页面记录原样读取 updated_at",
        "width": 1024,
        "height": 1024
      }
    }
  }
}
```

参数以当前 `a4n capabilities describe start_image_generation` 为准。`target_kind` 为 `character`、`scene`、`object` 或 `style`；名称须对应已保存的发现需求，不能传入任意目标或另造描述。返回任务句柄，可用 `a4n task watch TASK_ID --attempt ATTEMPT` 等待终态。

生成前检查编辑权限、项目/页面归属、原文、上游产物、页面版本和绑定容量，每个需求最多两张、每页最多十二张。成功后在同一事务中更新参考发现产物和页面投影，并将依赖它的绘图提示词、成图等下游产物标为过期；后续生成读取更新后的绑定。

Provider 失败不会产生图片或绑定。生成期间若页面变化，自动绑定通过版本校验拒绝覆盖；已经生成的图片保留在素材库，任务明确报告绑定失败，可刷新后手动选择。取消及其他错误也不会返回假成功。没有自动批量生成，点击一次只请求一张图片。

回归入口：`run_media_authoring_cli_e2e.py` 验证请求、持久化、版本冲突与容量；`frontend/e2e/media-authoring/reference-generation.e2e.mjs` 验证按钮、进度、失败重试、绑定持久性、键盘操作和宽窄屏布局。
