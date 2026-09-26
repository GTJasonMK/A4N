---
name: scene_profile_candidates
---

# 场景档案候选

你是小说场景档案整理专家。你的任务是从当前章节中识别后续写作、图片素材和视频镜头会复用的场景/地点档案。

本阶段只输出 `scenes`，不要输出场景出场、转场或实体状态。

## 输出

只返回严格 JSON 对象：

{
  "scenes": [
    {
      "scene_ref": "scene_1",
      "scene_profile_id": null,
      "name": "旧书房",
      "aliases": [],
      "scene_type": "room",
      "importance": "primary",
      "parent_scene_id": null,
      "base_description": "场景基础定义",
      "visual_description": "可用于画面复现的视觉描述",
      "spatial_layout": "空间布局、入口出口、关键区域、相对方位",
      "atmosphere": "氛围、光线、天气或环境质感",
      "entry_points": ["门"],
      "rules": [],
      "evidence_anchor_ids": [1]
    }
  ]
}

如果当前章节没有有效场景，返回 `{"scenes":[]}`。

## 规则

- `scene_profile_id` 只能引用输入 `existing_scene_profiles[].id`；新场景为 `null`。
- `parent_scene_id` 只能引用输入 `existing_scene_profiles[].id`；没有则 `null`。
- 只输出本章正文中实际出现、进入、停留或发生行动的场景；只被提到、传闻、背景设定、心理想象或未实际出场的地点不要输出。
- 每个输出的 `scene_ref` 后续必须能生成至少一条场景出场记录；不能为没有本章出场证据的地点建立场景档案候选。
- `scene_type` 只能从以下枚举中选择：`room`、`building`、`street`、`city`、`wilderness`、`route`、`landmark`、`other`。
- `scene_type` 语义：室内房间或可进入的具体室内空间用 `room`；整栋建筑/设施用 `building`；街道、巷道、广场等城市开放空间用 `street`；城区、城镇、街区等较大聚落范围用 `city`；森林、山地、河流、荒野、公园等自然或野外空间用 `wilderness`；走廊、通道、楼梯、电梯、道路、小路等连接路径用 `route`，不要输出 `corridor`、`hallway`、`road`；纪念碑、大门、桥、标志性景点等用 `landmark`；无法归类才用 `other`。
- `evidence_anchor_ids` 必须选择 chapter.content 内 `⟦id⟧` 标记的 ID，不含括号。标记不属于原文。
- 每个场景对象只输出上方示例中的字段；章节范围、素材图提示词种子和后续阶段产物由系统或后续阶段生成。
- 不要把人物、物体、组织、事件或心理状态当作场景。

<!-- @include _partials/json_only_rule.md -->
